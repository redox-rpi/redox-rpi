//! pkg-persist: packages that survive a reset on network and direct boot
//! (issue #44).
//!
//! Those images run their root filesystem from RAM, so whatever `pkg
//! install` puts into / is gone after a reset. pkg-persist installs into
//! the SD card's data partition instead (`pkg --root /scheme/data/pkg`) and
//! copies the installed files into /, at once and again at every boot.
//! Copies, not links: Redox does not follow symlinks into another scheme.
//!
//!     pkg-persist install <package>...   install, then copy into /
//!     pkg-persist remove <package>...    remove from the data partition
//!                                        (the copies in / stay until reset)
//!     pkg-persist update                 update, then copy into /
//!     pkg-persist list                   packages on the data partition
//!     pkg-persist restore                copy into / (boot service)
//!
//! The image wins: a file is copied only if / does not have it, or if
//! pkg-persist copied it itself during this boot (so an update replaces
//! it). The data partition gets the image's etc/pkg.d and etc/pkg/hold
//! before every pkg run, so the same repositories and holds apply.
//! install and update refuse to run while the clock is not set (1970),
//! with a message that says so, instead of pkg's certificate error.
//! Before it copies, pkg-persist checks that the files fit into / (the RAM
//! root is small) and copies nothing if they do not. pkg downloads into
//! <root>/tmp/pkg_download; pkg-persist empties it before and after every
//! pkg run, so a failed run leaves no downloads behind.
//! PKG_PERSIST_ROOT overrides the directory, PKG_PERSIST_DEST the target
//! (/, for tests).

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const DEFAULT_ROOT: &str = "/scheme/data/pkg";
/// The files pkg-persist copied into / during this boot, relative to the
/// target (the root is in RAM, so the list starts empty after a reset).
const COPIED: &str = "var/lib/pkg-persist/copied";
/// pkg's own state and configuration in the root, never copied into /.
const SKIP: [&str; 5] = ["etc/pkg", "etc/pkg.d", "var/lib/packages", "pkg", "tmp"];
/// pkg's download cache in the root (pkg with --root downloads there).
const DOWNLOADS: &str = "tmp/pkg_download";
/// Space left free in / for the running system (logs, /tmp, the desktop).
const KEEP_FREE: u64 = 4 * 1024 * 1024;

/// Before this (2025-01-01) the clock is not set: the Pi has no battery
/// clock and starts at 1970 until timesync sets it.
const CLOCK_SET_AFTER: u64 = 1_735_689_600;

/// HTTPS downloads from static.redox-os.org fail while the clock is wrong,
/// with a certificate error that does not say why: refuse with a clear
/// message instead.
fn check_clock() -> io::Result<()> {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    if secs >= CLOCK_SET_AFTER {
        return Ok(());
    }
    Err(io::Error::other(format!(
        "the clock is not set (it is at {}), so downloads over HTTPS would fail \
         with a certificate error. Wait until timesync has set it (check `date`), \
         then try again",
        1970 + secs / 31_556_952
    )))
}

fn root() -> PathBuf {
    env::var_os("PKG_PERSIST_ROOT").map_or_else(|| PathBuf::from(DEFAULT_ROOT), PathBuf::from)
}

fn dest_root() -> PathBuf {
    env::var_os("PKG_PERSIST_DEST").map_or_else(|| PathBuf::from("/"), PathBuf::from)
}

/// The image's repositories and hold list, so pkg in the root uses them.
fn sync_config(root: &Path) -> io::Result<()> {
    let dir = root.join("etc/pkg.d");
    fs::create_dir_all(&dir)?;
    for entry in fs::read_dir(&dir)? {
        fs::remove_file(entry?.path())?;
    }
    for entry in fs::read_dir("/etc/pkg.d")? {
        let entry = entry?;
        fs::copy(entry.path(), dir.join(entry.file_name()))?;
    }
    let hold = root.join("etc/pkg/hold");
    fs::create_dir_all(root.join("etc/pkg"))?;
    match fs::copy("/etc/pkg/hold", &hold) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let _ = fs::remove_file(&hold);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// Removes pkg's downloads in the root, left behind by a pkg that failed
/// or was stopped; pkg itself empties the directory after every run.
fn clear_downloads(root: &Path) -> io::Result<()> {
    match fs::remove_dir_all(root.join(DOWNLOADS)) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

fn pkg(root: &Path, args: &[String]) -> io::Result<()> {
    sync_config(root)?;
    clear_downloads(root)?;
    let status = Command::new("pkg")
        .arg("--root")
        .arg(root)
        .arg("--yes")
        .args(args)
        .status();
    clear_downloads(root)?;
    let status = status?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("pkg {} failed ({status})", args.join(" "))))
    }
}

/// Copies everything under `root` except pkg's own files into /.
fn restore(root: &Path) -> io::Result<()> {
    if !root.is_dir() {
        println!("pkg-persist: {} does not exist, nothing to copy", root.display());
        return Ok(());
    }
    let start = Instant::now();
    let target = dest_root();
    let list_path = target.join(COPIED);
    let copied_before: BTreeSet<String> = fs::read_to_string(&list_path)
        .map(|s| s.lines().map(String::from).collect())
        .unwrap_or_default();
    let mut copied = copied_before.clone();
    let (mut new, mut kept) = (0, 0);
    let mut needed = 0;
    copy_dir(root, root, &target, &copied_before, &mut copied, &mut new, &mut kept, Some(&mut needed))?;
    let free = free_bytes(&target)?;
    if needed + KEEP_FREE > free {
        return Err(io::Error::other(format!(
            "the packages in {} need {} in {}, but it has only {} free (and {} stay free \
             for the system), so nothing was copied. They stay on the data partition: \
             remove some with `pkg-persist remove`, or use an image with a larger root",
            root.display(),
            mb(needed),
            target.display(),
            mb(free),
            mb(KEEP_FREE)
        )));
    }
    let (mut new, mut kept) = (0, 0);
    copied = copied_before.clone();
    copy_dir(root, root, &target, &copied_before, &mut copied, &mut new, &mut kept, None)?;
    if let Some(dir) = list_path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut list = fs::File::create(&list_path)?;
    for path in &copied {
        writeln!(list, "{path}")?;
    }
    println!(
        "pkg-persist: {new} files copied into {}, {kept} kept from the image ({} ms)",
        target.display(),
        start.elapsed().as_millis()
    );
    Ok(())
}

fn mb(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / 1_000_000.0)
}

/// Bytes free for an ordinary user in the filesystem that holds `path`.
fn free_bytes(path: &Path) -> io::Result<u64> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::other("path with a NUL byte"))?;
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut st) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let unit = if st.f_frsize > 0 { st.f_frsize } else { st.f_bsize };
    Ok(st.f_bavail as u64 * unit as u64)
}

/// With `count`, only adds up the bytes the copy would write and changes
/// nothing; without it, copies.
#[allow(clippy::too_many_arguments)]
fn copy_dir(
    root: &Path,
    dir: &Path,
    target: &Path,
    copied_before: &BTreeSet<String>,
    copied: &mut BTreeSet<String>,
    new: &mut usize,
    kept: &mut usize,
    mut count: Option<&mut u64>,
) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let src = entry?.path();
        let rel = src.strip_prefix(root).unwrap().to_string_lossy().into_owned();
        // pkg's temporary files, left behind when an install fails (pkg
        // exits with 0 even then, so pkg-persist cannot tell otherwise).
        let temp = src.file_name().is_some_and(|n| n.to_string_lossy().starts_with(".pkgar."));
        if SKIP.contains(&rel.as_str()) || temp {
            continue;
        }
        let dest = target.join(&rel);
        let meta = fs::symlink_metadata(&src)?;
        if meta.is_dir() {
            if count.is_none() {
                fs::create_dir_all(&dest)?;
            }
            copy_dir(root, &src, target, copied_before, copied, new, kept, count.as_deref_mut())?;
            continue;
        }
        let exists = fs::symlink_metadata(&dest).is_ok();
        if exists && !copied_before.contains(&rel) {
            *kept += 1;
            continue;
        }
        if exists {
            let same = if meta.file_type().is_symlink() {
                fs::read_link(&src).ok() == fs::read_link(&dest).ok()
            } else {
                meta.is_file() && fs::read(&src)? == fs::read(&dest)?
            };
            if same {
                continue;
            }
            if count.is_none() {
                fs::remove_file(&dest)?;
            }
        }
        if let Some(n) = count.as_deref_mut() {
            // An update frees the old copy first.
            let old = if exists { fs::symlink_metadata(&dest)?.len() } else { 0 };
            *n += meta.len().saturating_sub(old);
            *new += 1;
            continue;
        }
        if meta.file_type().is_symlink() {
            std::os::unix::fs::symlink(fs::read_link(&src)?, &dest)?;
        } else {
            // Also copies the permission bits.
            fs::copy(&src, &dest)?;
        }
        copied.insert(rel);
        *new += 1;
    }
    Ok(())
}

fn list(root: &Path) -> io::Result<()> {
    let dir = root.join("var/lib/packages");
    let mut names: Vec<String> = match fs::read_dir(&dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().to_str()?.strip_suffix(".pkgar_head").map(String::from))
            .collect(),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e),
    };
    names.sort();
    for name in names {
        println!("{name}");
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let root = root();
    let result = match args.first().map(String::as_str) {
        Some("install") if args.len() > 1 => {
            check_clock().and_then(|_| pkg(&root, &args)).and_then(|_| restore(&root))
        }
        Some("update") => check_clock().and_then(|_| pkg(&root, &args)).and_then(|_| restore(&root)),
        Some("remove") if args.len() > 1 => pkg(&root, &args).map(|_| {
            println!("pkg-persist: removed from {}; the copies in / stay until the next reset", root.display())
        }),
        Some("list") => list(&root),
        Some("restore") => restore(&root),
        _ => {
            eprintln!("usage: pkg-persist install|remove <package>... | update | list | restore");
            process::exit(2);
        }
    };
    if let Err(e) = result {
        eprintln!("pkg-persist: {e}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir(name: &str) -> PathBuf {
        let d = env::temp_dir().join(format!("pkg-persist-test-{}-{name}", process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn count_skips_downloads_and_image_files() {
        let root = tempdir("root");
        let target = tempdir("target");
        fs::create_dir_all(root.join("usr/bin")).unwrap();
        fs::create_dir_all(root.join(DOWNLOADS)).unwrap();
        fs::write(root.join("usr/bin/new"), vec![0u8; 1000]).unwrap();
        fs::write(root.join("usr/bin/image"), vec![0u8; 500]).unwrap();
        fs::write(root.join(DOWNLOADS).join("x.pkgar"), vec![0u8; 9000]).unwrap();
        fs::create_dir_all(target.join("usr/bin")).unwrap();
        fs::write(target.join("usr/bin/image"), b"from the image").unwrap();
        let before = BTreeSet::new();
        let mut copied = BTreeSet::new();
        let (mut new, mut kept, mut needed) = (0, 0, 0);
        copy_dir(&root, &root, &target, &before, &mut copied, &mut new, &mut kept, Some(&mut needed)).unwrap();
        assert_eq!((needed, new, kept), (1000, 1, 1));
        assert!(!target.join("usr/bin/new").exists(), "counting must not copy");
        let (mut new, mut kept) = (0, 0);
        copy_dir(&root, &root, &target, &before, &mut copied, &mut new, &mut kept, None).unwrap();
        assert_eq!((new, kept), (1, 1));
        assert_eq!(fs::read(target.join("usr/bin/new")).unwrap().len(), 1000);
        assert!(!target.join("tmp").exists(), "pkg's downloads must not be copied");
    }

    #[test]
    fn count_on_update_is_the_difference() {
        let root = tempdir("root2");
        let target = tempdir("target2");
        fs::write(root.join("f"), vec![1u8; 3000]).unwrap();
        fs::write(target.join("f"), vec![0u8; 1000]).unwrap();
        let before: BTreeSet<String> = ["f".to_string()].into();
        let mut copied = before.clone();
        let (mut new, mut kept, mut needed) = (0, 0, 0);
        copy_dir(&root, &root, &target, &before, &mut copied, &mut new, &mut kept, Some(&mut needed)).unwrap();
        assert_eq!((needed, new), (2000, 1));
    }

    #[test]
    fn restore_refuses_when_it_does_not_fit() {
        let root = tempdir("root3");
        let target = tempdir("target3");
        let free = free_bytes(&target).unwrap();
        // A sparse file larger than the free space: it costs nothing on disk.
        let f = fs::File::create(root.join("big")).unwrap();
        f.set_len(free + 1).unwrap();
        env::set_var("PKG_PERSIST_DEST", &target);
        let err = restore(&root).unwrap_err().to_string();
        env::remove_var("PKG_PERSIST_DEST");
        assert!(err.contains("nothing was copied"), "{err}");
        assert!(!target.join("big").exists());
        let _ = clear_downloads(&root);
    }

    #[test]
    fn clear_downloads_is_quiet_when_there_are_none() {
        let root = tempdir("root4");
        clear_downloads(&root).unwrap();
        fs::create_dir_all(root.join(DOWNLOADS)).unwrap();
        fs::write(root.join(DOWNLOADS).join("a.pkgar"), b"x").unwrap();
        clear_downloads(&root).unwrap();
        assert!(!root.join(DOWNLOADS).exists());
    }
}
