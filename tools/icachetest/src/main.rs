//! Stress test for instruction cache maintenance on aarch64 (redox-rpi #3).
//!
//! Each round writes `movz w0, #n; ret` into a page as data, makes the page
//! executable and calls it. If the data cache was not cleaned and the
//! instruction cache not invalidated, a real Cortex-A53 can run the code of an
//! earlier round (a wrong n) or garbage. QEMU does not model caches, so there
//! every round passes.
//!
//! Usage: icachetest [rounds]   (default 10000 per test)

use std::ffi::CString;
use std::process::exit;
use std::ptr;

const PAGE: usize = 4096;
const RET: u32 = 0xd65f03c0;

fn movz_w0(n: u32) -> u32 {
    0x52800000 | ((n & 0xffff) << 5)
}

/// Fill the page with `movz w0, #n; ret` pairs, so every 8-byte offset is a
/// function that returns n.
unsafe fn fill(page: *mut u8, n: u32) {
    let words = page as *mut u32;
    for i in 0..PAGE / 8 {
        ptr::write_volatile(words.add(2 * i), movz_w0(n));
        ptr::write_volatile(words.add(2 * i + 1), RET);
    }
}

unsafe fn call(page: *const u8, round: usize) -> u32 {
    let offset = (round * 8) % PAGE;
    let f: extern "C" fn() -> u32 = std::mem::transmute(page.add(offset));
    f()
}

fn mmap(len: usize, prot: i32, flags: i32, fd: i32) -> *mut u8 {
    let p = unsafe { libc::mmap(ptr::null_mut(), len, prot, flags, fd, 0) };
    if p == libc::MAP_FAILED {
        ptr::null_mut()
    } else {
        p as *mut u8
    }
}

fn anon(prot: i32) -> *mut u8 {
    let p = mmap(PAGE, prot, libc::MAP_PRIVATE | libc::MAP_ANONYMOUS, -1);
    if p.is_null() {
        eprintln!("mmap: {}", std::io::Error::last_os_error());
        exit(2);
    }
    p
}

fn mprotect(p: *mut u8, prot: i32) {
    if unsafe { libc::mprotect(p as *mut _, PAGE, prot) } != 0 {
        eprintln!("mprotect: {}", std::io::Error::last_os_error());
        exit(2);
    }
}

fn value(round: usize) -> u32 {
    // Never the same n twice in a row, and never 0 (a zeroed page).
    (round as u32 % 0xfffe) + 1
}

/// New page every round: mmap read-write, write, mprotect read-execute,
/// call, munmap. The allocator hands out the same frames again, so old
/// instructions of that frame can still be in the instruction cache.
fn new_page(rounds: usize) -> usize {
    let mut wrong = 0;
    for round in 0..rounds {
        let n = value(round);
        let p = anon(libc::PROT_READ | libc::PROT_WRITE);
        unsafe { fill(p, n) };
        mprotect(p, libc::PROT_READ | libc::PROT_EXEC);
        if unsafe { call(p, round) } != n {
            wrong += 1;
        }
        unsafe { libc::munmap(p as *mut _, PAGE) };
    }
    wrong
}

/// One page, rewritten every round: mprotect read-write, write, mprotect
/// read-execute, call. Same address, same frame: the classic stale case.
fn same_page(rounds: usize) -> usize {
    let mut wrong = 0;
    let p = anon(libc::PROT_READ | libc::PROT_WRITE);
    for round in 0..rounds {
        let n = value(round);
        mprotect(p, libc::PROT_READ | libc::PROT_WRITE);
        unsafe { fill(p, n) };
        mprotect(p, libc::PROT_READ | libc::PROT_EXEC);
        if unsafe { call(p, round) } != n {
            wrong += 1;
        }
    }
    unsafe { libc::munmap(p as *mut _, PAGE) };
    wrong
}

/// A new file every round, mapped read-execute (no mprotect). A new name,
/// because RedoxFS hands out the old pages for a file that was truncated
/// and written again.
fn file_map(rounds: usize, base: &str) -> Option<usize> {
    let mut wrong = 0;
    let mut old_data = 0;
    let buf = anon(libc::PROT_READ | libc::PROT_WRITE);
    for round in 0..rounds {
        let n = value(round);
        unsafe { fill(buf, n) };
        let path = format!("{base}.{round}");
        let cpath = CString::new(path.as_str()).unwrap();
        let fd = unsafe {
            libc::open(cpath.as_ptr(), libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC, 0o600)
        };
        if fd < 0 {
            eprintln!("file: open {}: {}", path, std::io::Error::last_os_error());
            return None;
        }
        let written = unsafe { libc::write(fd, buf as *const _, PAGE) };
        if written != PAGE as isize {
            eprintln!("file: write: {}", std::io::Error::last_os_error());
            unsafe { libc::close(fd) };
            return None;
        }
        let p = mmap(PAGE, libc::PROT_READ | libc::PROT_EXEC, libc::MAP_PRIVATE, fd);
        if p.is_null() {
            eprintln!("file: mmap: {}", std::io::Error::last_os_error());
            unsafe { libc::close(fd) };
            return None;
        }
        // Read the mapping as data first: if that is already old, the file
        // system handed out old pages, which is not a cache problem.
        let offset = (round * 8) % PAGE;
        let word = unsafe { ptr::read_volatile(p.add(offset) as *const u32) };
        if word != movz_w0(n) {
            if old_data == 0 {
                eprintln!(
                    "file-map: round {round}: the mapping holds movz #{}, the file #{n}",
                    (word >> 5) & 0xffff
                );
            }
            old_data += 1;
        } else if unsafe { call(p, round) } != n {
            wrong += 1;
        }
        unsafe {
            libc::munmap(p as *mut _, PAGE);
            libc::close(fd);
        }
        let _ = std::fs::remove_file(&path);
    }
    unsafe { libc::munmap(buf as *mut _, PAGE) };
    if old_data > 0 {
        println!("file-map: {old_data} of {rounds} mappings held old file data (not a cache problem)");
    }
    Some(wrong)
}

/// One read-write-execute page, rewritten every round with the maintenance
/// done in user space (what a JIT does, __clear_cache). Needs SCTLR_EL1.UCI;
/// without it, dc cvau traps and the program gets a signal.
fn user_flush(rounds: usize) -> usize {
    let mut wrong = 0;
    let p = mmap(
        PAGE,
        libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC,
        libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
        -1,
    );
    if p.is_null() {
        eprintln!("user-flush: mmap rwx: {}", std::io::Error::last_os_error());
        exit(2);
    }
    let ctr: u64;
    unsafe { core::arch::asm!("mrs {}, ctr_el0", out(reg) ctr) };
    let dline = 4usize << ((ctr >> 16) & 0xf);
    let iline = 4usize << (ctr & 0xf);
    for round in 0..rounds {
        let n = value(round);
        unsafe {
            fill(p, n);
            let start = p as usize;
            let mut a = start;
            while a < start + PAGE {
                core::arch::asm!("dc cvau, {}", in(reg) a);
                a += dline;
            }
            core::arch::asm!("dsb ish");
            let mut a = start;
            while a < start + PAGE {
                core::arch::asm!("ic ivau, {}", in(reg) a);
                a += iline;
            }
            core::arch::asm!("dsb ish", "isb");
        }
        if unsafe { call(p, round) } != n {
            wrong += 1;
        }
    }
    unsafe { libc::munmap(p as *mut _, PAGE) };
    wrong
}

fn main() {
    let rounds: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(10000);
    let mut failed = false;
    let mut report = |name: &str, wrong: Option<usize>| match wrong {
        Some(w) => {
            println!("{name}: {w} of {rounds} rounds ran stale code");
            failed |= w > 0;
        }
        None => println!("{name}: skipped"),
    };
    report("new-page", Some(new_page(rounds)));
    report("same-page", Some(same_page(rounds)));
    report("file-map", file_map(rounds, "/tmp/icachetest.bin"));
    report("user-flush", Some(user_flush(rounds)));
    if failed {
        println!("FAIL");
        exit(1);
    }
    println!("PASS");
}
