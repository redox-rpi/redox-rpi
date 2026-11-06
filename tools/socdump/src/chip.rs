//! Register tables, one per SoC. Addresses are VideoCore bus addresses
//! (`0x7e......`); `main.rs` turns them into CPU physical addresses with
//! the devicetree's `/soc` ranges, so the same table works whatever the
//! peripheral window is.
//!
//! Offsets and field layouts are from the BCM2835 peripherals manual and
//! the Linux clock driver (`drivers/clk/bcm/clk-bcm2835.c`), which reads
//! the same registers.

/// Clock manager (CPRMAN); the PLL (A2W) registers are at +0x1000.
pub const CM_BASE: u32 = 0x7e10_1000;
/// Length mapped at `CM_BASE`: the clock manager and the A2W block.
pub const CM_LEN: usize = 0x2000;
pub const GPIO_BASE: u32 = 0x7e20_0000;
pub const GPIO_LEN: usize = 0x1000;

pub const CM_LOCK: u32 = 0x114;

pub struct Chip {
    pub name: &'static str,
    /// Root `compatible` strings of the devicetree.
    pub compatible: &'static [&'static str],
    /// Processor field (bits 12-15) of a new-style board revision code.
    pub revision_processor: u32,
    /// Crystal, when the devicetree does not say.
    pub xosc_hz: u32,
    /// The ANA1 "feedback prediv" bit doubles the PLL multiplier.
    pub pll_prediv: bool,
    pub plls: &'static [Pll],
    pub clocks: &'static [Clock],
    pub gpio_pins: u32,
}

pub struct Pll {
    pub name: &'static str,
    /// A2W_PLLx_CTRL, A2W_PLLx_FRAC and A2W_PLLx_ANA0, from `CM_BASE`.
    pub ctrl: u32,
    pub frac: u32,
    pub ana: u32,
    /// Bit in ANA1 that turns the feedback prediv on.
    pub prediv_bit: u32,
    /// CM_LOCK bit set while the PLL is locked.
    pub lock_bit: u32,
    /// CM_PLLx (load/hold bits of the channels).
    pub cm: u32,
    pub channels: &'static [Channel],
}

pub struct Channel {
    pub name: &'static str,
    /// A2W_PLLx_<channel>, from `CM_BASE`.
    pub reg: u32,
    /// Fixed divider after the channel's own one (10 for PLLH PIX/RCAL).
    pub fixed_div: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Parents {
    /// gnd, xosc, testdebug0/1, plla_per, pllc_per, plld_per, pllh_aux
    Per,
    /// gnd, xosc, testdebug0/1, plla_core, pllc_core0, plld_core,
    /// pllh_aux, pllc_core1, pllc_core2
    Vpu,
    /// The crystal only.
    Osc,
    /// DSI byte clocks from the DSI PHY: no divider we can decode.
    Dsi,
}

pub struct Clock {
    pub name: &'static str,
    /// CM_<name>CTL; CM_<name>DIV follows at +4.
    pub ctl: u32,
    pub parents: Parents,
    /// Used bits of the 12.12 fixed-point divider.
    pub int_bits: u32,
    pub frac_bits: u32,
}

const PER_PARENTS: &[&str] = &[
    "gnd", "xosc", "testdebug0", "testdebug1", "plla_per", "pllc_per", "plld_per", "pllh_aux",
];
const VPU_PARENTS: &[&str] = &[
    "gnd", "xosc", "testdebug0", "testdebug1", "plla_core", "pllc_core0", "plld_core", "pllh_aux",
    "pllc_core1", "pllc_core2",
];

impl Parents {
    pub fn name(self, src: u32) -> &'static str {
        let list = match self {
            Parents::Per => PER_PARENTS,
            Parents::Vpu => VPU_PARENTS,
            Parents::Osc => &PER_PARENTS[..2],
            Parents::Dsi => return "dsi-phy",
        };
        list.get(src as usize).copied().unwrap_or("?")
    }
}

const fn ch(name: &'static str, reg: u32) -> Channel {
    Channel { name, reg, fixed_div: 1 }
}

const fn clk(name: &'static str, ctl: u32, parents: Parents, int_bits: u32, frac_bits: u32) -> Clock {
    Clock { name, ctl, parents, int_bits, frac_bits }
}

use Parents::*;

/// PLLs of the BCM2835/6/7 (the BCM2711 has no PLLH and no prediv).
const BCM2837_PLLS: &[Pll] = &[
    Pll {
        name: "plla", ctrl: 0x1100, frac: 0x1200, ana: 0x1010, prediv_bit: 14, lock_bit: 8, cm: 0x104,
        channels: &[ch("plla_core", 0x1400), ch("plla_per", 0x1500), ch("plla_dsi0", 0x1300), ch("plla_ccp2", 0x1600)],
    },
    Pll {
        name: "pllb", ctrl: 0x11e0, frac: 0x12e0, ana: 0x10f0, prediv_bit: 14, lock_bit: 9, cm: 0x170,
        channels: &[ch("pllb_arm", 0x13e0), ch("pllb_sp0", 0x14e0), ch("pllb_sp1", 0x15e0), ch("pllb_sp2", 0x16e0)],
    },
    Pll {
        name: "pllc", ctrl: 0x1120, frac: 0x1220, ana: 0x1030, prediv_bit: 14, lock_bit: 10, cm: 0x108,
        channels: &[ch("pllc_core0", 0x1620), ch("pllc_core1", 0x1420), ch("pllc_core2", 0x1320), ch("pllc_per", 0x1520)],
    },
    Pll {
        name: "plld", ctrl: 0x1140, frac: 0x1240, ana: 0x1050, prediv_bit: 14, lock_bit: 11, cm: 0x10c,
        channels: &[ch("plld_core", 0x1440), ch("plld_per", 0x1540), ch("plld_dsi0", 0x1340), ch("plld_dsi1", 0x1640)],
    },
    Pll {
        name: "pllh", ctrl: 0x1160, frac: 0x1260, ana: 0x1070, prediv_bit: 11, lock_bit: 12, cm: 0x110,
        channels: &[
            ch("pllh_aux", 0x1360),
            Channel { name: "pllh_pix", reg: 0x1560, fixed_div: 10 },
            Channel { name: "pllh_rcal", reg: 0x1460, fixed_div: 10 },
        ],
    },
];

const BCM2837_CLOCKS: &[Clock] = &[
    clk("vpu", 0x008, Vpu, 12, 8),
    clk("h264", 0x028, Vpu, 4, 8),
    clk("isp", 0x030, Vpu, 4, 8),
    clk("v3d", 0x038, Vpu, 4, 8),
    clk("sdram", 0x1a8, Vpu, 6, 0),
    clk("emmc", 0x1c0, Per, 4, 8),
    clk("uart", 0x0f0, Per, 10, 12),
    clk("pwm", 0x0a0, Per, 12, 12),
    clk("pcm", 0x098, Per, 12, 12),
    clk("gp0", 0x070, Per, 12, 12),
    clk("gp1", 0x078, Per, 12, 12),
    clk("gp2", 0x080, Per, 12, 12),
    clk("hsm", 0x088, Per, 4, 8),
    clk("dpi", 0x068, Per, 4, 8),
    clk("vec", 0x0f8, Per, 4, 0),
    clk("smi", 0x0b0, Per, 4, 8),
    clk("slim", 0x0a8, Per, 12, 12),
    clk("cam0", 0x040, Per, 4, 8),
    clk("cam1", 0x048, Per, 4, 8),
    clk("dsi0e", 0x058, Per, 4, 8),
    clk("dsi1e", 0x158, Per, 4, 8),
    clk("dsi0p", 0x060, Dsi, 0, 0),
    clk("dsi1p", 0x160, Dsi, 0, 0),
    clk("aveo", 0x1b8, Per, 4, 0),
    clk("dft", 0x168, Per, 5, 0),
    clk("timer", 0x0e8, Osc, 6, 12),
    clk("tsens", 0x0e0, Osc, 5, 0),
    clk("tec", 0x0c8, Osc, 6, 0),
    clk("otp", 0x090, Osc, 4, 0),
];

pub const CHIPS: &[Chip] = &[Chip {
    name: "BCM2837",
    compatible: &["brcm,bcm2837"],
    revision_processor: 2,
    xosc_hz: 19_200_000,
    pll_prediv: true,
    plls: BCM2837_PLLS,
    clocks: BCM2837_CLOCKS,
    gpio_pins: 54,
}];

// --- decoding, kept free of register access so it can be tested on a host

/// PLL output in Hz: crystal * (ndiv + frac / 2^20) / pdiv.
pub fn pll_rate(xosc: u32, ctrl: u32, frac: u32, prediv: bool) -> f64 {
    let (mut ndiv, mut fdiv) = (ctrl & 0x3ff, frac & 0xf_ffff);
    let pdiv = (ctrl >> 12) & 7;
    if pdiv == 0 {
        return 0.0;
    }
    if prediv {
        ndiv *= 2;
        fdiv *= 2;
    }
    f64::from(xosc) * (f64::from(ndiv) + f64::from(fdiv) / f64::from(1u32 << 20)) / f64::from(pdiv)
}

pub fn pll_powered_down(ctrl: u32) -> bool {
    ctrl & (1 << 16) != 0
}

/// A PLL channel's divider (0 means 256) and whether the channel is off.
pub fn channel_div(reg: u32) -> (u32, bool) {
    let div = reg & 0xff;
    (if div == 0 { 256 } else { div }, reg & (1 << 8) != 0)
}

/// Divider of a clock as a number, from CM_xxxDIV (12.12 fixed point,
/// only `int_bits`.`frac_bits` of it used). `None` when the clock has no
/// divider; 0.0 when the divider is 0 (clock stopped).
pub fn clock_div(clock: &Clock, div: u32) -> Option<f64> {
    if clock.int_bits == 0 && clock.frac_bits == 0 {
        return None;
    }
    let field = (div >> (12 - clock.frac_bits)) & ((1 << (clock.int_bits + clock.frac_bits)) - 1);
    Some(f64::from(field) / f64::from(1u32 << clock.frac_bits))
}

pub struct ClockCtl {
    pub src: u32,
    pub enabled: bool,
    pub busy: bool,
    pub mash: u32,
}

pub fn clock_ctl(ctl: u32) -> ClockCtl {
    ClockCtl { src: ctl & 0xf, enabled: ctl & (1 << 4) != 0, busy: ctl & (1 << 7) != 0, mash: (ctl >> 9) & 3 }
}

/// Function of a GPIO pin from its 3 GPFSEL bits.
pub fn gpio_function(bits: u32) -> &'static str {
    ["in", "out", "alt5", "alt4", "alt0", "alt1", "alt2", "alt3"][(bits & 7) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pllc_at_1200_mhz() {
        // 19.2 MHz * 2 * (31 + 0.25) / 1 = 1200 MHz
        let rate = pll_rate(19_200_000, (1 << 12) | 31, 1 << 18, true);
        assert!((rate - 1_200_000_000.0).abs() < 1.0, "{rate}");
    }

    #[test]
    fn pll_without_pdiv_is_stopped() {
        assert_eq!(pll_rate(19_200_000, 52, 0, false), 0.0);
    }

    #[test]
    fn channel_divider_zero_is_256() {
        assert_eq!(channel_div(0x100), (256, true));
        assert_eq!(channel_div(0x002), (2, false));
    }

    #[test]
    fn uart_divider() {
        let uart = BCM2837_CLOCKS.iter().find(|c| c.name == "uart").unwrap();
        // 2.5 in 12.12 fixed point with the password byte set
        assert_eq!(clock_div(uart, 0x5a00_2800), Some(2.5));
    }

    #[test]
    fn vpu_divider_drops_low_frac_bits() {
        let vpu = BCM2837_CLOCKS.iter().find(|c| c.name == "vpu").unwrap();
        // 3.0 plus a fraction below 1/256, which the VPU divider ignores
        assert_eq!(clock_div(vpu, 0x3000 | 0x00f), Some(3.0));
    }

    #[test]
    fn parents() {
        assert_eq!(Parents::Vpu.name(5), "pllc_core0");
        assert_eq!(Parents::Per.name(6), "plld_per");
        assert_eq!(Parents::Osc.name(4), "?");
    }
}
