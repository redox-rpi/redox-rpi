//! HDMI of the BCM2711 (Pi 4, Pi 400, CM4; "vc5" in Linux): the DVP
//! clock and reset block and the two HDMI ports, each with its core,
//! HD (audio FIFO, video control), PHY and rate manager registers. Read
//! only, for the HDMI audio work (#543): which port the firmware drives,
//! its mode and pixel clock, and the audio registers before we touch them.
//!
//! Offsets from Linux `vc5_hdmi_hdmi0_fields`/`vc5_hdmi_hdmi1_fields`
//! (`drivers/gpu/drm/vc4/vc4_hdmi_regs.h`), field layouts from
//! `vc4_hdmi.c` and `vc4_hdmi_phy.c`; the decoding is our own. `MAI_DATA`
//! (the audio FIFO) and the CEC data registers are left out, since reading
//! them changes something.

use crate::display::{bits, Block, Timing};

/// DVP block: per-port reset (`SW_INIT`) and 108 MHz clock gate
/// (`MISC_CONFIG`, a set bit stops the clock). Always clocked.
pub const DVP: Block = Block {
    name: "dvp",
    bus: 0x7ef0_0000,
    len: 0x10,
    regs: &[(0x04, "HT_RPI_SW_INIT"), (0x08, "HT_RPI_MISC_CONFIG")],
};
pub const DVP_SW_INIT: u32 = 0x04;
pub const DVP_MISC_CONFIG: u32 = 0x08;

/// The audio clock of both ports: the DVP gate on the fixed 108 MHz clock.
pub const AUDIO_CLOCK_HZ: u32 = 108_000_000;
/// The PHY PLL's reference.
pub const PHY_REF_HZ: f64 = 54_000_000.0;

const CORE_REGS: &[(u32, &str)] = &[
    (0x074, "FIFO_CTL"),
    (0x09c, "MAI_CHANNEL_MAP"),
    (0x0a0, "MAI_CONFIG"),
    (0x0b8, "AUDIO_PACKET_CONFIG"),
    (0x0bc, "RAM_PACKET_CONFIG"),
    (0x0c4, "RAM_PACKET_STATUS"),
    (0x0c8, "CRP_CFG"),
    (0x0cc, "CTS_0"),
    (0x0d0, "CTS_1"),
    (0x0e0, "SCHEDULER_CONTROL"),
    (0x0e4, "HORZA"),
    (0x0e8, "HORZB"),
    (0x0ec, "VERTA0"),
    (0x0f0, "VERTB0"),
    (0x0f4, "VERTA1"),
    (0x0f8, "VERTB1"),
    (0x100, "MISC_CONTROL"),
    (0x170, "DEEP_COLOR_CONFIG_1"),
    (0x178, "GCP_CONFIG"),
    (0x1a8, "HOTPLUG"),
    (0x1c4, "SCRAMBLER_CTL"),
];

const PHY_REGS: &[(u32, &str)] = &[
    (0x00, "TX_PHY_RESET_CTL"),
    (0x04, "TX_PHY_POWERDOWN_CTL"),
    (0x08, "TX_PHY_CTL_0"),
    (0x0c, "TX_PHY_CTL_1"),
    (0x10, "TX_PHY_CTL_2"),
    (0x14, "TX_PHY_CTL_3"),
    (0x1c, "TX_PHY_PLL_CTL_0"),
    (0x20, "TX_PHY_PLL_CTL_1"),
    (0x28, "TX_PHY_CLK_DIV"),
    (0x34, "TX_PHY_PLL_CFG"),
    (0x44, "TX_PHY_TMDS_CLK_WORD_SEL"),
    (0x4c, "TX_PHY_CHANNEL_SWAP"),
];

const RM_REGS: &[(u32, &str)] = &[(0x00, "RM_CONTROL"), (0x18, "RM_OFFSET"), (0x1c, "RM_FORMAT")];

/// Offsets in the shared HD block (`0x7ef20000`) of one port.
pub struct HdRegs {
    pub dvp_ctl: u32,
    pub mai_ctl: u32,
    pub mai_thr: u32,
    pub mai_fmt: u32,
    pub mai_smp: u32,
    pub vid_ctl: u32,
    pub frame_count: u32,
}

pub struct Port {
    pub name: &'static str,
    /// Bit in DVP `SW_INIT` (set: held in reset) and in `MISC_CONFIG`
    /// (set: 108 MHz clock stopped).
    pub reset_bit: u32,
    pub gate_bit: u32,
    /// DREQ of its audio FIFO (devicetree `dmas`).
    pub dreq: u32,
    pub core: Block,
    pub hd: Block,
    pub hd_regs: HdRegs,
    pub phy: Block,
    pub rm: Block,
}

pub const HD_BUS: u32 = 0x7ef2_0000;

pub const PORTS: [Port; 2] = [
    Port {
        name: "hdmi0",
        reset_bit: 0,
        gate_bit: 3,
        dreq: 10,
        core: Block { name: "hdmi0", bus: 0x7ef0_0700, len: 0x300, regs: CORE_REGS },
        hd: Block {
            name: "hd0",
            bus: HD_BUS,
            len: 0x100,
            regs: &[(0x00, "DVP_CTL"), (0x10, "MAI_CTL"), (0x14, "MAI_THR"), (0x18, "MAI_FMT"), (0x20, "MAI_SMP"), (0x44, "VID_CTL"), (0x60, "FRAME_COUNT")],
        },
        hd_regs: HdRegs { dvp_ctl: 0x00, mai_ctl: 0x10, mai_thr: 0x14, mai_fmt: 0x18, mai_smp: 0x20, vid_ctl: 0x44, frame_count: 0x60 },
        phy: Block { name: "phy0", bus: 0x7ef0_0f00, len: 0x80, regs: PHY_REGS },
        rm: Block { name: "rm0", bus: 0x7ef0_0f80, len: 0x80, regs: RM_REGS },
    },
    Port {
        name: "hdmi1",
        reset_bit: 1,
        gate_bit: 4,
        dreq: 17,
        core: Block { name: "hdmi1", bus: 0x7ef0_5700, len: 0x300, regs: CORE_REGS },
        hd: Block {
            name: "hd1",
            bus: HD_BUS,
            len: 0x100,
            regs: &[(0x00, "DVP_CTL"), (0x30, "MAI_CTL"), (0x34, "MAI_THR"), (0x38, "MAI_FMT"), (0x40, "MAI_SMP"), (0x48, "VID_CTL"), (0x64, "FRAME_COUNT")],
        },
        hd_regs: HdRegs { dvp_ctl: 0x00, mai_ctl: 0x30, mai_thr: 0x34, mai_fmt: 0x38, mai_smp: 0x40, vid_ctl: 0x48, frame_count: 0x64 },
        phy: Block { name: "phy1", bus: 0x7ef0_5f00, len: 0x80, regs: PHY_REGS },
        rm: Block { name: "rm1", bus: 0x7ef0_5f80, len: 0x80, regs: RM_REGS },
    },
];

/// Whether a port's registers may be read: out of reset and its 108 MHz
/// clock running. Reading a block without its clock can hang the bus.
pub fn port_clocked(port: &Port, sw_init: u32, misc_config: u32) -> Result<(), &'static str> {
    if sw_init & (1 << port.reset_bit) != 0 {
        return Err("held in reset (DVP SW_INIT)");
    }
    if misc_config & (1 << port.gate_bit) != 0 {
        return Err("108 MHz clock stopped (DVP MISC_CONFIG)");
    }
    Ok(())
}

/// Timing from HORZA, HORZB, VERTA0, VERTB0 (vc5 layout).
pub fn timing(horza: u32, horzb: u32, verta: u32, vertb: u32) -> Timing {
    Timing {
        hactive: bits(horza, 13, 0),
        hfp: bits(horza, 28, 16),
        hsync: bits(horzb, 10, 0),
        hbp: bits(horzb, 26, 16),
        vactive: bits(verta, 12, 0),
        vfp: bits(verta, 22, 16),
        vsync: bits(verta, 28, 24),
        vbp: bits(vertb, 8, 0),
    }
}

/// Sync polarity from HORZA bits 15 (v) and 14 (h): (hsync +, vsync +).
pub fn polarity(horza: u32) -> (bool, bool) {
    (horza & (1 << 14) != 0, horza & (1 << 15) != 0)
}

/// TMDS character rate as Linux would have programmed the PHY PLL:
/// `RM_OFFSET` = vco × 2^21 / 54 MHz, `CLK_DIV` bits 15:8 = vco_div,
/// tmds = vco / (vco_div × 10). None when the PLL is not set up.
pub fn tmds_from_phy(rm_offset: u32, clk_div: u32) -> Option<f64> {
    let offset = rm_offset & 0x7fff_ffff;
    let vco_div = bits(clk_div, 15, 8);
    if offset == 0 || vco_div == 0 {
        return None;
    }
    let vco = f64::from(offset) * PHY_REF_HZ / f64::from(1u32 << 21);
    Some(vco / (f64::from(vco_div) * 10.0))
}

/// N and CTS of the audio clock regeneration packet from `CRP_CFG`
/// (bits 19:0 N, bit 24 external CTS) and `CTS_0`.
pub fn n_cts(crp_cfg: u32, cts0: u32) -> (u32, bool, u32) {
    (bits(crp_cfg, 19, 0), crp_cfg & (1 << 24) != 0, bits(cts0, 19, 0))
}

/// `MAI_CTL`: (enabled, channels, FIFO full, FIFO empty, DLATE).
pub fn mai_ctl(ctl: u32) -> (bool, u32, bool, bool, bool) {
    (ctl & (1 << 3) != 0, bits(ctl, 7, 4), ctl & (1 << 11) != 0, ctl & (1 << 10) != 0, ctl & (1 << 15) != 0)
}

/// `MAI_SMP`: N (bits 31:8) and M - 1 (bits 7:0); sample rate = clock × M / N
/// as Linux sets it (rational_best_approximation(clock, rate)).
pub fn mai_rate(smp: u32, clock_hz: u32) -> Option<f64> {
    let (n, m) = (smp >> 8, (smp & 0xff) + 1);
    (n != 0).then(|| f64::from(clock_hz) * f64::from(m) / f64::from(n))
}

/// Pixel clock from two frame counter reads `seconds` apart and the
/// frame size (the counter is the HD block's `FRAME_COUNT`).
pub fn pixel_from_frames(count0: u32, count1: u32, seconds: f64, t: &Timing) -> f64 {
    let frames = f64::from(count1.wrapping_sub(count0));
    frames / seconds * f64::from(t.htotal()) * f64::from(t.vtotal())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timing_1080p() {
        // 1920x1080@60: h 88/44/148, v 4/5/36, sync + / +.
        let horza = (1 << 15) | (1 << 14) | (88 << 16) | 1920;
        let horzb = (148 << 16) | 44;
        let verta = (5 << 24) | (4 << 16) | 1080;
        let vertb = 36;
        let t = timing(horza, horzb, verta, vertb);
        assert_eq!((t.hactive, t.hfp, t.hsync, t.hbp), (1920, 88, 44, 148));
        assert_eq!((t.vactive, t.vfp, t.vsync, t.vbp), (1080, 4, 5, 36));
        assert_eq!((t.htotal(), t.vtotal()), (2200, 1125));
        assert_eq!(polarity(horza), (true, true));
    }

    #[test]
    fn tmds_148_5() {
        // Linux: vco_div 3 gives 148.5 MHz × 30 = 4455 MHz.
        let vco = 4_455_000_000u64;
        let offset = (((vco * 2) << 22) / 54_000_000) >> 2;
        let tmds = tmds_from_phy(offset as u32 | (1 << 31), 3 << 8).unwrap();
        assert!((tmds - 148_500_000.0).abs() < 1_000.0, "{tmds}");
        assert_eq!(tmds_from_phy(0, 3 << 8), None);
    }

    #[test]
    fn audio_fields() {
        // N 6272 for 44.1 kHz, CTS 148500 at 148.5 MHz, external CTS.
        assert_eq!(n_cts((1 << 24) | 6272, 148_500), (6272, true, 148_500));
        assert_eq!(mai_ctl((2 << 4) | (1 << 3) | (1 << 10)), (true, 2, false, true, false));
        // 108 MHz × 49 / 120000 = 44100 Hz.
        let rate = mai_rate((120_000 << 8) | 48, AUDIO_CLOCK_HZ).unwrap();
        assert!((rate - 44_100.0).abs() < 0.01, "{rate}");
    }

    #[test]
    fn clock_gate() {
        let p = &PORTS[0];
        assert!(port_clocked(p, 0, 0).is_ok());
        assert!(port_clocked(p, 1, 0).is_err());
        assert!(port_clocked(p, 0, 1 << 3).is_err());
        assert!(port_clocked(&PORTS[1], 0, 1 << 3).is_ok());
    }

    #[test]
    fn frames_to_pixel_clock() {
        let t = Timing { hactive: 1920, hfp: 88, hsync: 44, hbp: 148, vactive: 1080, vfp: 4, vsync: 5, vbp: 36 };
        let hz = pixel_from_frames(100, 160, 1.0, &t);
        assert!((hz - 148_500_000.0).abs() < 1.0);
    }
}
