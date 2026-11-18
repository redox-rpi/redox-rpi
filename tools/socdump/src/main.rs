//! `socdump`: read the Raspberry Pi's clock, PLL and GPIO registers and
//! print them decoded, like `pll-inspector` of librerpi/rpi-tools. It only
//! reads; nothing here writes a register.
//!
//! ```text
//! socdump [plls|clocks|gpio|all]   decoded (default all)
//! socdump display                  HVS, pixel valves and HDMI, decoded
//! socdump check                    compare with /scheme/rpi-firmware/clocks
//! socdump regs                     every register as `block address value name`
//! socdump diff A B                 registers that differ between two `regs` dumps
//! ```
//!
//! The chip comes from the devicetree's root `compatible` (or the board
//! revision in `/scheme/rpi-firmware/board`), the peripheral address from
//! its `/soc` ranges and the crystal from `/clocks/clk-osc`. Register
//! tables are per chip (`chip.rs`); today the BCM2837 (Pi 3B, 3B+, Zero 2 W).
//!
//! On Redox it maps `/scheme/memory/physical@dev` and needs root. It also
//! builds for Linux on the Pi, where it maps `/dev/mem`, to compare with
//! Raspberry Pi OS.

mod chip;
mod display;

use chip::{Chip, Parents, CHIPS, CM_BASE, CM_LEN, CM_LOCK, GPIO_BASE, GPIO_LEN};
use std::collections::HashMap;
use std::fs::File;
use std::os::fd::AsRawFd;
use std::process::ExitCode;

const USAGE: &str = "usage: socdump [plls|clocks|gpio|display|all|check|regs] | socdump diff A B";
const FIRMWARE_CLOCKS: &str = "/scheme/rpi-firmware/clocks";
const FIRMWARE_BOARD: &str = "/scheme/rpi-firmware/board";

#[cfg(target_os = "redox")]
const DTB: &str = "/scheme/kernel.dtb";
#[cfg(not(target_os = "redox"))]
const DTB: &str = "/sys/firmware/fdt";

#[cfg(target_os = "redox")]
const MEMORY: &str = "/scheme/memory/physical@dev";
#[cfg(not(target_os = "redox"))]
const MEMORY: &str = "/dev/mem";

/// A read-only mapping of a register block.
struct Window {
    ptr: *const u8,
    len: usize,
    /// Bus address of the first byte.
    bus: u32,
    _file: File,
}

impl Window {
    fn map(phys: u64, bus: u32, len: usize) -> Result<Window, String> {
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(target_os = "linux")]
        std::os::unix::fs::OpenOptionsExt::custom_flags(&mut options, libc::O_SYNC);
        let file = options.open(MEMORY).map_err(|err| format!("{MEMORY}: {err} (root needed)"))?;
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ,
                libc::MAP_SHARED,
                file.as_raw_fd(),
                phys as libc::off_t,
            )
        };
        if ptr == libc::MAP_FAILED {
            return Err(format!("map 0x{phys:x}: {}", std::io::Error::last_os_error()));
        }
        Ok(Window { ptr: ptr as *const u8, len, bus, _file: file })
    }

    fn read(&self, offset: u32) -> u32 {
        assert!(offset % 4 == 0 && offset as usize + 4 <= self.len, "offset 0x{offset:x} outside the window");
        unsafe { std::ptr::read_volatile(self.ptr.add(offset as usize) as *const u32) }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe { libc::munmap(self.ptr as *mut libc::c_void, self.len) };
    }
}

/// What the devicetree (and the firmware) say about the board.
struct Board {
    chip: &'static Chip,
    /// (child bus address, parent CPU address, size) from `/soc` ranges.
    ranges: Vec<(u64, u64, u64)>,
    xosc: u32,
    xosc_from_dt: bool,
}

impl Board {
    fn phys(&self, bus: u32) -> Result<u64, String> {
        let bus = u64::from(bus);
        self.ranges
            .iter()
            .find(|(child, _, size)| bus >= *child && bus - child < *size)
            .map(|(child, parent, _)| parent + (bus - child))
            .ok_or_else(|| format!("bus address 0x{bus:x} not in the devicetree's /soc ranges"))
    }

    fn map(&self, bus: u32, len: usize) -> Result<Window, String> {
        Window::map(self.phys(bus)?, bus, len)
    }
}

/// Big-endian cells of a property as numbers of `cells` words each.
fn cells(bytes: &[u8], cells: usize) -> impl Iterator<Item = u64> + '_ {
    bytes.chunks_exact(4 * cells.max(1)).map(|chunk| {
        chunk.chunks_exact(4).fold(0u64, |acc, word| (acc << 32) | u64::from(u32::from_be_bytes(word.try_into().unwrap())))
    })
}

fn parse_ranges(value: &[u8], child_cells: usize, parent_cells: usize, size_cells: usize) -> Vec<(u64, u64, u64)> {
    let entry = 4 * (child_cells + parent_cells + size_cells);
    value
        .chunks_exact(entry)
        .map(|e| {
            let child = cells(&e[..4 * child_cells], child_cells).next().unwrap_or(0);
            let parent = cells(&e[4 * child_cells..4 * (child_cells + parent_cells)], parent_cells).next().unwrap_or(0);
            let size = cells(&e[4 * (child_cells + parent_cells)..], size_cells).next().unwrap_or(0);
            (child, parent, size)
        })
        .collect()
}

fn firmware_processor() -> Option<u32> {
    let board = std::fs::read_to_string(FIRMWARE_BOARD).ok()?;
    let revision = board.lines().find_map(|line| line.strip_prefix("revision "))?;
    let revision = u32::from_str_radix(revision.trim().trim_start_matches("0x"), 16).ok()?;
    (revision & (1 << 23) != 0).then_some((revision >> 12) & 0xf)
}

fn board() -> Result<Board, String> {
    let dtb = std::fs::read(DTB).map_err(|err| format!("{DTB}: {err}"))?;
    let fdt = fdt::Fdt::new(&dtb).map_err(|err| format!("{DTB}: {err:?}"))?;

    let compatible: Vec<&str> = fdt.root().compatible().all().collect();
    let chip = CHIPS
        .iter()
        .find(|chip| chip.compatible.iter().any(|c| compatible.contains(c)))
        .or_else(|| {
            let processor = firmware_processor()?;
            CHIPS.iter().find(|chip| chip.revision_processor == processor)
        })
        .ok_or_else(|| format!("no register table for this board (compatible {compatible:?}); known: {}",
            CHIPS.iter().map(|c| c.name).collect::<Vec<_>>().join(", ")))?;

    let address_cells = |path: &str, default: usize| {
        fdt.find_node(path)
            .and_then(|node| node.property("#address-cells"))
            .and_then(|prop| prop.as_usize())
            .unwrap_or(default)
    };
    let soc = fdt.find_node("/soc").ok_or("devicetree has no /soc")?;
    let size_cells = soc.property("#size-cells").and_then(|p| p.as_usize()).unwrap_or(1);
    let ranges = soc
        .property("ranges")
        .map(|prop| parse_ranges(prop.value, address_cells("/soc", 1), address_cells("/", 1), size_cells))
        .unwrap_or_default();
    if ranges.is_empty() {
        return Err("devicetree /soc has no ranges".into());
    }

    let osc = fdt
        .find_node("/clocks/clk-osc")
        .or_else(|| fdt.all_nodes().find(|node| node.name.starts_with("clk-osc") || node.name.starts_with("clk_osc")))
        .and_then(|node| node.property("clock-frequency"))
        .and_then(|prop| prop.as_usize());
    Ok(Board {
        chip,
        ranges,
        xosc: osc.map_or(chip.xosc_hz, |hz| hz as u32),
        xosc_from_dt: osc.is_some(),
    })
}

fn mhz(hz: f64) -> String {
    format!("{:.3}", hz / 1e6)
}

/// Rates of all PLL channels, by name, for the clock muxes.
fn channel_rates(board: &Board, cm: &Window) -> HashMap<&'static str, f64> {
    let mut rates = HashMap::new();
    rates.insert("xosc", f64::from(board.xosc));
    for pll in board.chip.plls {
        let ctrl = cm.read(pll.ctrl);
        let on = !chip::pll_powered_down(ctrl);
        let rate = if on { pll_rate(board, cm, pll) } else { 0.0 };
        for channel in pll.channels {
            let (div, off) = chip::channel_div(cm.read(channel.reg));
            let out = if off { 0.0 } else { rate / f64::from(div) / f64::from(channel.fixed_div) };
            rates.insert(channel.name, out);
        }
    }
    rates
}

fn pll_rate(board: &Board, cm: &Window, pll: &chip::Pll) -> f64 {
    let prediv = board.chip.pll_prediv && cm.read(pll.ana + 4) & (1 << pll.prediv_bit) != 0;
    chip::pll_rate(board.xosc, cm.read(pll.ctrl), cm.read(pll.frac), prediv)
}

fn print_plls(board: &Board, cm: &Window) {
    let lock = cm.read(CM_LOCK);
    println!("PLLs (CM_LOCK 0x{lock:08x})");
    println!("{:<11} {:<9} {:>5} {:>8} {:>4} {:>6} {:>10}   {:<10} {:<10} {:<10}", "name", "state", "ndiv", "frac", "pdiv", "prediv", "MHz", "ctrl", "frac", "ana1");
    for pll in board.chip.plls {
        let (ctrl, frac, ana1) = (cm.read(pll.ctrl), cm.read(pll.frac), cm.read(pll.ana + 4));
        let down = chip::pll_powered_down(ctrl);
        let locked = lock & (1 << pll.lock_bit) != 0;
        let state = match (down, locked) {
            (true, _) => "off",
            (false, true) => "locked",
            (false, false) => "unlocked",
        };
        let prediv = board.chip.pll_prediv && ana1 & (1 << pll.prediv_bit) != 0;
        let rate = if down { 0.0 } else { pll_rate(board, cm, pll) };
        println!(
            "{:<11} {:<9} {:>5} {:>8} {:>4} {:>6} {:>10}   0x{ctrl:08x} 0x{frac:08x} 0x{ana1:08x}",
            pll.name, state, ctrl & 0x3ff, frac & 0xf_ffff, (ctrl >> 12) & 7, if prediv { "x2" } else { "-" }, mhz(rate)
        );
        for channel in pll.channels {
            let reg = cm.read(channel.reg);
            let (div, off) = chip::channel_div(reg);
            let fixed = if channel.fixed_div > 1 { format!("/{div}/{}", channel.fixed_div) } else { format!("/{div}") };
            let out = if off || down { 0.0 } else { rate / f64::from(div) / f64::from(channel.fixed_div) };
            println!("  {:<13} {:<9} {:>20} {:>10}   0x{reg:08x}", channel.name, if off { "off" } else { "on" }, fixed, mhz(out));
        }
    }
    println!();
}

fn print_clocks(board: &Board, cm: &Window) {
    let rates = channel_rates(board, cm);
    println!("Clocks");
    println!("{:<7} {:<11} {:<3} {:<4} {:>10} {:>10}   {:<10} {:<10}", "name", "source", "on", "mash", "divider", "MHz", "ctl", "div");
    for clock in board.chip.clocks {
        let (ctl, div) = (cm.read(clock.ctl), cm.read(clock.ctl + 4));
        let c = chip::clock_ctl(ctl);
        let source = clock.parents.name(c.src);
        let divider = chip::clock_div(clock, div);
        let parent = if clock.parents == Parents::Dsi { None } else { rates.get(source).copied() };
        let firmware = board.chip.firmware_clocks.contains(&clock.name);
        let rate = match (c.enabled, parent, divider) {
            _ if firmware => None,
            (false, _, _) => Some(0.0),
            (true, Some(p), Some(d)) if d > 0.0 => Some(p / d),
            (true, Some(p), None) => Some(p),
            (true, Some(_), Some(_)) => Some(0.0),
            (true, None, _) => None,
        };
        println!(
            "{:<7} {:<11} {:<3} {:<4} {:>10} {:>10}   0x{ctl:08x} 0x{div:08x}",
            clock.name,
            source,
            format!("{}{}", if c.enabled { "y" } else { "-" }, if c.busy { "b" } else { "" }),
            c.mash,
            divider.map_or("-".into(), |d| format!("{d:.4}")),
            rate.map_or(if firmware { "fw".into() } else { "?".into() }, mhz),
        );
    }
    if !board.chip.firmware_clocks.is_empty() {
        println!("fw: the firmware drives this clock; its registers keep their reset values");
    }
    println!();
}

fn print_gpio(board: &Board, gpio: &Window) {
    println!("GPIO (function, level)");
    let pins = board.chip.gpio_pins;
    let level = |pin: u32| (gpio.read(0x34 + 4 * (pin / 32)) >> (pin % 32)) & 1;
    let function = |pin: u32| chip::gpio_function(gpio.read(4 * (pin / 10)) >> (3 * (pin % 10)));
    let rows = pins.div_ceil(4);
    for row in 0..rows {
        let mut line = String::new();
        for col in 0..4 {
            let pin = row + col * rows;
            if pin < pins {
                line.push_str(&format!("{pin:>3} {:<5} {}    ", function(pin), level(pin)));
            }
        }
        println!("{}", line.trim_end());
    }
    println!();
}

/// Firmware clock name -> our name. `arm` is the PLLB ARM channel; the
/// ARM side may divide it further, so a mismatch there is reported but
/// expected to be a factor of 2.
const FIRMWARE_NAMES: &[(&str, &str)] = &[
    ("emmc", "emmc"),
    ("uart", "uart"),
    ("arm", "pllb_arm"),
    ("core", "vpu"),
    ("v3d", "v3d"),
    ("h264", "h264"),
    ("isp", "isp"),
    ("sdram", "sdram"),
    ("pwm", "pwm"),
];

/// Rate of a clock or PLL channel by our name.
fn rate_by_name(board: &Board, cm: &Window, name: &str) -> Option<f64> {
    let rates = channel_rates(board, cm);
    if let Some(rate) = rates.get(name) {
        return Some(*rate);
    }
    let clock = board.chip.clocks.iter().find(|c| c.name == name)?;
    let c = chip::clock_ctl(cm.read(clock.ctl));
    if !c.enabled {
        return Some(0.0);
    }
    let parent = rates.get(clock.parents.name(c.src)).copied()?;
    match chip::clock_div(clock, cm.read(clock.ctl + 4)) {
        Some(d) if d > 0.0 => Some(parent / d),
        Some(_) => Some(0.0),
        None => Some(parent),
    }
}

fn check(board: &Board, cm: &Window) -> Result<bool, String> {
    let text = std::fs::read_to_string(FIRMWARE_CLOCKS).map_err(|err| format!("{FIRMWARE_CLOCKS}: {err}"))?;
    println!("{:<7} {:<9} {:>12} {:>12}  result", "clock", "register", "firmware MHz", "socdump MHz");
    let mut all_ok = true;
    for line in text.lines() {
        let mut words = line.split_whitespace();
        let (Some(name), Some(rate)) = (words.next(), words.next().and_then(|w| w.parse::<f64>().ok())) else {
            continue;
        };
        let Some(&(_, ours)) = FIRMWARE_NAMES.iter().find(|(fw, _)| *fw == name) else {
            println!("{name:<7} {:<9} {:>12} {:>12}  not decoded", "-", mhz(rate), "-");
            continue;
        };
        if board.chip.firmware_clocks.contains(&ours) {
            println!("{name:<7} {ours:<9} {:>12} {:>12}  set by the firmware, not readable here", mhz(rate), "fw");
            continue;
        }
        let measured = rate_by_name(board, cm, ours);
        let result = match measured {
            Some(m) if (m - rate).abs() <= rate * 0.01 => "ok".to_string(),
            Some(m) if m > 0.0 && ((m / rate) - 2.0).abs() < 0.02 => "x2 (divided after the channel)".to_string(),
            Some(_) => {
                all_ok = false;
                "DIFFERENT".to_string()
            }
            None => "no rate".to_string(),
        };
        println!("{name:<7} {ours:<9} {:>12} {:>12}  {result}", mhz(rate), measured.map_or("?".into(), mhz));
    }
    Ok(all_ok)
}

/// Whether the HDMI blocks may be read: their state machine clock and PLLH
/// must run, since a read of a block without its clock can hang the bus.
fn hdmi_clocked(board: &Board, cm: &Window) -> Result<(), String> {
    let hsm = board.chip.clocks.iter().find(|c| c.name == "hsm").ok_or("no hsm clock in the table")?;
    if !chip::clock_ctl(cm.read(hsm.ctl)).enabled {
        return Err("hsm clock off".into());
    }
    let pllh = board.chip.plls.iter().find(|p| p.name == "pllh").ok_or("no pllh in the table")?;
    if chip::pll_powered_down(cm.read(pllh.ctrl)) {
        return Err("pllh powered down".into());
    }
    Ok(())
}

fn print_display(board: &Board, cm: &Window) -> Result<(), String> {
    use display::*;
    let hvs = board.map(HVS.bus, HVS.len)?;
    let ctrl = hvs.read(0x00);
    println!("HVS (DISPCTRL 0x{ctrl:08x}, enabled {}, DISPSTAT 0x{:08x})", u32::from(ctrl & (1 << 31) != 0), hvs.read(0x04));
    for ch in 0..3u32 {
        let base = 0x40 + 0x10 * ch;
        let (on, width, height) = hvs_channel(hvs.read(base));
        let (mode, full, empty, frame, line) = hvs_status(hvs.read(base + 8));
        let cob = hvs.read(base + 0xc);
        let list = hvs.read(0x20 + 4 * ch) & 0x3fff;
        println!(
            "  channel {ch}: {} {width}x{height}, background 0x{:08x}, mode {mode} full {} empty {} frame {frame} line {line}, cob {}..={}, list at word {list}",
            if on { "on" } else { "off" }, hvs.read(base + 4), u32::from(full), u32::from(empty), cob & 0xffff, cob >> 16
        );
        if !on {
            continue;
        }
        if list >= HVS_DLIST_WORDS {
            println!("    list outside the display list memory");
            continue;
        }
        let read = |i: u32| if i < HVS_DLIST_WORDS { hvs.read(HVS_DLIST + 4 * i) } else { 1 << 31 };
        match walk_list(list, 16, read) {
            Ok(planes) if planes.is_empty() => println!("    no planes"),
            Ok(planes) => {
                for p in planes {
                    let raw: Vec<String> = (p.start..p.start + p.words).map(|i| format!("{:08x}", read(i))).collect();
                    println!(
                        "    plane at word {}: {} {}x{} at ({}, {}){}, {} words: {}",
                        p.start, hvs_format(p.format), p.width, p.height, p.x, p.y,
                        p.source.map_or(" unscaled".into(), |(w, h)| format!(" scaled from {w}x{h}")),
                        p.words, raw.join(" ")
                    );
                }
            }
            Err(err) => println!("    {err}"),
        }
    }
    println!();

    println!("Pixel valves");
    for (block, feeds, clock) in PVS {
        let pv = board.map(block.bus, block.len)?;
        let (control, vcontrol) = (pv.read(0x00), pv.read(0x04));
        print!("  {} ({feeds}): control 0x{control:08x} v_control 0x{vcontrol:08x}", block.name);
        if control & 1 == 0 {
            println!(", off");
            continue;
        }
        let t = pv_timing(pv.read(0x0c), pv.read(0x10), pv.read(0x14), pv.read(0x18));
        let pixel = rate_by_name(board, cm, clock).unwrap_or(0.0);
        println!(
            ", video {}, {}; pixel clock {clock} {} MHz, {:.3} Hz; stat 0x{:08x} intstat 0x{:08x} hact_act {}",
            u32::from(vcontrol & 1 != 0), t.describe(), mhz(pixel), t.refresh(pixel), pv.read(0x2c), pv.read(0x28), pv.read(0x30)
        );
    }
    println!();

    if let Err(why) = hdmi_clocked(board, cm) {
        println!("HDMI: not read ({why})\n");
        return Ok(());
    }
    let (core, hd) = (board.map(HDMI.bus, HDMI.len)?, board.map(HD.bus, HD.len)?);
    let horza = core.read(0xc4);
    let t = hdmi_timing(horza, core.read(0xc8), core.read(0xcc), core.read(0xd0));
    let (hpos, vpos) = hdmi_polarity(horza);
    let pixel = rate_by_name(board, cm, "pllh_pix").unwrap_or(0.0);
    let sched = core.read(0xc0);
    let vid = hd.read(0x38);
    println!("HDMI (core rev 0x{:08x})", core.read(0x00));
    println!("  {}, hsync {}, vsync {}; pixel clock pllh_pix {} MHz, {:.3} Hz", t.describe(),
        if hpos { "+" } else { "-" }, if vpos { "+" } else { "-" }, mhz(pixel), t.refresh(pixel));
    println!(
        "  scheduler 0x{sched:08x} (hdmi mode {}, active {}), hotplug 0x{:08x}, fifo_ctl 0x{:08x}, misc 0x{:08x}",
        u32::from(sched & 1 != 0), u32::from(sched & 2 != 0), core.read(0x0c), core.read(0x5c), core.read(0xe4)
    );
    println!("  phy: reset_ctl 0x{:08x}, ctl_0 0x{:08x}", core.read(0x2c0), core.read(0x2c4));
    println!(
        "  hd: m_ctl 0x{:08x}, vid_ctl 0x{vid:08x} (enable {}), csc_ctl 0x{:08x}, frame count {}",
        hd.read(0x0c), u32::from(vid & (1 << 31) != 0), hd.read(0x40), hd.read(0x68)
    );
    println!();
    Ok(())
}

fn print_block(board: &Board, block: &display::Block) -> Result<(), String> {
    let w = board.map(block.bus, block.len)?;
    for &(offset, name) in block.regs {
        println!("{} 0x{:08x} 0x{:08x} {name}", block.name, block.bus + offset, w.read(offset));
    }
    Ok(())
}

fn print_display_regs(board: &Board, cm: &Window) -> Result<(), String> {
    print_block(board, &display::HVS)?;
    for (block, _, _) in display::PVS {
        print_block(board, block)?;
    }
    match hdmi_clocked(board, cm) {
        Ok(()) => {
            print_block(board, &display::HDMI)?;
            print_block(board, &display::HD)?;
        }
        Err(why) => println!("# hdmi and hd not read: {why}"),
    }
    Ok(())
}

/// `block address value name` lines of a `regs` dump, by block and address.
fn read_regs(path: &str) -> Result<Vec<(String, String, String)>, String> {
    let text = std::fs::read_to_string(path).map_err(|err| format!("{path}: {err}"))?;
    Ok(text
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| {
            let mut w = line.split_whitespace();
            let (block, addr, value) = (w.next()?, w.next()?, w.next()?);
            let name = w.next().unwrap_or("");
            Some((format!("{block} {addr}"), value.to_string(), name.to_string()))
        })
        .collect())
}

/// Registers that differ between two `regs` dumps, and those in only one.
fn diff(a: &str, b: &str) -> Result<bool, String> {
    let (left, right) = (read_regs(a)?, read_regs(b)?);
    let right_map: HashMap<&str, (&str, &str)> = right.iter().map(|(k, v, n)| (k.as_str(), (v.as_str(), n.as_str()))).collect();
    let left_keys: std::collections::HashSet<&str> = left.iter().map(|(k, _, _)| k.as_str()).collect();
    let mut same = true;
    println!("{:<20} {:<10} {:<10} name", "block address", "A", "B");
    for (key, value, name) in &left {
        match right_map.get(key.as_str()) {
            Some((other, _)) if other == value => {}
            Some((other, _)) => {
                same = false;
                println!("{key:<20} {value:<10} {other:<10} {name}");
            }
            None => {
                same = false;
                println!("{key:<20} {value:<10} {:<10} {name}", "-");
            }
        }
    }
    for (key, value, name) in &right {
        if !left_keys.contains(key.as_str()) {
            same = false;
            println!("{key:<20} {:<10} {value:<10} {name}", "-");
        }
    }
    if same {
        println!("(no differences)");
    }
    Ok(same)
}

fn print_regs(board: &Board, cm: &Window, gpio: &Window) {
    let line = |w: &Window, block: &str, offset: u32, name: &str| {
        println!("{block} 0x{:08x} 0x{:08x} {name}", w.bus + offset, w.read(offset));
    };
    line(cm, "cm", CM_LOCK, "LOCK");
    for pll in board.chip.plls {
        let up = pll.name.to_uppercase();
        line(cm, "cm", pll.cm, &format!("CM_{up}"));
        line(cm, "a2w", pll.ctrl, &format!("{up}_CTRL"));
        line(cm, "a2w", pll.frac, &format!("{up}_FRAC"));
        for i in 0..4 {
            line(cm, "a2w", pll.ana + 4 * i, &format!("{up}_ANA{i}"));
        }
        for channel in pll.channels {
            line(cm, "a2w", channel.reg, &channel.name.to_uppercase());
        }
    }
    for clock in board.chip.clocks {
        let up = clock.name.to_uppercase();
        line(cm, "cm", clock.ctl, &format!("{up}CTL"));
        line(cm, "cm", clock.ctl + 4, &format!("{up}DIV"));
    }
    for i in 0..board.chip.gpio_pins.div_ceil(10) {
        line(gpio, "gpio", 4 * i, &format!("GPFSEL{i}"));
    }
    for i in 0..board.chip.gpio_pins.div_ceil(32) {
        line(gpio, "gpio", 0x34 + 4 * i, &format!("GPLEV{i}"));
    }
}

fn run(what: &str) -> Result<bool, String> {
    let board = board()?;
    let cm = board.map(CM_BASE, CM_LEN)?;
    if what != "regs" {
        println!(
            "socdump: {}, peripherals at 0x{:x}, crystal {} MHz ({})\n",
            board.chip.name,
            board.phys(0x7e00_0000)?,
            mhz(f64::from(board.xosc)),
            if board.xosc_from_dt { "devicetree" } else { "default for the chip" }
        );
    }
    match what {
        "plls" => print_plls(&board, &cm),
        "clocks" => print_clocks(&board, &cm),
        "gpio" => print_gpio(&board, &board.map(GPIO_BASE, GPIO_LEN)?),
        "all" => {
            print_plls(&board, &cm);
            print_clocks(&board, &cm);
            print_gpio(&board, &board.map(GPIO_BASE, GPIO_LEN)?);
            print_display(&board, &cm)?;
        }
        "check" => return check(&board, &cm),
        "display" => print_display(&board, &cm)?,
        "regs" => {
            print_regs(&board, &cm, &board.map(GPIO_BASE, GPIO_LEN)?);
            print_display_regs(&board, &cm)?;
        }
        _ => return Err(USAGE.into()),
    }
    Ok(true)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let [cmd, a, b] = args.as_slice() {
        if cmd == "diff" {
            return match diff(a, b) {
                Ok(true) => ExitCode::SUCCESS,
                Ok(false) => ExitCode::FAILURE,
                Err(err) => {
                    eprintln!("socdump: {err}");
                    ExitCode::from(2)
                }
            };
        }
    }
    let what = match args.as_slice() {
        [] => "all",
        [one] if one != "-h" && one != "--help" => one.as_str(),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match run(what) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(err) => {
            eprintln!("socdump: {err}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_of_the_3bp() {
        // ranges = <0x7e000000 0x3f000000 0x01000000 0x40000000 0x40000000 0x00001000>
        let words: [u32; 6] = [0x7e00_0000, 0x3f00_0000, 0x0100_0000, 0x4000_0000, 0x4000_0000, 0x1000];
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_be_bytes()).collect();
        let ranges = parse_ranges(&bytes, 1, 1, 1);
        assert_eq!(ranges[0], (0x7e00_0000, 0x3f00_0000, 0x0100_0000));
        assert_eq!(ranges.len(), 2);
    }

    #[test]
    fn ranges_of_the_pi4() {
        // child 1 cell, parent 2 cells, size 1: 0x7e000000 -> 0x0_fe000000
        let words: [u32; 4] = [0x7e00_0000, 0x0, 0xfe00_0000, 0x0180_0000];
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_be_bytes()).collect();
        assert_eq!(parse_ranges(&bytes, 1, 2, 1), vec![(0x7e00_0000, 0xfe00_0000, 0x0180_0000)]);
    }
}
