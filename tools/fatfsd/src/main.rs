//! fatfsd: serves a FAT partition as a Redox scheme.
//!
//!     fatfsd <disk> <scheme>
//!     fatfsd /scheme/disk.mmc/0p0 boot     # the Pi boot partition as /scheme/boot
//!
//! The FAT code is the `fatfs` crate; this file only binds it to the scheme
//! interface of `redox-scheme`, the way redoxfs does. Handles keep a path and
//! reopen the file in the crate for each call, so no crate object has to
//! outlive a request. fatfsd stays in the foreground, start it with `&`.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::process;

use fatfs::{Dir, FileSystem, FsOptions};
use redox_scheme::scheme::{register_sync_scheme, SchemeState, SchemeSync};
use redox_scheme::{CallerCtx, OpenResult, RequestKind, Response, SignalBehavior, Socket};
use syscall::dirent::{DirEntry, DirentBuf, DirentKind};
use syscall::error::{
    Error, Result, EBADF, EEXIST, EINVAL, EIO, EISDIR, ENOENT, ENOTDIR, ENOTEMPTY,
};
use syscall::flag::{
    AT_REMOVEDIR, MODE_DIR, MODE_FILE, O_ACCMODE, O_CREAT, O_DIRECTORY, O_EXCL, O_RDONLY, O_STAT,
    O_TRUNC,
};
use syscall::schemev2::NewFdFlags;
use syscall::data::Stat;

const SECTOR: u64 = 512;

/// The disk device with byte access on top of whole-sector reads and writes.
struct Disk {
    file: File,
    pos: u64,
}

impl Disk {
    fn sector(&mut self, n: u64, buf: &mut [u8; SECTOR as usize]) -> io::Result<()> {
        self.file.seek(SeekFrom::Start(n * SECTOR))?;
        self.file.read_exact(buf)
    }
}

impl Read for Disk {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut done = 0;
        let mut sector = [0; SECTOR as usize];
        while done < buf.len() {
            let pos = self.pos + done as u64;
            let skip = (pos % SECTOR) as usize;
            let n = (SECTOR as usize - skip).min(buf.len() - done);
            self.sector(pos / SECTOR, &mut sector)?;
            buf[done..done + n].copy_from_slice(&sector[skip..skip + n]);
            done += n;
        }
        self.pos += done as u64;
        Ok(done)
    }
}

impl Write for Disk {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut done = 0;
        let mut sector = [0; SECTOR as usize];
        while done < buf.len() {
            let pos = self.pos + done as u64;
            let skip = (pos % SECTOR) as usize;
            let n = (SECTOR as usize - skip).min(buf.len() - done);
            if n < SECTOR as usize {
                self.sector(pos / SECTOR, &mut sector)?;
            }
            sector[skip..skip + n].copy_from_slice(&buf[done..done + n]);
            self.file.seek(SeekFrom::Start(pos / SECTOR * SECTOR))?;
            self.file.write_all(&sector)?;
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

enum Handle {
    Root,
    Dir(String),
    File { path: String, writable: bool },
}

struct FatScheme {
    fs: FileSystem<Disk>,
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
            Handle::File { path, writable } => Ok((path, *writable)),
            _ => Err(Error::new(EISDIR)),
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
                    let mut file = self.fs.root_dir().open_file(&path).map_err(io_err)?;
                    file.truncate().map_err(io_err)?;
                    file.flush().map_err(io_err)?;
                }
                Handle::File { path, writable }
            }
            Err(err) if err.errno == ENOENT && flags & O_CREAT != 0 => {
                let root = self.fs.root_dir();
                if flags & O_DIRECTORY != 0 {
                    root.create_dir(&path).map_err(io_err)?;
                    Handle::Dir(path)
                } else {
                    root.create_file(&path).map_err(io_err)?.flush().map_err(io_err)?;
                    Handle::File { path, writable: flags & O_ACCMODE != O_RDONLY }
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
        let (path, _) = self.file_path(id)?;
        let mut file = self.fs.root_dir().open_file(path).map_err(io_err)?;
        file.seek(SeekFrom::Start(offset)).map_err(io_err)?;
        let mut done = 0;
        while done < buf.len() {
            match file.read(&mut buf[done..]).map_err(io_err)? {
                0 => break,
                n => done += n,
            }
        }
        Ok(done)
    }

    fn write(
        &mut self,
        id: usize,
        buf: &[u8],
        offset: u64,
        _fcntl_flags: u32,
        _ctx: &CallerCtx,
    ) -> Result<usize> {
        let (path, writable) = self.file_path(id)?;
        if !writable {
            return Err(Error::new(EBADF));
        }
        let mut file = self.fs.root_dir().open_file(path).map_err(io_err)?;
        // Seeking past the end is not possible in fatfs; fill the gap.
        let len = file.seek(SeekFrom::End(0)).map_err(io_err)?;
        if offset > len {
            file.write_all(&vec![0; (offset - len) as usize]).map_err(io_err)?;
        }
        file.seek(SeekFrom::Start(offset)).map_err(io_err)?;
        file.write_all(buf).map_err(io_err)?;
        file.flush().map_err(io_err)?;
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
        let mut file = self.fs.root_dir().open_file(path).map_err(io_err)?;
        let old = file.seek(SeekFrom::End(0)).map_err(io_err)?;
        if len < old {
            file.seek(SeekFrom::Start(len)).map_err(io_err)?;
            file.truncate().map_err(io_err)?;
        } else if len > old {
            file.write_all(&vec![0; (len - old) as usize]).map_err(io_err)?;
        }
        file.flush().map_err(io_err)
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
    let file = OpenOptions::new().read(true).write(true).open(disk)?;
    let fs = FileSystem::new(Disk { file, pos: 0 }, FsOptions::new())?;
    eprintln!("fatfsd: {} {:?}, label {:?}", disk, fs.fat_type(), fs.volume_label());

    let socket = Socket::create()?;
    let mut scheme = FatScheme { fs, name: name.to_string(), handles: BTreeMap::new(), next_id: 1 };
    let mut state = SchemeState::new();
    register_sync_scheme(&socket, name, &mut scheme).map_err(io::Error::from)?;
    eprintln!("fatfsd: serving /scheme/{name}");

    loop {
        let req = match socket.next_request(SignalBehavior::Restart)? {
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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: fatfsd <disk> <scheme>   e.g. fatfsd /scheme/disk.mmc/0p0 boot");
        process::exit(1);
    }
    if let Err(err) = run(&args[1], &args[2]) {
        eprintln!("fatfsd: {}: {err}", args[1]);
        process::exit(1);
    }
}
