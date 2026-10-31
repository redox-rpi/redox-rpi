//! Checks that taking a signal leaves every register of the interrupted
//! code unchanged.
//!
//! The main thread loads a known pattern into x1-x17, x19-x29 and v0-v31
//! and checks them in a loop while signals arrive. The handler overwrites
//! all of them. A register that differs after a signal shows a bug in the
//! signal entry or return. x18 only with `--x18`: macOS reserves it, and
//! Redox keeps it since relibc 0013 and kernel 0009 (redox-rpi #1, #127).
//! Not checked: x30 (scratch register of the loop), x0 (points to `STATE`), nzcv and
//! sp.
//!
//! Two modes: `process` sends the signals with kill() from a child
//! process, `thread` with pthread_kill() from another thread. They take
//! different paths in the signal entry. A third mode, `sender`, checks
//! si_pid of one signal from another thread and one from a child process.
//! Without an argument these three run. `segv` (only when named) makes
//! loads from an unmapped address: each must raise SIGSEGV, and the program
//! continues after the handler moved pc past the load (relibc 0014).
//!
//! Usage: sigregs [process|thread|sender|segv] [-n <signals>] [--x18]
//! Exit code 0 if every mode passed.

use std::{
    env,
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

#[cfg(not(target_arch = "aarch64"))]
compile_error!("sigregs only supports aarch64");

/// Shared between the check loop, the signal handler and the watchdog.
#[repr(C)]
struct State {
    /// Signals still to take; the handler counts down, the loop ends at 0.
    remaining: AtomicU64,
    /// Set by the watchdog when the signals stop arriving.
    abort: AtomicU64,
    /// Scratch for the check loop.
    scratch: AtomicU64,
}

static STATE: State = State {
    remaining: AtomicU64::new(0),
    abort: AtomicU64::new(0),
    scratch: AtomicU64::new(0),
};

const ABORTED: u64 = 1000;

/// The pattern of register xN: 0xa5a5_00NN_0000_00NN, built in x-register
/// `$reg`.
macro_rules! pattern {
    ($reg:expr, $n:literal) => {
        concat!(
            "    mov ", $reg, ", #", $n, "\n",
            "    movk ", $reg, ", #", $n, ", lsl #32\n",
            "    movk ", $reg, ", #0xa5a5, lsl #48\n",
        )
    };
}

macro_rules! check_loop {
    ($name:ident; ints: $($n:literal)*; vectors: $($v:literal $lo:ident $hi:ident)*) => {
        /// Returns 0 when `remaining` reached 0, ABORTED when the watchdog
        /// stopped it, otherwise the number of the register that changed.
        #[unsafe(naked)]
        extern "C" fn $name(state: *const State) -> u64 {
            std::arch::naked_asm!(concat!(
            // Save the registers the C ABI wants back.
            "    stp x29, x30, [sp, #-16]!\n",
            "    stp x27, x28, [sp, #-16]!\n",
            "    stp x25, x26, [sp, #-16]!\n",
            "    stp x23, x24, [sp, #-16]!\n",
            "    stp x21, x22, [sp, #-16]!\n",
            "    stp x19, x20, [sp, #-16]!\n",
            "    stp d14, d15, [sp, #-16]!\n",
            "    stp d12, d13, [sp, #-16]!\n",
            "    stp d10, d11, [sp, #-16]!\n",
            "    stp d8, d9, [sp, #-16]!\n",
            // Integer registers: their own pattern. Vector register vN: the
            // patterns of two integer registers, a different pair for each.
            $(pattern!(concat!("x", $n), $n),)*
            $(
                "    fmov d", $v, ", ", stringify!($lo), "\n",
                "    mov v", $v, ".d[1], ", stringify!($hi), "\n",
            )*
            "1:\n",
            // On a difference, x30 gets the register number: N for xN,
            // 100 + N for the low and 200 + N for the high half of vN.
            $(
                pattern!("x30", $n),
                "    cmp x", $n, ", x30\n",
                "    mov x30, #", $n, "\n",
                "    b.ne 9f\n",
            )*
            $(
                "    fmov x30, d", $v, "\n",
                "    cmp x30, ", stringify!($lo), "\n",
                "    mov x30, #(100 + ", $v, ")\n",
                "    b.ne 8f\n",
                "    mov x30, v", $v, ".d[1]\n",
                "    cmp x30, ", stringify!($hi), "\n",
                "    mov x30, #(200 + ", $v, ")\n",
                "    b.ne 8f\n",
            )*
            "    ldr x30, [x0]\n", // remaining
            "    cbz x30, 9f\n",
            "    ldr x30, [x0, #8]\n", // abort
            "    cbz x30, 1b\n",
            "    mov x30, #", 1000, "\n", // ABORTED
            "    b 9f\n",
            // A vector register differs. Its pattern comes from integer
            // registers, so check them again first: if one of them changed,
            // report that one.
            "8:\n",
            "    str x30, [x0, #16]\n",
            $(
                pattern!("x30", $n),
                "    cmp x", $n, ", x30\n",
                "    mov x30, #", $n, "\n",
                "    b.ne 9f\n",
            )*
            "    ldr x30, [x0, #16]\n",
            "9:\n",
            "    mov x0, x30\n",
            "    ldp d8, d9, [sp], #16\n",
            "    ldp d10, d11, [sp], #16\n",
            "    ldp d12, d13, [sp], #16\n",
            "    ldp d14, d15, [sp], #16\n",
            "    ldp x19, x20, [sp], #16\n",
            "    ldp x21, x22, [sp], #16\n",
            "    ldp x23, x24, [sp], #16\n",
            "    ldp x25, x26, [sp], #16\n",
            "    ldp x27, x28, [sp], #16\n",
            "    ldp x29, x30, [sp], #16\n",
            "    ret\n",
            ));
        }
    };
}

// With x18: an ordinary register on Linux and Redox, it must survive a
// signal too (redox-rpi #1); on macOS the system owns it.
check_loop!(
    check_loop_x18;
    ints: 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29;
    vectors:
        0 x1 x8 1 x2 x9 2 x3 x10 3 x4 x11 4 x5 x12 5 x6 x13 6 x7 x14 7 x8 x15
        8 x9 x16 9 x10 x17 10 x11 x19 11 x12 x20 12 x13 x21 13 x14 x22
        14 x15 x23 15 x16 x24 16 x17 x25 17 x19 x26 18 x20 x27 19 x21 x28
        20 x22 x29 21 x23 x1 22 x24 x2 23 x25 x3 24 x26 x4 25 x27 x5
        26 x28 x6 27 x29 x7 28 x1 x20 29 x2 x21 30 x3 x22 31 x4 x23
);

check_loop!(
    check_loop;
    ints: 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 19 20 21 22 23 24 25 26 27 28 29;
    vectors:
        0 x1 x8 1 x2 x9 2 x3 x10 3 x4 x11 4 x5 x12 5 x6 x13 6 x7 x14 7 x8 x15
        8 x9 x16 9 x10 x17 10 x11 x19 11 x12 x20 12 x13 x21 13 x14 x22
        14 x15 x23 15 x16 x24 16 x17 x25 17 x19 x26 18 x20 x27 19 x21 x28
        20 x22 x29 21 x23 x1 22 x24 x2 23 x25 x3 24 x26 x4 25 x27 x5
        26 x28 x6 27 x29 x7 28 x1 x20 29 x2 x21 30 x3 x22 31 x4 x23
);

extern "C" fn on_signal(_: libc::c_int) {
    let left = STATE.remaining.load(Ordering::Relaxed);
    if left > 0 {
        STATE.remaining.store(left - 1, Ordering::Relaxed);
    }
    // Overwrite every register the check loop looks at, except x19, which
    // LLVM does not let inline asm use.
    unsafe {
        std::arch::asm!(
            "mov x0, #0xbad", "mov x1, #0xbad", "mov x2, #0xbad", "mov x3, #0xbad",
            "mov x4, #0xbad", "mov x5, #0xbad", "mov x6, #0xbad", "mov x7, #0xbad",
            "mov x8, #0xbad", "mov x9, #0xbad", "mov x10, #0xbad", "mov x11, #0xbad",
            "mov x12, #0xbad", "mov x13, #0xbad", "mov x14, #0xbad", "mov x15, #0xbad",
            "mov x16, #0xbad", "mov x17, #0xbad", "mov x20, #0xbad",
            "mov x21, #0xbad", "mov x22, #0xbad", "mov x23, #0xbad", "mov x24, #0xbad",
            "mov x25, #0xbad", "mov x26, #0xbad", "mov x27, #0xbad", "mov x28, #0xbad",
            "movi v0.16b, #0xee", "movi v1.16b, #0xee", "movi v2.16b, #0xee", "movi v3.16b, #0xee",
            "movi v4.16b, #0xee", "movi v5.16b, #0xee", "movi v6.16b, #0xee", "movi v7.16b, #0xee",
            "movi v8.16b, #0xee", "movi v9.16b, #0xee", "movi v10.16b, #0xee", "movi v11.16b, #0xee",
            "movi v12.16b, #0xee", "movi v13.16b, #0xee", "movi v14.16b, #0xee", "movi v15.16b, #0xee",
            "movi v16.16b, #0xee", "movi v17.16b, #0xee", "movi v18.16b, #0xee", "movi v19.16b, #0xee",
            "movi v20.16b, #0xee", "movi v21.16b, #0xee", "movi v22.16b, #0xee", "movi v23.16b, #0xee",
            "movi v24.16b, #0xee", "movi v25.16b, #0xee", "movi v26.16b, #0xee", "movi v27.16b, #0xee",
            "movi v28.16b, #0xee", "movi v29.16b, #0xee", "movi v30.16b, #0xee", "movi v31.16b, #0xee",
            out("x20") _, out("x21") _, out("x22") _, out("x23") _,
            out("x24") _, out("x25") _, out("x26") _, out("x27") _, out("x28") _,
            out("v8") _, out("v9") _, out("v10") _, out("v11") _,
            out("v12") _, out("v13") _, out("v14") _, out("v15") _,
            clobber_abi("C"),
        );
    }
}

fn describe(code: u64) -> String {
    match code {
        0..=99 => format!("x{code}"),
        100..=199 => format!("low half of v{}", code - 100),
        _ => format!("high half of v{}", code - 200),
    }
}

/// Runs the check loop until `count` signals arrived or none arrived for
/// five seconds. `start_sender` starts sending and returns a function that
/// stops it.
fn run(mode: &str, count: u64, x18: bool, start_sender: impl FnOnce() -> Box<dyn FnOnce()>) -> bool {
    STATE.remaining.store(count, Ordering::SeqCst);
    STATE.abort.store(0, Ordering::SeqCst);

    // Start the sender first: fork() in a process with a second thread
    // crashed now and then on Redox (docs/test-reports.md).
    let stop = start_sender();
    let watchdog = thread::spawn(|| {
        let (mut last, mut since) = (u64::MAX, Instant::now());
        loop {
            let left = STATE.remaining.load(Ordering::SeqCst);
            if left == 0 {
                return;
            }
            if left != last {
                (last, since) = (left, Instant::now());
            } else if since.elapsed() > Duration::from_secs(5) {
                STATE.abort.store(1, Ordering::SeqCst);
                return;
            }
            thread::sleep(Duration::from_millis(100));
        }
    });
    let code = if x18 { check_loop_x18(&STATE) } else { check_loop(&STATE) };
    stop();
    // Let the watchdog end, also when the loop ended on a difference.
    STATE.abort.store(1, Ordering::SeqCst);
    let _ = watchdog.join();

    let arrived = count - STATE.remaining.load(Ordering::SeqCst);
    let ok = match code {
        0 => {
            println!("PASS {mode}: {count} signals, all registers kept");
            true
        }
        ABORTED => {
            println!("FAIL {mode}: only {arrived} of {count} signals arrived, then none for 5 s");
            false
        }
        _ => {
            println!("FAIL {mode}: {} changed after {arrived} signals", describe(code));
            false
        }
    };

    // A signal must not stay blocked after its handler returned.
    let mut set: libc::sigset_t = unsafe { std::mem::zeroed() };
    unsafe { libc::sigprocmask(libc::SIG_BLOCK, std::ptr::null(), &mut set) };
    if unsafe { libc::sigismember(&set, libc::SIGUSR1) } == 1 {
        println!("FAIL {mode}: SIGUSR1 is still blocked afterwards");
        let mut empty: libc::sigset_t = unsafe { std::mem::zeroed() };
        unsafe {
            libc::sigemptyset(&mut empty);
            libc::sigprocmask(libc::SIG_SETMASK, &empty, std::ptr::null_mut());
        }
        return false;
    }
    ok
}

/// si_pid of the last SIGUSR2, for the `sender` mode; 0 until one arrives.
static LAST_PID: AtomicU64 = AtomicU64::new(0);

extern "C" fn on_sigusr2(_: libc::c_int, info: *mut libc::siginfo_t, _: *mut libc::c_void) {
    LAST_PID.store(unsafe { (*info).si_pid() } as u64, Ordering::SeqCst);
}

/// Waits up to 2 s for a SIGUSR2 and returns its si_pid, or None.
fn wait_sigusr2() -> Option<u64> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(2) {
        match LAST_PID.swap(0, Ordering::SeqCst) {
            0 => thread::sleep(Duration::from_millis(1)),
            pid => return Some(pid),
        }
    }
    None
}

/// `sender` mode: si_pid of a signal sent to this thread with pthread_kill()
/// must be our pid, then that of a signal from a child process the child's
/// pid. The thread signal goes first, so a stale sender shows as our pid.
fn sender() -> bool {
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = on_sigusr2
            as extern "C" fn(libc::c_int, *mut libc::siginfo_t, *mut libc::c_void)
            as usize;
        action.sa_flags = libc::SA_SIGINFO;
        libc::sigemptyset(&mut action.sa_mask);
        libc::sigaction(libc::SIGUSR2, &action, std::ptr::null_mut());
    }
    let me = unsafe { libc::getpid() } as u64;
    LAST_PID.store(0, Ordering::SeqCst);
    unsafe { libc::pthread_kill(libc::pthread_self(), libc::SIGUSR2) };
    let from_thread = wait_sigusr2();

    let child = match unsafe { libc::fork() } {
        0 => unsafe {
            libc::kill(me as libc::pid_t, libc::SIGUSR2);
            libc::_exit(0)
        },
        child => child as u64,
    };
    let from_child = wait_sigusr2();
    unsafe { libc::waitpid(child as libc::pid_t, std::ptr::null_mut(), 0) };

    if from_thread == Some(me) && from_child == Some(child) {
        println!("PASS sender: si_pid {me} from the thread, {child} from the child");
        true
    } else {
        println!(
            "FAIL sender: si_pid {from_thread:?} from the thread (expected {me}), \
             {from_child:?} from the child (expected {child})"
        );
        false
    }
}

/// Address of the faulting load in the `segv` mode; unmapped on Redox.
const BAD_ADDR: u64 = 0x10;
/// Offset of the interrupted pc in the context relibc passes to a handler:
/// the 64-byte head of SigStack, then x30-x1, sp and nzcv (ArchIntRegs).
const CTX_PC: usize = 64 + 30 * 8 + 2 * 8;
/// si_addr of the last SIGSEGV; 0 until one arrives.
static SEGV_ADDR: AtomicU64 = AtomicU64::new(0);

extern "C" fn on_sigsegv(_: libc::c_int, info: *mut libc::siginfo_t, ctx: *mut libc::c_void) {
    SEGV_ADDR.store(unsafe { (*info).si_addr() } as u64, Ordering::SeqCst);
    // Continue after the faulting load, one 4-byte instruction.
    unsafe { *ctx.cast::<u8>().add(CTX_PC).cast::<u64>() += 4 };
}

/// `segv` mode: `count` loads from BAD_ADDR. Each must raise SIGSEGV with
/// si_addr BAD_ADDR and return to the next instruction, through the
/// exception entry's return (`svc #0x5352`).
fn segv(count: u64) -> bool {
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = on_sigsegv
            as extern "C" fn(libc::c_int, *mut libc::siginfo_t, *mut libc::c_void)
            as usize;
        action.sa_flags = libc::SA_SIGINFO;
        libc::sigemptyset(&mut action.sa_mask);
        libc::sigaction(libc::SIGSEGV, &action, std::ptr::null_mut());
    }
    for i in 0..count {
        SEGV_ADDR.store(0, Ordering::SeqCst);
        unsafe {
            std::arch::asm!("ldr {v}, [{addr}]", addr = in(reg) BAD_ADDR, v = out(reg) _, options(nostack));
        }
        let addr = SEGV_ADDR.load(Ordering::SeqCst);
        if addr != BAD_ADDR {
            println!("FAIL segv: load {i}: si_addr {addr:#x}, expected {BAD_ADDR:#x}");
            return false;
        }
    }
    println!("PASS segv: {count} faults, each handled and continued");
    true
}

fn process_sender() -> Box<dyn FnOnce()> {
    let parent = unsafe { libc::getpid() };
    match unsafe { libc::fork() } {
        0 => loop {
            unsafe {
                if libc::kill(parent, libc::SIGUSR1) != 0 {
                    libc::_exit(0);
                }
                libc::usleep(300);
            }
        },
        child => {
            println!("process: sending from child {child} to {parent}");
            Box::new(move || unsafe {
            libc::kill(child, libc::SIGKILL);
            libc::waitpid(child, std::ptr::null_mut(), 0);
            })
        }
    }
}

fn thread_sender() -> Box<dyn FnOnce()> {
    let target = unsafe { libc::pthread_self() } as usize;
    let sender = thread::spawn(move || {
        while STATE.remaining.load(Ordering::SeqCst) > 0 && STATE.abort.load(Ordering::SeqCst) == 0 {
            unsafe { libc::pthread_kill(target as libc::pthread_t, libc::SIGUSR1) };
            thread::sleep(Duration::from_micros(300));
        }
    });
    Box::new(move || {
        STATE.abort.store(1, Ordering::SeqCst);
        let _ = sender.join();
    })
}

fn main() {
    let mut modes = Vec::new();
    let mut count = 1000;
    let mut x18 = false;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-n" => count = args.next().and_then(|n| n.parse().ok()).expect("-n <signals>"),
            "--x18" => x18 = true,
            "process" | "thread" | "sender" | "segv" => modes.push(arg),
            _ => {
                eprintln!("usage: sigregs [process|thread|sender|segv] [-n <signals>] [--x18]");
                std::process::exit(2);
            }
        }
    }
    if modes.is_empty() {
        modes = vec!["process".into(), "thread".into(), "sender".into()];
    }

    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = on_signal as extern "C" fn(libc::c_int) as usize;
        libc::sigemptyset(&mut action.sa_mask);
        libc::sigaction(libc::SIGUSR1, &action, std::ptr::null_mut());
    }

    let mut passed = true;
    for mode in &modes {
        passed &= match mode.as_str() {
            "process" => run(mode, count, x18, process_sender),
            "thread" => run(mode, count, x18, thread_sender),
            "segv" => segv(count),
            _ => sender(),
        };
    }
    std::process::exit(if passed { 0 } else { 1 });
}
