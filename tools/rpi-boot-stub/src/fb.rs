//! A framebuffer from the Pi firmware through the mailbox property interface
//! (issue #33), so the direct boot has a screen without U-Boot.
//!
//! The stub asks the firmware which size it chose for the display (from the
//! monitor's EDID at power-on, or its fallback without a monitor), then for a
//! 32-bit buffer of that size, and gives it to the kernel in the
//! `FRAMEBUFFER_*` environment variables the Redox bootloader writes. `vesad`
//! and the kernel's graphical debug take it from there unchanged.
//!
//! Tags: https://github.com/raspberrypi/firmware/wiki/Mailbox-property-interface

use core::fmt::Write;
use core::ptr::{addr_of, addr_of_mut, read_volatile, write_volatile};

use crate::Uart;

/// VideoCore mailbox of the BCM2837 (Pi 3, 3B+, Zero 2 W).
const MBOX: usize = 0x3F00_B880;
const MBOX_READ: usize = MBOX;
const MBOX_STATUS: usize = MBOX + 0x18;
const MBOX_WRITE: usize = MBOX + 0x20;
const MBOX_WRITE_STATUS: usize = MBOX + 0x38;
const MBOX_FULL: u32 = 1 << 31;
const MBOX_EMPTY: u32 = 1 << 30;
/// Property channel, ARM to VideoCore.
const CHANNEL: u32 = 8;
/// The VideoCore sees ARM memory at this bus address (uncached alias).
const BUS_ALIAS: u32 = 0xC000_0000;

const REQUEST: u32 = 0;
const RESPONSE_OK: u32 = 0x8000_0000;

const TAG_GET_PHYSICAL_SIZE: u32 = 0x0004_0003;
const TAG_ALLOCATE: u32 = 0x0004_0001;
const TAG_GET_PITCH: u32 = 0x0004_0008;
const TAG_SET_PHYSICAL_SIZE: u32 = 0x0004_8003;
const TAG_SET_VIRTUAL_SIZE: u32 = 0x0004_8004;
const TAG_SET_DEPTH: u32 = 0x0004_8005;
const TAG_SET_PIXEL_ORDER: u32 = 0x0004_8006;
const TAG_SET_VIRTUAL_OFFSET: u32 = 0x0004_8009;

/// Pixel order 1 ("RGB" in the firmware's words) gives the byte order
/// B, G, R, x in memory: a little-endian 0x00RRGGBB, what Redox draws.
const PIXEL_ORDER_RGB: u32 = 1;
/// When the firmware reports no display size.
const DEFAULT_SIZE: (u32, u32) = (1024, 768);
const MAX_SIZE: u32 = 4096;

#[repr(C, align(16))]
struct Buffer([u32; 64]);

#[link_section = ".data.zeroed"]
static mut BUF: Buffer = Buffer([0; 64]);

pub struct Framebuffer {
    /// ARM physical address
    pub base: u64,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    /// In pixels, as in `FRAMEBUFFER_STRIDE`
    pub stride: u32,
}

/// Builds a property message in `BUF` tag by tag.
struct Message {
    len: usize,
}

impl Message {
    fn new() -> Self {
        Message { len: 2 }
    }

    /// Adds a tag with `values` and room for `answer` words; returns the
    /// index of its first value word.
    fn tag(&mut self, id: u32, values: &[u32], answer: usize) -> usize {
        let words = values.len().max(answer);
        let buf = unsafe { &mut (*addr_of_mut!(BUF)).0 };
        buf[self.len] = id;
        buf[self.len + 1] = (words * 4) as u32;
        buf[self.len + 2] = REQUEST;
        let first = self.len + 3;
        buf[first..first + words].fill(0);
        buf[first..first + values.len()].copy_from_slice(values);
        self.len = first + words;
        first
    }

    /// Sends the message and waits for the answer. False when the firmware
    /// did not answer with success.
    fn call(self) -> bool {
        let buf = unsafe { &mut (*addr_of_mut!(BUF)).0 };
        buf[self.len] = 0; // end tag
        buf[0] = ((self.len + 1) * 4) as u32;
        buf[1] = REQUEST;
        // The MMU and caches are still off: no cache maintenance needed.
        let addr = addr_of!(BUF) as u32;
        unsafe {
            while read_volatile(MBOX_WRITE_STATUS as *const u32) & MBOX_FULL != 0 {}
            write_volatile(MBOX_WRITE as *mut u32, (addr | BUS_ALIAS) | CHANNEL);
            loop {
                while read_volatile(MBOX_STATUS as *const u32) & MBOX_EMPTY != 0 {}
                if read_volatile(MBOX_READ as *const u32) & 0xF == CHANNEL {
                    break;
                }
            }
        }
        buf[1] == RESPONSE_OK
    }
}

fn word(i: usize) -> u32 {
    unsafe { (*addr_of!(BUF)).0[i] }
}

/// `RPI_FB=WIDTHxHEIGHT` on the command line asks for that size instead of
/// the firmware's, `RPI_FB=off` for no framebuffer.
pub enum Request {
    Firmware,
    Size(u32, u32),
    Off,
}

impl Request {
    pub fn from_bootargs(bootargs: Option<&str>) -> Self {
        let Some(value) = bootargs
            .into_iter()
            .flat_map(|args| args.split_ascii_whitespace())
            .filter_map(|word| word.strip_prefix("RPI_FB="))
            .last()
        else {
            return Request::Firmware;
        };
        if value == "off" {
            return Request::Off;
        }
        let parsed = value
            .split_once('x')
            .and_then(|(w, h)| Some((w.parse::<u32>().ok()?, h.parse::<u32>().ok()?)));
        match parsed {
            Some((w, h)) if (1..=MAX_SIZE).contains(&w) && (1..=MAX_SIZE).contains(&h) => {
                Request::Size(w, h)
            }
            _ => Request::Firmware,
        }
    }
}

/// Asks the firmware for a framebuffer. None when it gives none, then the
/// boot goes on without a screen as before.
pub fn setup(request: Request) -> Option<Framebuffer> {
    let (width, height) = match request {
        Request::Off => return None,
        Request::Size(w, h) => (w, h),
        Request::Firmware => {
            let mut msg = Message::new();
            let i = msg.tag(TAG_GET_PHYSICAL_SIZE, &[], 2);
            let (w, h) = if msg.call() { (word(i), word(i + 1)) } else { (0, 0) };
            if (1..=MAX_SIZE).contains(&w) && (1..=MAX_SIZE).contains(&h) {
                (w, h)
            } else {
                DEFAULT_SIZE
            }
        }
    };

    let mut msg = Message::new();
    let phys = msg.tag(TAG_SET_PHYSICAL_SIZE, &[width, height], 2);
    msg.tag(TAG_SET_VIRTUAL_SIZE, &[width, height], 2);
    msg.tag(TAG_SET_VIRTUAL_OFFSET, &[0, 0], 2);
    let depth = msg.tag(TAG_SET_DEPTH, &[32], 1);
    let order = msg.tag(TAG_SET_PIXEL_ORDER, &[PIXEL_ORDER_RGB], 1);
    let alloc = msg.tag(TAG_ALLOCATE, &[4096], 2);
    let pitch = msg.tag(TAG_GET_PITCH, &[], 1);
    if !msg.call() {
        return None;
    }

    let (width, height) = (word(phys), word(phys + 1));
    let base = (word(alloc) & !BUS_ALIAS) as u64;
    let size = word(alloc + 1) as u64;
    let pitch = word(pitch);
    if word(depth) != 32 || base == 0 || width == 0 || height == 0 || pitch < width * 4 {
        return None;
    }
    if word(order) != PIXEL_ORDER_RGB {
        println!("rpi-boot-stub: framebuffer pixel order {}, colours swapped", word(order));
    }
    if base % 4096 != 0 || size < pitch as u64 * height as u64 {
        println!(
            "rpi-boot-stub: framebuffer at {:#x}, {} bytes: not usable",
            base,
            size
        );
        return None;
    }
    Some(Framebuffer {
        base,
        size,
        width,
        height,
        stride: pitch / 4,
    })
}
