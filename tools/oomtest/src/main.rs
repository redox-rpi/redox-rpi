//! oomtest: use up all memory, so that the kernel must end this process.
//!
//! Modes:
//! - `touch` (default): allocate 4 MiB blocks and write one byte per page,
//!   so the kernel maps each page in the page fault handler.
//! - `pipe`: a thread writes into a pipe, the main thread reads into new
//!   blocks; the kernel fills the pages while it copies the data (usercopy).
//!
//! - `fork-shared [MIB]`: map MIB MiB (default 2048) with MAP_SHARED and
//!   MAP_ANONYMOUS, touch only the first page, and fork. The kernel maps a
//!   shared region lazily, so the fork must allocate every untouched page
//!   (#679). Expected: fork fails with ENOMEM or the process is killed.
//!   With MIB small (e.g. 1), it checks that the child's writes reach the
//!   parent (exit code 0) and that fork works at all.
//!
//! `-t N` starts N idle threads first: all of them must end with the process.
//!
//! The kernel should end the process (SIGKILL, exit status 137 in the shell).
//! Exit code 2: an allocation failed in user space, the kernel path was not
//! reached. The kernel must not panic.

use std::alloc::{alloc, Layout};
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::FromRawFd;
use std::time::Duration;

const BLOCK: usize = 4 << 20;
const PAGE: usize = 4096;

// The compiler may drop an allocation that is never read; the writes are
// volatile and every block is kept in BLOCKS, so the pages are really used.
static mut BLOCKS: Vec<usize> = Vec::new();

fn block(total: usize) -> &'static mut [u8] {
    let layout = Layout::from_size_align(BLOCK, PAGE).unwrap();
    let p = unsafe { alloc(layout) };
    if p.is_null() {
        println!("oomtest: allocation failed after {} MiB", total >> 20);
        std::process::exit(2);
    }
    unsafe {
        (*std::ptr::addr_of_mut!(BLOCKS)).push(p as usize);
        std::slice::from_raw_parts_mut(p, BLOCK)
    }
}

fn report(total: usize) {
    if total % (16 << 20) == 0 {
        println!("oomtest: {} MiB", total >> 20);
        let _ = std::io::stdout().flush();
    }
}

fn touch() {
    let mut total = 0;
    loop {
        let b = block(total);
        for i in (0..BLOCK).step_by(PAGE) {
            unsafe { std::ptr::write_volatile(&mut b[i], 1) };
        }
        total += BLOCK;
        report(total);
    }
}

fn pipe() {
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0, "pipe");
    let mut reader = unsafe { File::from_raw_fd(fds[0]) };
    let mut writer = unsafe { File::from_raw_fd(fds[1]) };
    std::thread::spawn(move || {
        let data = vec![0xa5u8; 64 << 10];
        loop {
            writer.write_all(&data).expect("write");
        }
    });
    let mut total = 0;
    loop {
        let b = block(total);
        reader.read_exact(b).expect("read");
        std::hint::black_box(&b[BLOCK - 1]);
        total += BLOCK;
        report(total);
    }
}

fn fork_shared(mib: usize) {
    let len = mib << 20;
    let p = unsafe {
        libc::mmap(std::ptr::null_mut(), len, libc::PROT_READ | libc::PROT_WRITE,
                   libc::MAP_SHARED | libc::MAP_ANONYMOUS, -1, 0)
    };
    if p == libc::MAP_FAILED {
        println!("oomtest: mmap of {mib} MiB failed: {}", std::io::Error::last_os_error());
        std::process::exit(3);
    }
    let p = p as *mut u8;
    unsafe { std::ptr::write_volatile(p, 1) };
    println!("oomtest: {mib} MiB shared, forking");
    let _ = std::io::stdout().flush();
    let pid = unsafe { libc::fork() };
    if pid < 0 {
        println!("oomtest: fork failed: {}", std::io::Error::last_os_error());
        std::process::exit(4);
    }
    if pid == 0 {
        // Child: write the last page, the parent must see it.
        unsafe { std::ptr::write_volatile(p.add(len - 1), 0x5a) };
        std::process::exit(0);
    }
    let mut status = 0;
    unsafe { libc::waitpid(pid, &mut status, 0) };
    let seen = unsafe { std::ptr::read_volatile(p.add(len - 1)) };
    println!("oomtest: child status {status:#x}, last byte {seen:#x} (0x5a expected)");
    std::process::exit(if seen == 0x5a { 0 } else { 5 });
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut mode = "touch";
    let mut threads = 0;
    let mut mib = 2048;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "touch" | "pipe" => mode = if args[i] == "pipe" { "pipe" } else { "touch" },
            "fork-shared" => {
                mode = "fork-shared";
                if let Some(n) = args.get(i + 1).and_then(|s| s.parse().ok()) {
                    mib = n;
                    i += 1;
                }
            }
            "-t" => {
                i += 1;
                threads = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(0);
            }
            _ => {
                eprintln!("usage: oomtest [touch|pipe|fork-shared [MIB]] [-t THREADS]");
                std::process::exit(64);
            }
        }
        i += 1;
    }
    for _ in 0..threads {
        std::thread::spawn(|| loop {
            std::thread::sleep(Duration::from_secs(1));
        });
    }
    println!("oomtest: mode {mode}, {threads} idle threads, pid {}", std::process::id());
    match mode {
        "pipe" => pipe(),
        "fork-shared" => fork_shared(mib),
        _ => touch(),
    }
}
