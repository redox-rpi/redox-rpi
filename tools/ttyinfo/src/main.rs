//! ttyinfo: print what a Ctrl-C on the terminal depends on.
//!
//! ttyinfo            print the state of stdin
//! ttyinfo wait N     also wait N seconds for SIGINT (press Ctrl-C)
//! ttyinfo fg N       first make this process the foreground group of the
//!                    terminal (setpgid + tcsetpgrp), then wait N seconds
//! ttyinfo killpg     send SIGINT to our own process group
//! ttyinfo sleep N    sleep N seconds with SIGINT left as it came (no handler)

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

static GOT_SIGINT: AtomicBool = AtomicBool::new(false);

extern "C" fn on_sigint(_: libc::c_int) {
    GOT_SIGINT.store(true, Ordering::SeqCst);
}

fn errno() -> std::io::Error {
    std::io::Error::last_os_error()
}

fn state() {
    unsafe {
        println!("isatty(0)    = {}", libc::isatty(0));
        println!("pid          = {}", libc::getpid());
        println!("pgrp         = {}", libc::getpgrp());
        let fg = libc::tcgetpgrp(0);
        if fg < 0 {
            println!("tcgetpgrp(0) = -1 ({})", errno());
        } else {
            println!("tcgetpgrp(0) = {}", fg);
        }
        let mut t: libc::termios = std::mem::zeroed();
        if libc::tcgetattr(0, &mut t) < 0 {
            println!("tcgetattr(0) failed ({})", errno());
        } else {
            println!(
                "lflag        = {:#x} ISIG={} ICANON={} ECHO={}",
                t.c_lflag,
                t.c_lflag & libc::ISIG != 0,
                t.c_lflag & libc::ICANON != 0,
                t.c_lflag & libc::ECHO != 0
            );
            // The libc crate's termios has a c_line byte before c_cc that
            // relibc's has not: index c_cc one lower to read VINTR.
            println!("VINTR        = {:#x}", t.c_cc[libc::VINTR - 1]);
        }
        let mut old: libc::sigaction = std::mem::zeroed();
        libc::sigaction(libc::SIGINT, std::ptr::null(), &mut old);
        let disp = match old.sa_sigaction {
            libc::SIG_DFL => "default".to_string(),
            libc::SIG_IGN => "ignored".to_string(),
            h => format!("handler {:#x}", h),
        };
        let mut mask: libc::sigset_t = std::mem::zeroed();
        libc::sigprocmask(libc::SIG_BLOCK, std::ptr::null(), &mut mask);
        println!(
            "SIGINT       = {}, blocked={}",
            disp,
            libc::sigismember(&mask, libc::SIGINT) == 1
        );
    }
}

fn wait(secs: u64) {
    unsafe {
        libc::signal(libc::SIGINT, on_sigint as libc::sighandler_t);
    }
    println!("waiting {} s for SIGINT (press Ctrl-C)", secs);
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(secs) {
        if GOT_SIGINT.load(Ordering::SeqCst) {
            println!("ttyinfo: got SIGINT");
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    println!("ttyinfo: no SIGINT");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let secs = |i: usize| args.get(i).and_then(|s| s.parse().ok()).unwrap_or(10);
    match args.first().map(String::as_str) {
        None => state(),
        Some("wait") => {
            state();
            wait(secs(1));
        }
        Some("fg") => unsafe {
            if libc::setpgid(0, 0) < 0 {
                println!("setpgid(0, 0) failed ({})", errno());
            }
            let pgrp = libc::getpgrp();
            if libc::tcsetpgrp(0, pgrp) < 0 {
                println!("tcsetpgrp(0, {}) failed ({})", pgrp, errno());
            }
            state();
            wait(secs(1));
        },
        Some("killpg") => unsafe {
            libc::signal(libc::SIGINT, on_sigint as libc::sighandler_t);
            let pgrp = libc::getpgrp();
            if libc::kill(-pgrp, libc::SIGINT) < 0 {
                println!("kill(-{}, SIGINT) failed ({})", pgrp, errno());
            }
            std::thread::sleep(Duration::from_millis(200));
            println!("got SIGINT: {}", GOT_SIGINT.load(Ordering::SeqCst));
        },
        Some("sleep") => {
            state();
            println!("sleeping {} s", secs(1));
            std::thread::sleep(Duration::from_secs(secs(1)));
            println!("ttyinfo: slept to the end");
        }
        Some(other) => {
            eprintln!("ttyinfo: unknown command {}", other);
            std::process::exit(2);
        }
    }
}
