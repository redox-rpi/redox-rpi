//! kernel8.img for the Raspberry Pi firmware: starts the unchanged Redox
//! kernel the way the Redox bootloader does on aarch64, without U-Boot and
//! UEFI (issue #15, README.md).
//!
//! The firmware (or QEMU `raspi3b` with `-kernel`) starts this image at
//! 0x80000 with the MMU off and `x0` pointing to the devicetree. The stub
//! drops to EL1, builds the memory areas from `/memory`, finds the initramfs
//! through `/chosen`, sets up the page tables the kernel expects and jumps to
//! the kernel's entry point with a `KernelArgs` block.
//!
//! The initramfs is the Redox initfs, optionally followed (at the next 4 KiB
//! boundary) by a RedoxFS image that becomes the live disk.

#![no_std]
#![no_main]

use core::arch::{asm, global_asm};
use core::fmt::Write;
use core::panic::PanicInfo;
use core::ptr::{addr_of, addr_of_mut};

const PAGE: u64 = 4096;
const BLOCK_2M: u64 = 2 * 1024 * 1024;
/// Where the kernel finds all physical memory (`PHYS_OFFSET` in the kernel).
const PHYS_OFFSET: u64 = 0xFFFF_8000_0000_0000;
/// Identity-mapped physical memory, as in the bootloader.
const IDENTITY_GIB: usize = 8;
/// Largest kernel the page tables below can map (4 L3 tables).
const KERNEL_MAX: u64 = 4 * BLOCK_2M;
const STACK_SIZE: usize = 128 * 1024;
const ENV_SIZE: usize = 64 * 1024;

// Page table entry bits, the same values as the bootloader (src/arch/aarch64.rs).
const PF_PRESENT: u64 = 1 << 0;
const PF_TABLE: u64 = 1 << 1;
const PF_PAGE: u64 = 1 << 1;
const PF_ACCESS: u64 = 1 << 10;
const PF_NORMAL_WB: u64 = 0b11 << 8; // inner shareable, MAIR index 0
const PF_DEV: u64 = (0b01 << 8) | (2 << 2); // outer shareable, MAIR index 2

/// PL011 UART of the BCM2837 (Pi 3, 3B+, Zero 2 W); the firmware set it up.
const UART: usize = 0x3F20_1000;

const KERNEL_LEN: usize = include_bytes!(env!("REDOX_KERNEL")).len();

#[repr(C, align(4096))]
struct Aligned<T>(T);

/// The kernel ELF. Its linker script makes the file a flat image: file offset
/// 0 is `KERNEL_OFFSET`, and `.bss` is in the file as zeros. The kernel runs
/// in place, so this memory becomes the kernel's.
#[link_section = ".rodata.kernel"]
static KERNEL: Aligned<[u8; KERNEL_LEN]> = Aligned(*include_bytes!(env!("REDOX_KERNEL")));

#[repr(C, align(4096))]
struct Table([u64; 512]);

// The zero-initialised statics below go to a .data section, not .bss, so
// kernel8.img contains them (see link.ld).

const EMPTY: Table = Table([0; 512]);

#[link_section = ".data.zeroed"]
static mut L0: Table = EMPTY;
#[link_section = ".data.zeroed"]
static mut L1_IDENTITY: Table = EMPTY;
#[link_section = ".data.zeroed"]
static mut L2_IDENTITY: [Table; IDENTITY_GIB] = [EMPTY; IDENTITY_GIB];
#[link_section = ".data.zeroed"]
static mut L1_KERNEL: Table = EMPTY;
#[link_section = ".data.zeroed"]
static mut L2_KERNEL: Table = EMPTY;
#[link_section = ".data.zeroed"]
static mut L3_KERNEL: [Table; 4] = [EMPTY; 4];

#[link_section = ".data.zeroed"]
static mut STACK: Aligned<[u8; STACK_SIZE]> = Aligned([0; STACK_SIZE]);
#[link_section = ".data.zeroed"]
static mut ENV: Aligned<[u8; ENV_SIZE]> = Aligned([0; ENV_SIZE]);

/// Same layout as `KernelArgs` in the kernel (src/startup/mod.rs).
#[repr(C, packed(8))]
struct KernelArgs {
    kernel_base: u64,
    kernel_size: u64,
    stack_base: u64,
    stack_size: u64,
    env_base: u64,
    env_size: u64,
    hwdesc_base: u64,
    hwdesc_size: u64,
    areas_base: u64,
    areas_size: u64,
    bootstrap_base: u64,
    bootstrap_size: u64,
}

#[link_section = ".data.zeroed"]
static mut ARGS: KernelArgs = KernelArgs {
    kernel_base: 0,
    kernel_size: 0,
    stack_base: 0,
    stack_size: 0,
    env_base: 0,
    env_size: 0,
    hwdesc_base: 0,
    hwdesc_size: 0,
    areas_base: 0,
    areas_size: 0,
    bootstrap_base: 0,
    bootstrap_size: 0,
};

/// Same layout as `BootloaderMemoryEntry` in the kernel.
#[derive(Clone, Copy)]
#[repr(C, packed(8))]
struct Area {
    base: u64,
    size: u64,
    kind: u64,
}

const AREA_FREE: u64 = 1;
const AREA_RESERVED: u64 = 3;
const MAX_AREAS: usize = 32;

#[link_section = ".data.zeroed"]
static mut AREAS: [Area; MAX_AREAS] = [Area { base: 0, size: 0, kind: 0 }; MAX_AREAS];
#[link_section = ".data.zeroed"]
static mut AREA_COUNT: usize = 0;

extern "C" {
    static __start: u8;
    static __end: u8;
}

// Entry point. The first 64 bytes are an arm64 Linux `Image` header, so QEMU
// loads the file at text_offset (0x80000) and passes the devicetree in x0,
// as the Pi firmware does.
global_asm!(
    r#"
    .section .text.head, "ax"
    .global _start
_start:
    b       1f
    .word   0
    .quad   0x80000             // text_offset
    .quad   __image_size        // image_size, .bss included
    .quad   0x2                 // flags: little endian, 4 KiB pages
    .quad   0, 0, 0
    .ascii  "ARM\x64"
    .word   0
1:
    mov     x19, x0             // devicetree

    mrs     x1, currentel
    lsr     x1, x1, #2
    cmp     x1, #3
    b.ne    2f
    // EL3: go to EL2 (non-secure, AArch64, HVC enabled)
    msr     cptr_el3, xzr
    mov     x1, #0x5b1
    msr     scr_el3, x1
    mov     x1, #0x3c9          // EL2h, interrupts masked
    msr     spsr_el3, x1
    adr     x1, 2f
    msr     elr_el3, x1
    eret
2:
    mrs     x1, currentel
    lsr     x1, x1, #2
    cmp     x1, #2
    b.ne    3f
    // EL2: go to EL1, the same steps as the bootloader's kernel_entry
    mrs     x1, cnthctl_el2
    orr     x1, x1, #0x3        // timer access from EL1
    msr     cnthctl_el2, x1
    msr     cntvoff_el2, xzr
    mrs     x1, midr_el1
    msr     vpidr_el2, x1
    mrs     x1, mpidr_el1
    msr     vmpidr_el2, x1
    mov     x1, #0x33ff         // no traps
    msr     cptr_el2, x1
    msr     hstr_el2, xzr
    mov     x1, #(3 << 20)      // floating point
    msr     cpacr_el1, x1
    ldr     x1, =0x30d00800     // SCTLR_EL1: MMU and caches off
    msr     sctlr_el1, x1
    ldr     x1, =0xa0000000     // HCR_EL2: EL1 is AArch64, no HVC
    msr     hcr_el2, x1
    mov     x1, #0x3c5          // EL1h, interrupts masked
    msr     spsr_el2, x1
    adr     x1, 3f
    msr     elr_el2, x1
    eret
3:
    mov     x1, #(3 << 20)
    msr     cpacr_el1, x1
    isb
    adrp    x1, {stack}
    add     x1, x1, :lo12:{stack}
    mov     x2, #{stack_size}
    add     sp, x1, x2
    mov     x0, x19
    bl      {main}
4:
    wfe
    b       4b
"#,
    stack = sym STACK,
    stack_size = const STACK_SIZE,
    main = sym stub_main,
);

struct Uart;

impl Write for Uart {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for b in s.bytes() {
            if b == b'\n' {
                putc(b'\r');
            }
            putc(b);
        }
        Ok(())
    }
}

fn putc(b: u8) {
    unsafe {
        // Wait while the transmit FIFO is full.
        while core::ptr::read_volatile((UART + 0x18) as *const u32) & (1 << 5) != 0 {}
        core::ptr::write_volatile(UART as *mut u32, b as u32);
    }
}

macro_rules! println {
    ($($arg:tt)*) => {{
        let _ = writeln!(Uart, $($arg)*);
    }};
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("rpi-boot-stub: {}", info);
    loop {
        unsafe { asm!("wfe") };
    }
}

fn align_up(x: u64) -> u64 {
    (x + PAGE - 1) & !(PAGE - 1)
}

fn align_down(x: u64) -> u64 {
    x & !(PAGE - 1)
}

fn read_u64_le(addr: u64) -> u64 {
    let mut bytes = [0u8; 8];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = unsafe { core::ptr::read_volatile((addr + i as u64) as *const u8) };
    }
    u64::from_le_bytes(bytes)
}

fn bytes_at(addr: u64, len: usize) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(addr as *const u8, len) }
}

unsafe fn add_area(base: u64, size: u64, kind: u64) {
    unsafe {
        let count = AREA_COUNT;
        assert!(count < MAX_AREAS, "too many memory areas");
        (*addr_of_mut!(AREAS))[count] = Area { base, size, kind };
        AREA_COUNT = count + 1;
    }
}

/// Adds [base, end) as free memory, minus the ranges in `used` (sorted).
unsafe fn add_free(base: u64, end: u64, used: &[(u64, u64)]) {
    let mut cur = base;
    for &(start, stop) in used {
        if stop <= cur || start >= end {
            continue;
        }
        if start > cur {
            unsafe { add_free_aligned(cur, start) };
        }
        cur = cur.max(stop);
    }
    if cur < end {
        unsafe { add_free_aligned(cur, end) };
    }
}

unsafe fn add_free_aligned(base: u64, end: u64) {
    let (base, end) = (align_up(base), align_down(end));
    if base < end {
        unsafe { add_area(base, end - base, AREA_FREE) };
    }
}

/// Writes environment lines for the kernel and userspace, as the bootloader
/// does ("NAME=value", values in hex with 16 digits).
struct EnvWriter {
    len: usize,
}

impl Write for EnvWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let env = unsafe { &mut (*addr_of_mut!(ENV)).0 };
        let end = self.len + s.len();
        if end > env.len() {
            return Err(core::fmt::Error);
        }
        env[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

unsafe extern "C" fn stub_main(dtb_addr: u64) -> ! {
    unsafe { boot(dtb_addr) }
}

unsafe fn boot(dtb_addr: u64) -> ! {
    println!("rpi-boot-stub: devicetree at {:#x}", dtb_addr);
    let fdt = unsafe { fdt::Fdt::from_ptr(dtb_addr as *const u8) }
        .unwrap_or_else(|e| panic!("no valid devicetree: {:?}", e));
    let dtb_size = fdt.total_size() as u64;

    let image_start = addr_of!(__start) as u64;
    let image_end = addr_of!(__end) as u64;
    let kernel_base = addr_of!(KERNEL) as u64;
    let kernel_size = KERNEL_LEN as u64;
    assert!(kernel_size <= KERNEL_MAX, "kernel larger than {} MiB", KERNEL_MAX >> 20);
    let kernel = bytes_at(kernel_base, KERNEL_LEN);
    assert!(&kernel[..4] == b"\x7fELF", "embedded kernel is not an ELF file");
    let entry = read_u64_le(kernel_base + 24);

    // The initramfs: initfs, then optionally a RedoxFS image.
    let chosen = fdt.find_node("/chosen").expect("no /chosen in the devicetree");
    let initrd_start = chosen
        .property("linux,initrd-start")
        .and_then(|p| p.as_usize())
        .expect("no initramfs (linux,initrd-start in /chosen)") as u64;
    let initrd_end = chosen
        .property("linux,initrd-end")
        .and_then(|p| p.as_usize())
        .expect("no linux,initrd-end in /chosen") as u64;
    let mut initrd = initrd_start;
    if initrd % PAGE != 0 {
        // The kernel needs the initfs page-aligned: move it up.
        let target = align_up(initrd);
        unsafe {
            core::ptr::copy(
                initrd as *const u8,
                target as *mut u8,
                (initrd_end - initrd_start) as usize,
            )
        };
        initrd = target;
    }
    let initrd_len = initrd_end - initrd_start;
    let initrd_used_end = align_up(initrd + initrd_len);
    println!(
        "rpi-boot-stub: initramfs {:#x}..{:#x}, {} KiB",
        initrd,
        initrd + initrd_len,
        initrd_len >> 10
    );
    assert!(
        bytes_at(initrd, 8) == b"RedoxFtw",
        "the initramfs does not start with a Redox initfs"
    );
    let initfs_size = read_u64_le(initrd + 0x0c);
    assert!(initfs_size <= initrd_len, "initfs larger than the initramfs");
    let live_base = initrd + align_up(initfs_size);
    let live = if live_base + PAGE <= initrd + initrd_len && bytes_at(live_base, 8) == b"RedoxFS\0" {
        let size = read_u64_le(live_base + 0x20).min(initrd + initrd_len - live_base);
        Some((live_base, size))
    } else {
        None
    };

    // Memory in use, sorted by start: everything below the end of this image
    // (firmware spin tables, stub, kernel, page tables, stack), the
    // devicetree, the initramfs and the /memreserve/ entries.
    let mut used = [(0u64, 0u64); 16];
    let mut n = 0;
    let mut reserve = |start: u64, end: u64| {
        if n < used.len() {
            used[n] = (start, end);
            n += 1;
        }
    };
    reserve(0, image_end);
    reserve(dtb_addr, dtb_addr + dtb_size);
    reserve(initrd_start.min(initrd), initrd_used_end);
    for r in fdt.memory_reservations() {
        let start = r.address() as u64;
        reserve(start, start + r.size() as u64);
    }
    let used = &mut used[..n];
    used.sort_unstable_by_key(|r| r.0);

    let mut ram = [(0u64, 0u64); 8];
    let mut ram_count = 0;
    for region in fdt.memory().regions() {
        let base = region.starting_address as u64;
        let size = region.size.unwrap_or(0) as u64;
        if size != 0 && ram_count < ram.len() {
            ram[ram_count] = (base, base + size);
            ram_count += 1;
            unsafe { add_free(base, base + size, used) };
        }
    }
    let ram = &ram[..ram_count];
    assert!(!ram.is_empty(), "no /memory in the devicetree");
    if let Some((base, size)) = live {
        unsafe { add_area(base, align_up(size), AREA_RESERVED) };
    }

    // Environment
    let mut env = EnvWriter { len: 0 };
    let _ = writeln!(env, "DTB_ADDR={:016x}", dtb_addr);
    let _ = writeln!(env, "DTB_SIZE={:016x}", dtb_size);
    if let Some((base, size)) = live {
        let uuid = bytes_at(base + 0x10, 16);
        let _ = writeln!(env, "DISK_LIVE_ADDR={:016x}", base);
        let _ = writeln!(env, "DISK_LIVE_SIZE={:016x}", size);
        let _ = writeln!(env, "REDOXFS_BLOCK={:016x}", 0);
        let _ = write!(env, "REDOXFS_UUID=");
        for (i, b) in uuid.iter().enumerate() {
            if i == 4 || i == 6 || i == 8 || i == 10 {
                let _ = write!(env, "-");
            }
            let _ = write!(env, "{:02x}", b);
        }
        let _ = writeln!(env);
    }
    // The command line (cmdline.txt, after the firmware's own Linux options)
    // is in /chosen bootargs. Words of the form NAME=value with an upper-case
    // NAME are Redox environment variables, as with the bootloader's
    // environment editor; the firmware's lower-case Linux options are left out.
    if let Some(bootargs) = chosen.property("bootargs").and_then(|p| p.as_str()) {
        for word in bootargs.split_ascii_whitespace() {
            if let Some((name, _)) = word.split_once('=') {
                if !name.is_empty()
                    && name.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
                {
                    println!("rpi-boot-stub: from the command line: {}", word);
                    let _ = writeln!(env, "{}", word);
                }
            }
        }
    }

    unsafe {
        let args = &mut *addr_of_mut!(ARGS);
        args.kernel_base = kernel_base;
        args.kernel_size = kernel_size;
        args.stack_base = addr_of!(STACK) as u64;
        args.stack_size = STACK_SIZE as u64;
        args.env_base = addr_of!(ENV) as u64;
        args.env_size = env.len as u64;
        args.hwdesc_base = dtb_addr;
        args.hwdesc_size = dtb_size;
        args.areas_base = addr_of!(AREAS) as u64;
        args.areas_size = (AREA_COUNT * core::mem::size_of::<Area>()) as u64;
        args.bootstrap_base = initrd;
        args.bootstrap_size = align_up(initfs_size);
    }

    for i in 0..unsafe { AREA_COUNT } {
        let a = unsafe { (*addr_of!(AREAS))[i] };
        let (base, size, kind) = (a.base, a.size, a.kind);
        println!(
            "rpi-boot-stub: area {:#x}..{:#x} {}",
            base,
            base + size,
            if kind == AREA_FREE { "free" } else { "reserved" }
        );
    }
    match live {
        Some((base, size)) => println!("rpi-boot-stub: live disk {:#x}, {} MiB", base, size >> 20),
        None => println!("rpi-boot-stub: no RedoxFS image after the initfs, no live disk"),
    }

    let ttbr = unsafe { build_page_tables(ram, kernel_base, kernel_size) };
    println!("rpi-boot-stub: starting the kernel at {:#x}", entry);

    unsafe {
        // The tables were written with the MMU and caches off: drop any stale
        // cache lines for this image before the MMU reads it cached.
        let mut line = image_start & !63;
        while line < image_end {
            asm!("dc ivac, {}", in(reg) line);
            line += 64;
        }
        asm!("dsb sy", "ic iallu", "dsb sy", "isb");
        enter_kernel(
            ttbr,
            addr_of!(STACK) as u64 + STACK_SIZE as u64 + PHYS_OFFSET,
            entry,
            addr_of!(ARGS) as u64,
        )
    }
}

/// The same layout as the bootloader's `paging_create`: the first 8 GiB
/// identity-mapped and at `PHYS_OFFSET` (2 MiB blocks; RAM write-back,
/// everything else device memory), the kernel at `KERNEL_OFFSET`.
unsafe fn build_page_tables(ram: &[(u64, u64)], kernel_base: u64, kernel_size: u64) -> u64 {
    unsafe {
        let l0 = &mut (*addr_of_mut!(L0)).0;
        let l1 = &mut (*addr_of_mut!(L1_IDENTITY)).0;
        let l2s = &mut *addr_of_mut!(L2_IDENTITY);

        let table = |t: *const u64| t as u64 | PF_ACCESS | PF_TABLE | PF_PRESENT;

        l0[0] = table(l1.as_ptr());
        l0[256] = table(l1.as_ptr());
        for (gib, l2) in l2s.iter_mut().enumerate() {
            l1[gib] = table(l2.0.as_ptr());
            for (i, entry) in l2.0.iter_mut().enumerate() {
                let addr = gib as u64 * 0x4000_0000 + i as u64 * BLOCK_2M;
                let is_ram = ram.iter().any(|&(s, e)| s <= addr && addr + BLOCK_2M <= e);
                let flags = if is_ram { PF_NORMAL_WB } else { PF_DEV };
                *entry = addr | PF_ACCESS | flags | PF_PRESENT;
            }
        }

        let l1k = &mut (*addr_of_mut!(L1_KERNEL)).0;
        let l2k = &mut (*addr_of_mut!(L2_KERNEL)).0;
        let l3ks = &mut *addr_of_mut!(L3_KERNEL);
        l0[510] = table(l1k.as_ptr());
        l1k[0] = table(l2k.as_ptr());
        let pages = align_up(kernel_size) / PAGE;
        for page in 0..pages {
            let (l2_i, l3_i) = ((page / 512) as usize, (page % 512) as usize);
            if l3_i == 0 {
                l2k[l2_i] = table(l3ks[l2_i].0.as_ptr());
            }
            l3ks[l2_i].0[l3_i] =
                (kernel_base + page * PAGE) | PF_ACCESS | PF_NORMAL_WB | PF_PAGE | PF_PRESENT;
        }

        l0.as_ptr() as u64
    }
}

/// MMU on with the bootloader's MAIR, TCR and SCTLR values, then the jump.
unsafe fn enter_kernel(ttbr: u64, stack: u64, entry: u64, args: u64) -> ! {
    unsafe {
        // Fixed registers: a noreturn asm block cannot have outputs, and x0
        // must hold the arguments at the jump.
        asm!(
            "msr mair_el1, x4",
            "mrs x9, id_aa64mmfr0_el1",
            "bfi x5, x9, #32, #3",      // physical address size
            "msr tcr_el1, x5",
            "isb",
            "dsb sy",
            "msr ttbr1_el1, x3",
            "msr ttbr0_el1, x3",
            "isb",
            "tlbi vmalle1",
            "dsb ish",
            "isb",
            "mrs x9, sctlr_el1",
            "bic x9, x9, x6",
            "orr x9, x9, x7",
            "msr sctlr_el1, x9",
            "isb",
            "mov sp, x1",
            "br x2",
            in("x0") args,
            in("x1") stack,
            in("x2") entry,
            in("x3") ttbr,
            in("x4") 0x0000_0000_BB00_44FFu64, // MAIR: WB, NC, device, WT
            in("x5") 0x10b5103510u64,         // TCR, 48-bit, 4 KiB granule
            in("x6") 0x32802c2u64,            // SCTLR bits to clear
            in("x7") 0x3485d13du64,           // SCTLR bits to set (MMU, caches)
            in("x9") 0u64,
            options(noreturn)
        );
    }
}
