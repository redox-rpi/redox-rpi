//! `desktop-runas <user> <command> [args...]`: run the desktop session as
//! `<user>` (redox-rpi #805).
//!
//! Orbital runs as root (it serves the display scheme) and starts this
//! program as its session command, so `desktop` needs no second login. It is
//! not setuid on disk: started by anyone but root it refuses. It drops the
//! group, the supplementary groups and then the user, checks that the drop
//! took (and that it cannot go back to root), sets the user's environment
//! and replaces itself with the command.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::os::unix::process::CommandExt;
use std::process::{exit, Command};

struct User {
    name: String,
    uid: libc::uid_t,
    gid: libc::gid_t,
    home: String,
    shell: String,
}

/// Redox's /etc/passwd: `name;uid;gid;full name;home;shell`.
fn find_user(name: &str) -> Option<User> {
    let passwd = fs::read_to_string("/etc/passwd").ok()?;
    for line in passwd.lines() {
        let f: Vec<&str> = line.split(';').collect();
        if f.len() >= 6 && f[0] == name {
            return Some(User {
                name: f[0].to_string(),
                uid: f[1].parse().ok()?,
                gid: f[2].parse().ok()?,
                home: f[4].to_string(),
                shell: f[5].to_string(),
            });
        }
    }
    None
}

/// Supplementary groups from /etc/group (`name;x;gid;member,member`).
fn groups_of(name: &str, primary: libc::gid_t) -> Vec<libc::gid_t> {
    let mut groups = vec![primary];
    if let Ok(group) = fs::read_to_string("/etc/group") {
        for line in group.lines() {
            let f: Vec<&str> = line.split(';').collect();
            if f.len() >= 4 && f[3].split(',').any(|m| m.trim() == name) {
                if let Ok(gid) = f[2].parse::<libc::gid_t>() {
                    if !groups.contains(&gid) {
                        groups.push(gid);
                    }
                }
            }
        }
    }
    groups
}

fn fail(msg: &str) -> ! {
    eprintln!("desktop-runas: {msg}");
    exit(1);
}

fn main() {
    let mut args = env::args_os().skip(1);
    let (Some(name), Some(command)) = (args.next(), args.next()) else {
        eprintln!("usage: desktop-runas <user> <command> [args...]");
        exit(2);
    };
    let rest: Vec<OsString> = args.collect();
    let name = name.into_string().unwrap_or_else(|_| fail("user name is not UTF-8"));

    if unsafe { libc::geteuid() } != 0 {
        fail("must be started by root (it is not setuid)");
    }
    let user = find_user(&name).unwrap_or_else(|| fail(&format!("no user {name} in /etc/passwd")));

    if user.uid != 0 {
        let groups = groups_of(&user.name, user.gid);
        unsafe {
            // gid, then the supplementary groups, then the uid: once the uid
            // is dropped, the others can no longer be changed.
            if libc::setgid(user.gid) != 0 {
                fail(&format!("setgid({}): {}", user.gid, std::io::Error::last_os_error()));
            }
            if libc::setgroups(groups.len() as _, groups.as_ptr()) != 0 {
                fail(&format!("setgroups: {}", std::io::Error::last_os_error()));
            }
            if libc::setuid(user.uid) != 0 {
                fail(&format!("setuid({}): {}", user.uid, std::io::Error::last_os_error()));
            }
            if libc::getuid() != user.uid
                || libc::geteuid() != user.uid
                || libc::getgid() != user.gid
                || libc::getegid() != user.gid
            {
                fail("the user or group did not change");
            }
            if libc::setuid(0) == 0 {
                fail("could go back to root after the drop");
            }
        }
    }

    let _ = env::set_current_dir(&user.home);
    let err = Command::new(command)
        .args(rest)
        .env("USER", &user.name)
        .env("LOGNAME", &user.name)
        .env("HOME", &user.home)
        .env("SHELL", &user.shell)
        .exec();
    fail(&format!("exec: {err}"));
}
