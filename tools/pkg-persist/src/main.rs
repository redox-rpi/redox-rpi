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
//! PKG_PERSIST_ROOT overrides the directory, PKG_PERSIST_DEST the target
//! (/, for tests).

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command};

const DEFAULT_ROOT: &str = "/scheme/data/pkg";
/// The files pkg-persist copied into / during this boot, relative to the
/// target (the root is in RAM, so the list starts empty after a reset).
const COPIED: &str = "var/lib/pkg-persist/copied";
/// pkg's own state and configuration in the root, never copied into /.
const SKIP: [&str; 4] = ["etc/pkg", "etc/pkg.d", "var/lib/packages", "pkg"];

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

fn pkg(root: &Path, args: &[String]) -> io::Result<()> {
    sync_config(root)?;
    let status = Command::new("pkg")
        .arg("--root")
        .arg(root)
        .arg("--yes")
        .args(args)
        .status()?;
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
    let target = dest_root();
    let list_path = target.join(COPIED);
    let copied_before: BTreeSet<String> = fs::read_to_string(&list_path)
        .map(|s| s.lines().map(String::from).collect())
        .unwrap_or_default();
    let mut copied = copied_before.clone();
    let (mut new, mut kept) = (0, 0);
    copy_dir(root, root, &target, &copied_before, &mut copied, &mut new, &mut kept)?;
    if let Some(dir) = list_path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut list = fs::File::create(&list_path)?;
    for path in &copied {
        writeln!(list, "{path}")?;
    }
    println!("pkg-persist: {new} files copied into {}, {kept} kept from the image", target.display());
    Ok(())
}

fn copy_dir(
    root: &Path,
    dir: &Path,
    target: &Path,
    copied_before: &BTreeSet<String>,
    copied: &mut BTreeSet<String>,
    new: &mut usize,
    kept: &mut usize,
) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let src = entry?.path();
        let rel = src.strip_prefix(root).unwrap().to_string_lossy().into_owned();
        if SKIP.contains(&rel.as_str()) {
            continue;
        }
        let dest = target.join(&rel);
        let meta = fs::symlink_metadata(&src)?;
        if meta.is_dir() {
            fs::create_dir_all(&dest)?;
            copy_dir(root, &src, target, copied_before, copied, new, kept)?;
            continue;
        }
        let exists = fs::symlink_metadata(&dest).is_ok();
        if exists && !copied_before.contains(&rel) {
            *kept += 1;
            continue;
        }
        if exists {
            if meta.is_file() && fs::read(&src)? == fs::read(&dest)? {
                continue;
            }
            fs::remove_file(&dest)?;
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
        Some("install") if args.len() > 1 => pkg(&root, &args).and_then(|_| restore(&root)),
        Some("update") => pkg(&root, &args).and_then(|_| restore(&root)),
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
