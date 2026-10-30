//! fatfsd (redox-fatfs2): serves a FAT partition as a Redox scheme. It
//! replaces the unmaintained redox-fatfs.
//!
//!     fatfsd <disk> <scheme>
//!     fatfsd /scheme/disk.mmc/0p0 boot     # the Pi boot partition as /scheme/boot
//!
//! The FAT code is the `fatfs` crate; this file only binds it to the scheme
//! interface of `redox-scheme`, the way redoxfs does. Handles keep a path;
//! a file handle also keeps the crate's file open with its position, so
//! sequential reads and writes do not walk the cluster chain again. fatfsd
//! stays in the foreground, start it with `&`.
//!
//! SIGTERM unmounts: open files are written, the FAT32 free-space info and
//! the volume's dirty flag are updated, and the scheme goes away. A disk
//! that goes away (ENODEV, ENXIO) ends fatfsd too. Either way it removes its
//! scheme name, so a stick plugged in again can use it.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::os::unix::fs::MetadataExt;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use fatfs::{Dir, FileSystem, FsOptions};
use redox_scheme::scheme::{register_sync_scheme, SchemeState, SchemeSync};
use redox_scheme::{CallerCtx, OpenResult, RequestKind, Response, SignalBehavior, Socket};
use syscall::dirent::{DirEntry, DirentBuf, DirentKind};
use syscall::error::{
    Error, Result, EBADF, EBUSY, EEXIST, EINTR, EINVAL, EIO, EISDIR, ENODEV, ENOENT, ENOTDIR,
    ENOTEMPTY, ENXIO, EXDEV,
};
use syscall::flag::{
    AT_REMOVEDIR, MODE_DIR, MODE_FILE, O_ACCMODE, O_CREAT, O_DIRECTORY, O_EXCL, O_RDONLY, O_STAT,
    O_TRUNC,
};
use syscall::schemev2::NewFdFlags;
use syscall::data::Stat;

/// The disk device with byte access on top of whole-block reads and writes.
/// The block size is the device's `st_blksize` (512 on SD cards and most USB
/// sticks, 4096 on some).
struct Disk {
    file: File,
    pos: u64,
    block: Vec<u8>,
    /// The block number `block` holds. fatfsd is the only writer, so it
    /// stays valid; the FAT is read entry by entry.
    cached: Option<u64>,
}

/// The scheme name, to remove it from the namespace at exit.
static SCHEME_NAME: OnceLock<String> = OnceLock::new();

/// Set by SIGTERM.
static UNMOUNT: AtomicBool = AtomicBool::new(false);

extern "C" fn on_sigterm(_: libc::c_int) {
    UNMOUNT.store(true, Ordering::SeqCst);
}

/// Removes the scheme name from the namespace and exits. A scheme's name
/// stays there after its socket is closed until it is unlinked (unlinking
/// the path would ask the scheme itself).
fn leave(code: i32) -> ! {
    if let Some(name) = SCHEME_NAME.get() {
        let unlinked = libredox::call::getns()
            .map_err(|e| e.to_string())
            .and_then(|ns| syscall::unlinkat(ns, format!("/scheme/{name}"), 0).map_err(|e| e.to_string()));
        if let Err(err) = unlinked {
            eprintln!("fatfsd: /scheme/{name} stays: {err}");
        }
    }
    process::exit(code);
}

/// errno of a disk that went away, e.g. an unplugged USB stick.
fn is_gone(err: &io::Error) -> bool {
    matches!(err.raw_os_error(), Some(ENODEV) | Some(ENXIO))
}

impl Disk {
    fn open(path: &str) -> io::Result<Disk> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        let size = match file.metadata()?.blksize() {
            n @ 512..=65536 if n.is_power_of_two() => n as usize,
            _ => 512,
        };
        Ok(Disk { file, pos: 0, block: vec![0; size], cached: None })
    }

    fn read_block(&mut self, n: u64) -> io::Result<()> {
        if self.cached == Some(n) {
            return Ok(());
        }
        self.cached = None;
        let size = self.block.len() as u64;
        self.file.seek(SeekFrom::Start(n * size))?;
        self.file.read_exact(&mut self.block)?;
        self.cached = Some(n);
        Ok(())
    }

    fn check(res: io::Result<()>) -> io::Result<()> {
        if let Err(err) = &res {
            if is_gone(err) {
                eprintln!("fatfsd: the disk is gone ({err}), exiting");
                leave(0);
            }
        }
        res
    }
}

impl Read for Disk {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let size = self.block.len();
        let mut done = 0;
        while done < buf.len() {
            let pos = self.pos + done as u64;
            let skip = (pos % size as u64) as usize;
            let n = (size - skip).min(buf.len() - done);
            Disk::check(self.read_block(pos / size as u64))?;
            buf[done..done + n].copy_from_slice(&self.block[skip..skip + n]);
            done += n;
        }
        self.pos += done as u64;
        Ok(done)
    }
}

impl Write for Disk {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let size = self.block.len();
        let mut done = 0;
        while done < buf.len() {
            let pos = self.pos + done as u64;
            let skip = (pos % size as u64) as usize;
            let n = (size - skip).min(buf.len() - done);
            if n < size {
                Disk::check(self.read_block(pos / size as u64))?;
            }
            self.block[skip..skip + n].copy_from_slice(&buf[done..done + n]);
            let start = pos - skip as u64;
            self.cached = None;
            let res = self.file.seek(SeekFrom::Start(start)).and_then(|_| self.file.write_all(&self.block));
            Disk::check(res)?;
            self.cached = Some(start / size as u64);
            done += n;
        }
        self.pos += done as u64;
        Ok(done)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

impl Seek for Disk {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        self.pos = match from {
            SeekFrom::Start(n) => n,
            SeekFrom::Current(n) => self.pos.checked_add_signed(n).ok_or(io::ErrorKind::InvalidInput)?,
            SeekFrom::End(n) => {
                let end = self.file.seek(SeekFrom::End(0))?;
                end.checked_add_signed(n).ok_or(io::ErrorKind::InvalidInput)?
            }
        };
        Ok(self.pos)
    }
}

type FatFile = fatfs::File<'static, Disk>;

enum Handle {
    Root,
    Dir(String),
    /// `open`: the crate's file and its position, opened at first use.
    File { path: String, writable: bool, open: Option<(FatFile, u64)> },
}

struct FatScheme {
    /// Leaked, so open files (which borrow it) can live in the handles.
    fs: &'static FileSystem<Disk>,
    name: String,
    handles: BTreeMap<usize, Handle>,
    next_id: usize,
}

fn io_err(err: io::Error) -> Error {
    Error::new(match err.kind() {
        io::ErrorKind::NotFound => ENOENT,
        io::ErrorKind::AlreadyExists => EEXIST,
        io::ErrorKind::InvalidInput => EINVAL,
        _ => match err.raw_os_error() {
            Some(errno) => errno,
            // fatfs reports "Directory not empty" and similar as Other.
            None if err.to_string().contains("not empty") => ENOTEMPTY,
            None => EIO,
        },
    })
}

/// Joins `path` to the directory `base` and removes ".", ".." and empty parts.
fn join(base: &str, path: &str) -> String {
    let mut parts: Vec<&str> = if path.starts_with('/') {
        Vec::new()
    } else {
        base.split('/').filter(|p| !p.is_empty()).collect()
    };
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    parts.join("/")
}

impl FatScheme {
    fn dir(&self, path: &str) -> Result<Dir<'_, Disk>> {
        let root = self.fs.root_dir();
        if path.is_empty() {
            Ok(root)
        } else {
            root.open_dir(path).map_err(io_err)
        }
    }

    /// (is a directory, size) of `path`, or ENOENT.
    fn lookup(&self, path: &str) -> Result<(bool, u64)> {
        if path.is_empty() {
            return Ok((true, 0));
        }
        let (parent, name) = path.rsplit_once('/').unwrap_or(("", path));
        for entry in self.dir(parent)?.iter() {
            let entry = entry.map_err(io_err)?;
            if entry.file_name().eq_ignore_ascii_case(name) {
                return Ok((entry.is_dir(), entry.len()));
            }
        }
        Err(Error::new(ENOENT))
    }

    fn path(&self, id: usize) -> Result<&str> {
        match self.handles.get(&id).ok_or(Error::new(EBADF))? {
            Handle::Root => Ok(""),
            Handle::Dir(path) | Handle::File { path, .. } => Ok(path),
        }
    }

    fn file_path(&self, id: usize) -> Result<(&str, bool)> {
        match self.handles.get(&id).ok_or(Error::new(EBADF))? {
            Handle::File { path, writable, .. } => Ok((path, *writable)),
            _ => Err(Error::new(EISDIR)),
        }
    }

    /// The handle's open file, seeked to `offset`, and the position it got:
    /// fatfs stops at the end of the file.
    fn file_at(&mut self, id: usize, offset: u64) -> Result<(&mut FatFile, u64)> {
        let fs = self.fs;
        let Some(Handle::File { path, open, .. }) = self.handles.get_mut(&id) else {
            return Err(Error::new(EISDIR));
        };
        if open.is_none() {
            *open = Some((fs.root_dir().open_file(path).map_err(io_err)?, 0));
        }
        let (file, pos) = open.as_mut().unwrap();
        if *pos != offset {
            *pos = file.seek(SeekFrom::Start(offset)).map_err(io_err)?;
        }
        Ok((file, *pos))
    }

    fn advance(&mut self, id: usize, n: u64) {
        if let Some(Handle::File { open: Some((_, pos)), .. }) = self.handles.get_mut(&id) {
            *pos += n;
        }
    }

    /// Closes the open files of `path` in all handles, before it is truncated
    /// or removed: an open file keeps its first cluster and would write to
    /// freed clusters. They reopen at their next use.
    fn close_files(&mut self, path: &str) {
        for handle in self.handles.values_mut() {
            if let Handle::File { path: p, open, .. } = handle {
                if p.eq_ignore_ascii_case(path) {
                    *open = None;
                }
            }
        }
    }

    /// The path a rename target names: `/scheme/<name>/...` (what relibc
    /// sends), or a path in this filesystem.
    fn target_path(&self, path: &str) -> Result<String> {
        match path.strip_prefix("/scheme/") {
            Some(rest) => {
                let (scheme, rest) = rest.split_once('/').unwrap_or((rest, ""));
                if scheme != self.name {
                    return Err(Error::new(EXDEV));
                }
                Ok(join("", rest))
            }
            None => Ok(join("", path)),
        }
    }

    /// Gives the handles of `from` and everything below it the new path.
    fn rename_handles(&mut self, from: &str, to: &str) {
        for handle in self.handles.values_mut() {
            if let Handle::Dir(path) | Handle::File { path, .. } = handle {
                let lower = path.to_ascii_lowercase();
                let from_lower = from.to_ascii_lowercase();
                if lower == from_lower {
                    *path = to.to_string();
                } else if lower.starts_with(&format!("{from_lower}/")) {
                    *path = format!("{to}{}", &path[from.len()..]);
                }
            }
        }
    }

    fn insert(&mut self, handle: Handle) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.handles.insert(id, handle);
        id
    }
}

impl SchemeSync for FatScheme {
    fn scheme_root(&mut self) -> Result<usize> {
        Ok(self.insert(Handle::Root))
    }

    fn openat(
        &mut self,
        dirfd: usize,
        path: &str,
        flags: usize,
        _fcntl_flags: u32,
        _ctx: &CallerCtx,
    ) -> Result<OpenResult> {
        let path = join(self.path(dirfd)?, path);
        let handle = match self.lookup(&path) {
            Ok(_) if flags & O_CREAT != 0 && flags & O_EXCL != 0 => return Err(Error::new(EEXIST)),
            Ok((true, _)) => Handle::Dir(path),
            Ok((false, _)) if flags & O_DIRECTORY != 0 && flags & O_STAT == 0 => {
                return Err(Error::new(ENOTDIR))
            }
            Ok((false, _)) => {
                let writable = flags & O_ACCMODE != O_RDONLY;
                if flags & O_TRUNC != 0 && writable {
                    self.close_files(&path);
                    let mut file = self.fs.root_dir().open_file(&path).map_err(io_err)?;
                    file.truncate().map_err(io_err)?;
                    file.flush().map_err(io_err)?;
                }
                Handle::File { path, writable, open: None }
            }
            Err(err) if err.errno == ENOENT && flags & O_CREAT != 0 => {
                let root = self.fs.root_dir();
                if flags & O_DIRECTORY != 0 {
                    root.create_dir(&path).map_err(io_err)?;
                    Handle::Dir(path)
                } else {
                    root.create_file(&path).map_err(io_err)?.flush().map_err(io_err)?;
                    Handle::File { path, writable: flags & O_ACCMODE != O_RDONLY, open: None }
                }
            }
            Err(err) => return Err(err),
        };
        Ok(OpenResult::ThisScheme {
            number: self.insert(handle),
            flags: NewFdFlags::POSITIONED,
        })
    }

    fn unlinkat(&mut self, dirfd: usize, path: &str, flags: usize, _ctx: &CallerCtx) -> Result<()> {
        let path = join(self.path(dirfd)?, path);
        let (is_dir, _) = self.lookup(&path)?;
        if is_dir != (flags & AT_REMOVEDIR != 0) {
            return Err(Error::new(if is_dir { EISDIR } else { ENOTDIR }));
        }
        self.close_files(&path);
        self.fs.root_dir().remove(&path).map_err(io_err)
    }

    fn read(
        &mut self,
        id: usize,
        buf: &mut [u8],
        offset: u64,
        _fcntl_flags: u32,
        _ctx: &CallerCtx,
    ) -> Result<usize> {
        let (file, _) = self.file_at(id, offset)?;
        let mut done = 0;
        let mut res = Ok(());
        while done < buf.len() {
            match file.read(&mut buf[done..]) {
                Ok(0) => break,
                Ok(n) => done += n,
                Err(err) => {
                    res = Err(io_err(err));
                    break;
                }
            }
        }
        self.advance(id, done as u64);
        res.map(|_| done)
    }

    fn write(
        &mut self,
        id: usize,
        buf: &[u8],
        offset: u64,
        _fcntl_flags: u32,
        _ctx: &CallerCtx,
    ) -> Result<usize> {
        let (_, writable) = self.file_path(id)?;
        if !writable {
            return Err(Error::new(EBADF));
        }
        let (file, pos) = self.file_at(id, offset)?;
        // Writing past the end: fill the gap.
        let res = (|| {
            if pos < offset {
                file.write_all(&vec![0; (offset - pos) as usize])?;
            }
            file.write_all(buf)?;
            file.flush()
        })();
        if res.is_err() {
            // Position unknown after a failed write: reopen next time.
            let path = self.path(id)?.to_string();
            self.close_files(&path);
        }
        res.map_err(io_err)?;
        self.advance(id, offset.max(pos) - pos + buf.len() as u64);
        Ok(buf.len())
    }

    fn fsize(&mut self, id: usize, _ctx: &CallerCtx) -> Result<u64> {
        let path = self.path(id)?.to_string();
        Ok(self.lookup(&path)?.1)
    }

    fn ftruncate(&mut self, id: usize, len: u64, _ctx: &CallerCtx) -> Result<()> {
        let (path, writable) = self.file_path(id)?;
        if !writable {
            return Err(Error::new(EBADF));
        }
        let path = path.to_string();
        self.close_files(&path);
        let mut file = self.fs.root_dir().open_file(&path).map_err(io_err)?;
        let old = file.seek(SeekFrom::End(0)).map_err(io_err)?;
        if len < old {
            file.seek(SeekFrom::Start(len)).map_err(io_err)?;
            file.truncate().map_err(io_err)?;
        } else if len > old {
            file.write_all(&vec![0; (len - old) as usize]).map_err(io_err)?;
        }
        file.flush().map_err(io_err)
    }

    /// Renames the handle's file or directory. Like POSIX, an existing
    /// target is replaced if it is a file, or an empty directory for a
    /// directory.
    fn frename(&mut self, id: usize, path: &str, _ctx: &CallerCtx) -> Result<usize> {
        let from = self.path(id)?.to_string();
        if from.is_empty() {
            return Err(Error::new(EBUSY));
        }
        let to = self.target_path(path)?;
        if to.is_empty() || to.to_ascii_lowercase().starts_with(&format!("{}/", from.to_ascii_lowercase())) {
            return Err(Error::new(EINVAL));
        }
        if to == from {
            return Ok(0);
        }
        let (from_dir, _) = self.lookup(&from)?;
        // A different name for the same entry (only the case changes) is a
        // rename, not a replace.
        if !to.eq_ignore_ascii_case(&from) {
            match self.lookup(&to) {
                Ok((to_dir, _)) => {
                    if to_dir != from_dir {
                        return Err(Error::new(if to_dir { EISDIR } else { ENOTDIR }));
                    }
                    self.close_files(&to);
                    self.fs.root_dir().remove(&to).map_err(io_err)?;
                }
                Err(err) if err.errno == ENOENT => {}
                Err(err) => return Err(err),
            }
        }
        self.close_files(&from);
        let root = self.fs.root_dir();
        root.rename(&from, &root, &to).map_err(io_err)?;
        self.rename_handles(&from, &to);
        Ok(0)
    }

    fn fstat(&mut self, id: usize, stat: &mut Stat, _ctx: &CallerCtx) -> Result<()> {
        let path = self.path(id)?.to_string();
        let (is_dir, size) = self.lookup(&path)?;
        *stat = Stat {
            st_mode: if is_dir { MODE_DIR | 0o755 } else { MODE_FILE | 0o644 },
            st_nlink: 1,
            st_size: size,
            st_blksize: 512,
            st_blocks: size.div_ceil(512),
            ..Default::default()
        };
        Ok(())
    }

    fn fpath(&mut self, id: usize, buf: &mut [u8], _ctx: &CallerCtx) -> Result<usize> {
        let full = format!("/scheme/{}/{}", self.name, self.path(id)?);
        let n = full.len().min(buf.len());
        buf[..n].copy_from_slice(&full.as_bytes()[..n]);
        Ok(n)
    }

    fn fsync(&mut self, id: usize, _ctx: &CallerCtx) -> Result<()> {
        self.path(id)?;
        Ok(())
    }

    fn getdents<'buf>(
        &mut self,
        id: usize,
        mut buf: DirentBuf<&'buf mut [u8]>,
        opaque_offset: u64,
    ) -> Result<DirentBuf<&'buf mut [u8]>> {
        let path = match self.handles.get(&id).ok_or(Error::new(EBADF))? {
            Handle::Root => String::new(),
            Handle::Dir(path) => path.clone(),
            Handle::File { .. } => return Err(Error::new(ENOTDIR)),
        };
        let entries = self
            .dir(&path)?
            .iter()
            .filter_map(|e| e.ok())
            .map(|e| (e.file_name(), e.is_dir()))
            .filter(|(name, _)| name != "." && name != "..");
        for (idx, (name, is_dir)) in entries.enumerate().skip(opaque_offset as usize) {
            let result = buf.entry(DirEntry {
                inode: idx as u64 + 1,
                next_opaque_id: idx as u64 + 1,
                name: &name,
                kind: if is_dir { DirentKind::Directory } else { DirentKind::Regular },
            });
            if let Err(err) = result {
                // A full buffer ends the call; the caller asks again.
                if err.errno == EINVAL && idx > opaque_offset as usize {
                    break;
                }
                return Err(err);
            }
        }
        Ok(buf)
    }

    fn on_close(&mut self, id: usize) {
        self.handles.remove(&id);
    }
}

fn run(disk: &str, name: &str) -> io::Result<()> {
    let disk_file = Disk::open(disk)?;
    let block = disk_file.block.len();
    let fs: &'static FileSystem<Disk> = Box::leak(Box::new(FileSystem::new(disk_file, FsOptions::new())?));
    eprintln!("fatfsd: {} {:?}, label {:?}, {} byte blocks", disk, fs.fat_type(), fs.volume_label(), block);

    let socket = Socket::create()?;
    let mut scheme = FatScheme { fs, name: name.to_string(), handles: BTreeMap::new(), next_id: 1 };
    let mut state = SchemeState::new();
    register_sync_scheme(&socket, name, &mut scheme).map_err(io::Error::from)?;
    let _ = SCHEME_NAME.set(name.to_string());
    eprintln!("fatfsd: serving /scheme/{name}");

    // Without SA_RESTART, so that waiting for a request returns EINTR.
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = on_sigterm as usize;
        libc::sigaction(libc::SIGTERM, &action, std::ptr::null_mut());
    }

    loop {
        if UNMOUNT.load(Ordering::SeqCst) {
            return unmount(scheme);
        }
        let next = match socket.next_request(SignalBehavior::Interrupt) {
            Err(err) if err.errno == EINTR => continue,
            next => next?,
        };
        let req = match next {
            None => return Ok(()),
            Some(req) => match req.kind() {
                RequestKind::Call(req) => req,
                RequestKind::OnClose { id } => {
                    scheme.on_close(id);
                    state.on_close(id);
                    continue;
                }
                _ => continue,
            },
        };
        let response: Response = req.handle_sync(&mut scheme, &mut state);
        if !socket.write_response(response, SignalBehavior::Restart)? {
            return Ok(());
        }
    }
}

/// Writes the open files, then the FAT32 free-space info and the clean
/// flag (fatfs' unmount).
fn unmount(mut scheme: FatScheme) -> io::Result<()> {
    eprintln!("fatfsd: unmounting /scheme/{}", scheme.name);
    // The open files borrow the filesystem; dropping them writes them.
    scheme.handles.clear();
    let fs = scheme.fs as *const FileSystem<Disk> as *mut FileSystem<Disk>;
    drop(scheme);
    // SAFETY: `fs` came from Box::leak in run(), and nothing borrows it any
    // more: the handles are gone and the scheme is dropped.
    let fs = unsafe { Box::from_raw(fs) };
    fs.unmount()?;
    eprintln!("fatfsd: unmounted, the disk can be removed");
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: fatfsd <disk> <scheme>   e.g. fatfsd /scheme/disk.mmc/0p0 boot");
        process::exit(1);
    }
    if let Err(err) = run(&args[1], &args[2]) {
        eprintln!("fatfsd: {}: {err}", args[1]);
        leave(1);
    }
    leave(0);
}
