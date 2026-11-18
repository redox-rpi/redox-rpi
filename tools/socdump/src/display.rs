//! Display blocks of the BCM2835/6/7: the HVS (scaler), the three pixel
//! valves and the HDMI encoder (core and HD blocks). Read only, for
//! reference values (#224 step 3): what the closed firmware programmed for
//! the mode it picked.
//!
//! Offsets from the BCM2835 register list in Linux
//! `drivers/gpu/drm/vc4/vc4_regs.h` and `vc4_hdmi_regs.h`; the decoding is
//! our own. Registers whose read has a side effect (the audio FIFO
//! `MAI_DATA`, the CEC data registers) are left out.

/// A register block: bus address, length to map, named registers.
pub struct Block {
    pub name: &'static str,
    pub bus: u32,
    pub len: usize,
    pub regs: &'static [(u32, &'static str)],
}

pub const HVS: Block = Block {
    name: "hvs",
    bus: 0x7e40_0000,
    len: 0x6000,
    regs: &[
        (0x00, "DISPCTRL"),
        (0x04, "DISPSTAT"),
        (0x08, "DISPID"),
        (0x0c, "DISPECTRL"),
        (0x10, "DISPPROF"),
        (0x14, "DISPDITHER"),
        (0x18, "DISPEOLN"),
        (0x20, "DISPLIST0"),
        (0x24, "DISPLIST1"),
        (0x28, "DISPLIST2"),
        (0x2c, "DISPLSTAT"),
        (0x30, "DISPLACT0"),
        (0x34, "DISPLACT1"),
        (0x38, "DISPLACT2"),
        (0x40, "DISPCTRL0"),
        (0x44, "DISPBKGND0"),
        (0x48, "DISPSTAT0"),
        (0x4c, "DISPBASE0"),
        (0x50, "DISPCTRL1"),
        (0x54, "DISPBKGND1"),
        (0x58, "DISPSTAT1"),
        (0x5c, "DISPBASE1"),
        (0x60, "DISPCTRL2"),
        (0x64, "DISPBKGND2"),
        (0x68, "DISPSTAT2"),
        (0x6c, "DISPBASE2"),
    ],
};

/// Display list memory inside the HVS block, in words from here.
pub const HVS_DLIST: u32 = 0x2000;
pub const HVS_DLIST_WORDS: u32 = 0x1000;

const PV_REGS: &[(u32, &str)] = &[
    (0x00, "CONTROL"),
    (0x04, "V_CONTROL"),
    (0x08, "VSYNCD_EVEN"),
    (0x0c, "HORZA"),
    (0x10, "HORZB"),
    (0x14, "VERTA"),
    (0x18, "VERTB"),
    (0x1c, "VERTA_EVEN"),
    (0x20, "VERTB_EVEN"),
    (0x24, "INTEN"),
    (0x28, "INTSTAT"),
    (0x2c, "STAT"),
    (0x30, "HACT_ACT"),
];

/// Pixel valves and what feeds them on the BCM2837.
pub const PVS: &[(Block, &str, &str)] = &[
    (Block { name: "pv0", bus: 0x7e20_6000, len: 0x100, regs: PV_REGS }, "DSI0 or DPI", "dpi"),
    (Block { name: "pv1", bus: 0x7e20_7000, len: 0x100, regs: PV_REGS }, "DSI1", "dsi1p"),
    (Block { name: "pv2", bus: 0x7e80_7000, len: 0x100, regs: PV_REGS }, "HDMI", "pllh_pix"),
];

pub const HDMI: Block = Block {
    name: "hdmi",
    bus: 0x7e90_2000,
    len: 0x1000,
    regs: &[
        (0x000, "CORE_REV"),
        (0x004, "SW_RESET_CONTROL"),
        (0x008, "HOTPLUG_INT"),
        (0x00c, "HOTPLUG"),
        (0x05c, "FIFO_CTL"),
        (0x090, "MAI_CHANNEL_MAP"),
        (0x094, "MAI_CONFIG"),
        (0x098, "MAI_FORMAT"),
        (0x09c, "AUDIO_PACKET_CONFIG"),
        (0x0a0, "RAM_PACKET_CONFIG"),
        (0x0a4, "RAM_PACKET_STATUS"),
        (0x0a8, "CRP_CFG"),
        (0x0ac, "CTS_0"),
        (0x0b0, "CTS_1"),
        (0x0c0, "SCHEDULER_CONTROL"),
        (0x0c4, "HORZA"),
        (0x0c8, "HORZB"),
        (0x0cc, "VERTA0"),
        (0x0d0, "VERTB0"),
        (0x0d4, "VERTA1"),
        (0x0d8, "VERTB1"),
        (0x0e4, "MISC_CONTROL"),
        (0x2c0, "TX_PHY_RESET_CTL"),
        (0x2c4, "TX_PHY_CTL_0"),
    ],
};

pub const HD: Block = Block {
    name: "hd",
    bus: 0x7e80_8000,
    len: 0x100,
    regs: &[
        (0x0c, "M_CTL"),
        (0x14, "MAI_CTL"),
        (0x18, "MAI_THR"),
        (0x1c, "MAI_FMT"),
        (0x2c, "MAI_SMP"),
        (0x38, "VID_CTL"),
        (0x40, "CSC_CTL"),
        (0x44, "CSC_12_11"),
        (0x48, "CSC_14_13"),
        (0x4c, "CSC_22_21"),
        (0x50, "CSC_24_23"),
        (0x54, "CSC_32_31"),
        (0x58, "CSC_34_33"),
        (0x68, "FRAME_COUNT"),
    ],
};

// --- decoding, free of register access so it can be tested on a host

pub fn bits(reg: u32, high: u32, low: u32) -> u32 {
    (reg >> low) & (u32::MAX >> (31 - (high - low)))
}

/// A video mode's timing: active, front porch, sync and back porch.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Timing {
    pub hactive: u32,
    pub hfp: u32,
    pub hsync: u32,
    pub hbp: u32,
    pub vactive: u32,
    pub vfp: u32,
    pub vsync: u32,
    pub vbp: u32,
}

impl Timing {
    pub fn htotal(&self) -> u32 {
        self.hactive + self.hfp + self.hsync + self.hbp
    }

    pub fn vtotal(&self) -> u32 {
        self.vactive + self.vfp + self.vsync + self.vbp
    }

    /// Frames per second at this pixel clock.
    pub fn refresh(&self, pixel_hz: f64) -> f64 {
        let total = f64::from(self.htotal()) * f64::from(self.vtotal());
        if total == 0.0 { 0.0 } else { pixel_hz / total }
    }

    pub fn describe(&self) -> String {
        format!(
            "{}x{}, h {} fp {} sync {} bp {} (total {}), v {} fp {} sync {} bp {} (total {})",
            self.hactive, self.vactive, self.hactive, self.hfp, self.hsync, self.hbp, self.htotal(),
            self.vactive, self.vfp, self.vsync, self.vbp, self.vtotal()
        )
    }
}

/// Pixel valve timing from HORZA, HORZB, VERTA, VERTB: each register holds
/// two 16-bit fields (back porch or front porch high, sync or active low).
pub fn pv_timing(horza: u32, horzb: u32, verta: u32, vertb: u32) -> Timing {
    Timing {
        hactive: horzb & 0xffff,
        hfp: horzb >> 16,
        hsync: horza & 0xffff,
        hbp: horza >> 16,
        vactive: vertb & 0xffff,
        vfp: vertb >> 16,
        vsync: verta & 0xffff,
        vbp: verta >> 16,
    }
}

/// HDMI core timing from HORZA, HORZB, VERTA0, VERTB0.
pub fn hdmi_timing(horza: u32, horzb: u32, verta: u32, vertb: u32) -> Timing {
    Timing {
        hactive: bits(horza, 12, 0),
        hfp: bits(horzb, 9, 0),
        hsync: bits(horzb, 19, 10),
        hbp: bits(horzb, 29, 20),
        vactive: bits(verta, 12, 0),
        vfp: bits(verta, 19, 13),
        vsync: bits(verta, 24, 20),
        vbp: bits(vertb, 8, 0),
    }
}

/// HDMI sync polarity from HORZA bit 14 and 13 (1 = positive).
pub fn hdmi_polarity(horza: u32) -> (bool, bool) {
    (horza & (1 << 14) != 0, horza & (1 << 13) != 0)
}

/// An HVS channel's DISPCTRLx: enabled, width, height.
pub fn hvs_channel(ctrl: u32) -> (bool, u32, u32) {
    (ctrl & (1 << 31) != 0, bits(ctrl, 23, 12), bits(ctrl, 11, 0))
}

/// DISPSTATx: mode (0 idle .. 2 run), FIFO full, FIFO empty, frame count, line.
pub fn hvs_status(stat: u32) -> (u32, bool, bool, u32, u32) {
    (bits(stat, 31, 30), stat & (1 << 29) != 0, stat & (1 << 28) != 0, bits(stat, 17, 12), bits(stat, 11, 0))
}

/// One plane of an HVS display list.
#[derive(Debug, PartialEq, Eq)]
pub struct Plane {
    /// Index of its first word in the display list memory.
    pub start: u32,
    pub words: u32,
    pub format: u32,
    pub unity: bool,
    pub x: u32,
    pub y: u32,
    /// Size on screen.
    pub width: u32,
    pub height: u32,
    /// Size of the source when scaled.
    pub source: Option<(u32, u32)>,
}

/// Walks a display list from `start` with `read(word index)`, up to the
/// END word or `max` planes. Errors on an entry that cannot be a plane.
pub fn walk_list(start: u32, max: usize, read: impl Fn(u32) -> u32) -> Result<Vec<Plane>, String> {
    let mut planes = Vec::new();
    let mut at = start;
    while planes.len() < max {
        let ctl = read(at);
        if ctl & (1 << 31) != 0 {
            return Ok(planes);
        }
        let words = bits(ctl, 29, 24);
        if ctl & (1 << 30) == 0 || words < 4 {
            return Err(format!("word {at}: 0x{ctl:08x} is no plane"));
        }
        let unity = ctl & (1 << 4) != 0;
        let pos0 = read(at + 1);
        let (width, height, source) = if unity {
            let pos2 = read(at + 2);
            (bits(pos2, 11, 0), bits(pos2, 27, 16), None)
        } else {
            let (pos1, pos2) = (read(at + 2), read(at + 3));
            (bits(pos1, 11, 0), bits(pos1, 27, 16), Some((bits(pos2, 11, 0), bits(pos2, 27, 16))))
        };
        planes.push(Plane {
            start: at,
            words,
            format: ctl & 0xf,
            unity,
            x: bits(pos0, 11, 0),
            y: bits(pos0, 23, 12),
            width,
            height,
            source,
        });
        at += words;
    }
    Ok(planes)
}

pub fn hvs_format(format: u32) -> &'static str {
    match format {
        0 => "rgb332",
        1 => "rgba4444",
        2 => "rgb555",
        3 => "rgba5551",
        4 => "rgb565",
        5 => "rgb888",
        6 => "rgba6666",
        7 => "rgba8888",
        8 => "yuv420-3plane",
        9 => "yuv420-2plane",
        10 => "yuv422-3plane",
        11 => "yuv422-2plane",
        _ => "?",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODE_1080P: Timing = Timing {
        hactive: 1920, hfp: 88, hsync: 44, hbp: 148, vactive: 1080, vfp: 4, vsync: 5, vbp: 36,
    };

    #[test]
    fn pixel_valve_1080p() {
        // The values pixel valve 2 had on the 3B+ in Pi test #324.
        let t = pv_timing(148 << 16 | 44, 88 << 16 | 1920, 36 << 16 | 5, 4 << 16 | 1080);
        assert_eq!(t, MODE_1080P);
        assert_eq!((t.htotal(), t.vtotal()), (2200, 1125));
        assert!((t.refresh(148.5e6) - 60.0).abs() < 1e-9);
    }

    #[test]
    fn hdmi_1080p() {
        let horza = 1920 | 1 << 14 | 1 << 13;
        let horzb = 88 | 44 << 10 | 148 << 20;
        let verta = 1080 | 4 << 13 | 5 << 20;
        assert_eq!(hdmi_timing(horza, horzb, verta, 36), MODE_1080P);
        assert_eq!(hdmi_polarity(horza), (true, true));
    }

    #[test]
    fn channel_and_status() {
        assert_eq!(hvs_channel(1 << 31 | 1920 << 12 | 1080), (true, 1920, 1080));
        assert_eq!(hvs_status(2 << 30 | 1 << 28 | 5 << 12 | 77), (2, false, true, 5, 77));
    }

    #[test]
    fn list_with_a_scaled_and_a_unity_plane() {
        let mut mem = vec![0u32; 32];
        // Scaled plane, 9 words: rgba8888 1824x984 at (48, 22), source 640x480.
        mem[0] = 1 << 30 | 9 << 24 | 7;
        mem[1] = 22 << 12 | 48;
        mem[2] = 984 << 16 | 1824;
        mem[3] = 480 << 16 | 640;
        // Unity plane, 7 words: rgb565 64x64 at (100, 200).
        mem[9] = 1 << 30 | 7 << 24 | 1 << 4 | 4;
        mem[10] = 200 << 12 | 100;
        mem[11] = 64 << 16 | 64;
        mem[16] = 1 << 31;
        let planes = walk_list(0, 16, |i| mem[i as usize]).unwrap();
        assert_eq!(planes.len(), 2);
        assert_eq!((planes[0].width, planes[0].height, planes[0].source), (1824, 984, Some((640, 480))));
        assert_eq!((planes[0].x, planes[0].y, hvs_format(planes[0].format)), (48, 22, "rgba8888"));
        assert_eq!((planes[1].start, planes[1].unity, planes[1].width), (9, true, 64));
    }

    #[test]
    fn list_garbage_is_an_error() {
        assert!(walk_list(0, 16, |_| 0x1234).is_err());
    }
}
