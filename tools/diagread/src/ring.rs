//! The ring of the firmware's display sampler (rpi-open-firmware
//! `bootcode/diag.c`, #66) and what we read from it. Pure code: it works on
//! words already copied out of memory or a file, so the host tests run it.
//!
//! Layout, little-endian 32-bit words:
//!
//! ```text
//! header (16 words): magic "DIAG", version, record words, records in the
//!   ring, records written (wraps), period in us, marker, dropped samples
//! record (8 words):  timer us, HVS DISPSTAT, HVS DISPSTAT1, PV2 STAT,
//!   HDMI frame count, thermal sensor status, PLLB word, marker
//! ```
//!
//! Version 2 adds header word 8 (flags, bit 0: AXI counters) and word 9 (the
//! six AXI bus numbers, 5 bits each). With AXI counters a record has 32
//! words: after the 8 above, per bus watcher read transactions, write
//! transactions and wait cycles since the last record; words 26..31 are
//! kept for V3D counters.

pub const MAGIC: u32 = 0x4741_4944; // "DIAG"
pub const HEADER_WORDS: usize = 16;

const H_MAGIC: usize = 0;
const H_VERSION: usize = 1;
const H_REC_WORDS: usize = 2;
const H_RECS: usize = 3;
const H_WRITTEN: usize = 4;
const H_PERIOD: usize = 5;
const H_MARKER: usize = 6;
const H_DROPPED: usize = 7;
const H_FLAGS: usize = 8;
const H_AXI_BUSES: usize = 9;

/// Byte offset of the marker word, which the ARM may write.
pub const MARKER_OFFSET: usize = H_MARKER * 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub version: u32,
    pub rec_words: u32,
    pub recs: u32,
    pub written: u32,
    pub period_us: u32,
    pub marker: u32,
    pub dropped: u32,
    pub axi: bool,
    pub axi_buses: [u32; 6],
}

impl Header {
    pub fn parse(words: &[u32]) -> Result<Header, String> {
        Header::parse_head(words, words.len())
    }

    /// From the first words only; `total_words` is the size of the ring.
    pub fn parse_head(words: &[u32], total_words: usize) -> Result<Header, String> {
        if words.len() < HEADER_WORDS {
            return Err("ring shorter than its header".into());
        }
        if words[H_MAGIC] != MAGIC {
            return Err(format!(
                "no sampler ring (magic 0x{:08x}, not \"DIAG\"): not started, or not our firmware",
                words[H_MAGIC]
            ));
        }
        let h = Header {
            version: words[H_VERSION],
            rec_words: words[H_REC_WORDS],
            recs: words[H_RECS],
            written: words[H_WRITTEN],
            period_us: words[H_PERIOD],
            marker: words[H_MARKER],
            dropped: words[H_DROPPED],
            axi: words[H_VERSION] >= 2 && words[H_FLAGS] & 1 != 0,
            axi_buses: std::array::from_fn(|i| (words[H_AXI_BUSES] >> (5 * i)) & 0x1f),
        };
        let known = match h.version {
            1 => h.rec_words == 8,
            2 => h.rec_words == if h.axi { 32 } else { 8 },
            _ => false,
        };
        if !known {
            return Err(format!("ring version {} with {} words per record: this reader knows versions 1 and 2", h.version, h.rec_words));
        }
        if h.recs == 0 || HEADER_WORDS + (h.recs as usize) * (h.rec_words as usize) > total_words {
            return Err(format!("ring says {} records, more than its {} bytes", h.recs, total_words * 4));
        }
        Ok(h)
    }

    /// Records in the ring now (at most `recs`).
    pub fn valid(&self) -> u32 {
        self.written.min(self.recs)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Record {
    pub t_us: u32,
    pub dispstat: u32,
    pub dispstat1: u32,
    pub pv2_stat: u32,
    pub hdmi_frames: u32,
    pub ts_stat: u32,
    pub pllb: u32,
    pub marker: u32,
    /// AXI bus watchers (version 2 with AXI), else all 0.
    pub axi: [Axi; 6],
}

/// One AXI bus watcher's counts since the record before.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Axi {
    pub reads: u32,
    pub writes: u32,
    pub wait: u32,
}

/// Name of the bus that watcher `w` (0..2 system monitor, 3..5 VPU monitor)
/// counts. Tables of Raspberry Pi's Linux `raspberrypi_axi_monitor.c`
/// (BCM2835/6/7).
pub fn axi_bus_name(w: usize, bus: u32) -> &'static str {
    const SYSTEM: [&str; 16] = [
        "DMA_L2", "TRANS", "JPEG", "SYSTEM_UC", "DMA_UC", "SYSTEM_L2", "CCP2TX", "MPHI_RX", "MPHI_TX", "HVS", "H264",
        "ISP", "V3D", "PERIPHERAL", "CPU_UC", "CPU_L2",
    ];
    const VPU: [&str; 16] = [
        "VPU1_D_L2", "VPU0_D_L2", "VPU1_I_L2", "VPU0_I_L2", "SYSTEM_L2", "L2_FLUSH", "DMA_L2", "VPU1_D_UC",
        "VPU0_D_UC", "VPU1_I_UC", "VPU0_I_UC", "SYSTEM_UC", "L2_OUT", "DMA_UC", "SDRAM", "L2_IN",
    ];
    let table = if w < 3 { &SYSTEM } else { &VPU };
    table.get(bus as usize).copied().unwrap_or("?")
}

impl Record {
    pub fn from_words(w: &[u32]) -> Record {
        let axi = if w.len() >= 26 {
            std::array::from_fn(|i| Axi { reads: w[8 + 3 * i], writes: w[9 + 3 * i], wait: w[10 + 3 * i] })
        } else {
            [Axi::default(); 6]
        };
        Record {
            axi,
            t_us: w[0],
            dispstat: w[1],
            dispstat1: w[2],
            pv2_stat: w[3],
            hdmi_frames: w[4],
            ts_stat: w[5],
            pllb: w[6],
            marker: w[7],
        }
    }

    /// HVS channel 1 frame count (DISPSTAT1 bits 17:12, 6 bits; Linux
    /// `vc4_regs.h` SCALER_DISPSTATX_FRAME_COUNT). Channel 1 feeds pixel
    /// valve 2 and HDMI on the BCM2835/6/7.
    pub fn hvs_frame(&self) -> u32 {
        (self.dispstat1 >> 12) & 0x3f
    }

    /// Temperature in millidegrees, when the sensor value is valid (bit 10).
    /// Linux `bcm2835_thermal.c`: offset 407000, slope -538 per step.
    pub fn temp_mc(&self) -> Option<i32> {
        (self.ts_stat & (1 << 10) != 0).then(|| 407_000 - 538 * (self.ts_stat & 0x3ff) as i32)
    }

    /// PLLB multiplier (ndiv in bits 29:20 + frac / 2^20). With the feedback
    /// prediv as on the Pi 3B+ and Zero 2 W, PLLB runs at 19.2 MHz times this.
    pub fn pllb_mult(&self) -> f64 {
        f64::from((self.pllb >> 20) & 0x3ff) + f64::from(self.pllb & 0xf_ffff) / f64::from(1u32 << 20)
    }

    /// The ARM channel divider in bits 31:30 (rof #67 since Test 530; 0 from
    /// older firmware, taken as 1).
    pub fn arm_div(&self) -> u32 {
        (self.pllb >> 30).max(1)
    }

    /// The ARM clock, an estimate: PLLB / the ARM channel divider.
    pub fn arm_mhz(&self) -> f64 {
        19.2 * self.pllb_mult() / f64::from(self.arm_div())
    }
}

/// Channel 1 error bits of HVS DISPSTAT (Linux `vc4_regs.h`, BIT(n + 8 * 1)):
/// sticky until someone writes them back.
pub const HVS_FLAGS: [(u32, &str); 5] = [
    (1 << 17, "hvs1 underflow (EUFLOW)"),
    (1 << 18, "hvs1 line error (ESLINE)"),
    (1 << 19, "hvs1 frame error (ESFRAME)"),
    (1 << 21, "hvs1 COB low (COBLOW)"),
    (1 << 7, "hvs DMA error"),
];

/// Records oldest first, from a whole ring image (header + records).
pub fn records(words: &[u32], h: &Header) -> Vec<Record> {
    let n = h.valid();
    (h.written - n..h.written)
        .map(|i| {
            let at = HEADER_WORDS + (i % h.recs) as usize * h.rec_words as usize;
            Record::from_words(&words[at..at + h.rec_words as usize])
        })
        .collect()
}

/// Little-endian bytes to words (a saved ring file).
pub fn words_from_bytes(bytes: &[u8]) -> Vec<u32> {
    bytes.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A counter stood still from `t_us` for `len_us`.
    Stall { what: &'static str, t_us: u32, len_us: u32 },
    /// The sampler itself had no record for `len_us` (VPU busy elsewhere).
    Gap { t_us: u32, len_us: u32 },
    /// An HVS error bit went from 0 to 1.
    Flag { what: &'static str, t_us: u32 },
    /// The ARM clock changed (throttling, governor).
    Clock { t_us: u32, from_mhz: u32, to_mhz: u32 },
}

/// Finds stalls in a stream of records. Feed records oldest first; `finish`
/// reports a stall still going on at the end.
pub struct Detector {
    threshold_us: u32,
    period_us: u32,
    prev: Option<Record>,
    hdmi_since: u32,
    hvs_since: u32,
    marker_since: u32,
    marker_moved: bool,
}

impl Detector {
    pub fn new(threshold_us: u32, period_us: u32) -> Detector {
        Detector { threshold_us, period_us, prev: None, hdmi_since: 0, hvs_since: 0, marker_since: 0, marker_moved: false }
    }

    pub fn feed(&mut self, r: &Record, out: &mut Vec<Event>) {
        let Some(p) = self.prev else {
            self.hdmi_since = r.t_us;
            self.hvs_since = r.t_us;
            self.marker_since = r.t_us;
            self.prev = Some(*r);
            return;
        };
        let dt = r.t_us.wrapping_sub(p.t_us);
        if dt > 3 * self.period_us.max(1) && dt >= self.threshold_us {
            out.push(Event::Gap { t_us: p.t_us, len_us: dt });
        }
        let mut stall = |what, since: &mut u32, moved: bool| {
            if moved {
                let len = r.t_us.wrapping_sub(*since);
                if len >= self.threshold_us {
                    out.push(Event::Stall { what, t_us: *since, len_us: len });
                }
                *since = r.t_us;
            }
        };
        stall("hdmi frames", &mut self.hdmi_since, r.hdmi_frames != p.hdmi_frames);
        stall("hvs1 frames", &mut self.hvs_since, r.hvs_frame() != p.hvs_frame());
        if r.marker != p.marker {
            if self.marker_moved {
                stall("marker", &mut self.marker_since, true);
            } else {
                self.marker_since = r.t_us;
            }
            self.marker_moved = true;
        }
        for (bit, what) in HVS_FLAGS {
            if r.dispstat & bit != 0 && p.dispstat & bit == 0 {
                out.push(Event::Flag { what, t_us: r.t_us });
            }
        }
        let (a, b) = (p.arm_mhz().round() as u32, r.arm_mhz().round() as u32);
        if a != b {
            out.push(Event::Clock { t_us: r.t_us, from_mhz: a, to_mhz: b });
        }
        self.prev = Some(*r);
    }

    /// Stalls still going on at the last record.
    pub fn finish(&self, out: &mut Vec<Event>) {
        let Some(p) = self.prev else { return };
        let mut open = |what, since: u32| {
            let len = p.t_us.wrapping_sub(since);
            if len >= self.threshold_us {
                out.push(Event::Stall { what, t_us: since, len_us: len });
            }
        };
        open("hdmi frames (still)", self.hdmi_since);
        open("hvs1 frames (still)", self.hvs_since);
        if self.marker_moved {
            open("marker (still)", self.marker_since);
        }
    }
}

pub fn analyse(recs: &[Record], threshold_us: u32, period_us: u32) -> Vec<Event> {
    let mut d = Detector::new(threshold_us, period_us);
    let mut out = Vec::new();
    for r in recs {
        d.feed(r, &mut out);
    }
    d.finish(&mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 1 ms records; HDMI frame every 16.7 ms, HVS frame with it.
    fn stream(n: u32, freeze: std::ops::Range<u32>) -> Vec<Record> {
        let mut frame = 0;
        let mut v = Vec::new();
        for i in 0..n {
            if !freeze.contains(&i) && i % 17 == 0 {
                frame += 1;
            }
            v.push(Record {
                t_us: 5_000_000 + i * 1000,
                dispstat1: (frame & 0x3f) << 12,
                hdmi_frames: frame,
                ts_stat: (1 << 10) | 650,
                pllb: (72 << 20) | 0xe_6666,
                ..Default::default()
            });
        }
        v
    }

    fn ring(recs: u32, written: u32) -> Vec<u32> {
        let mut w = vec![0u32; HEADER_WORDS + recs as usize * 8];
        w[..8].copy_from_slice(&[MAGIC, 1, 8, recs, written, 1000, 0, 0]);
        w[9] = 0x1f; // ignored in version 1
        for i in 0..written {
            let at = HEADER_WORDS + (i % recs) as usize * 8;
            w[at] = i;
        }
        w
    }

    #[test]
    fn header_and_wrap() {
        let w = ring(4, 6);
        let h = Header::parse(&w).unwrap();
        assert_eq!(h.valid(), 4);
        let t: Vec<u32> = records(&w, &h).iter().map(|r| r.t_us).collect();
        assert_eq!(t, vec![2, 3, 4, 5]);
        let w = ring(4, 3);
        let h = Header::parse(&w).unwrap();
        assert_eq!(records(&w, &h).len(), 3);
    }

    #[test]
    fn bad_magic_and_version() {
        let mut w = ring(4, 0);
        w[0] = 0;
        assert!(Header::parse(&w).unwrap_err().contains("not started"));
        let mut w = ring(4, 0);
        w[1] = 3;
        assert!(Header::parse(&w).is_err());
        let mut w = ring(4, 0);
        w[3] = 100;
        assert!(Header::parse(&w).is_err());
    }

    #[test]
    fn version_2_with_axi() {
        let recs = 3;
        let mut w = vec![0u32; HEADER_WORDS + recs * 32];
        let buses = 9 | 15 << 5 | 12 << 10 | 14 << 15 | 15 << 20 | 13 << 25;
        w[..10].copy_from_slice(&[MAGIC, 2, 32, recs as u32, 4, 1000, 0, 0, 1, buses]);
        for i in 0..4usize {
            let at = HEADER_WORDS + (i % recs) * 32;
            w[at] = i as u32;
            w[at + 8] = 100 + i as u32; // HVS reads
            w[at + 25] = 7; // DMA_UC wait
        }
        let h = Header::parse(&w).unwrap();
        assert!(h.axi);
        assert_eq!(h.axi_buses, [9, 15, 12, 14, 15, 13]);
        assert_eq!(axi_bus_name(0, h.axi_buses[0]), "HVS");
        assert_eq!(axi_bus_name(3, h.axi_buses[3]), "SDRAM");
        let r = records(&w, &h);
        assert_eq!(r.iter().map(|r| r.axi[0].reads).collect::<Vec<_>>(), vec![101, 102, 103]);
        assert_eq!(r[0].axi[5].wait, 7);
        // version 2 without AXI: 8 words; with the flag but 8 words: refused
        let mut w8 = ring(4, 2);
        w8[1] = 2;
        assert!(!Header::parse(&w8).unwrap().axi);
        w8[8] = 1;
        assert!(Header::parse(&w8).is_err());
    }

    #[test]
    fn decode() {
        let r = stream(1, 0..0)[0];
        assert_eq!(r.temp_mc(), Some(407_000 - 538 * 650));
        assert_eq!(r.arm_mhz().round() as u32, 1400);
        assert_eq!(Record { ts_stat: 650, ..r }.temp_mc(), None);
    }

    #[test]
    fn smooth_stream_has_no_events() {
        assert_eq!(analyse(&stream(2000, 0..0), 50_000, 1000), vec![]);
    }

    #[test]
    fn frozen_display_is_found() {
        let ev = analyse(&stream(3000, 1000..2000), 50_000, 1000);
        let stalls: Vec<_> = ev.iter().filter(|e| matches!(e, Event::Stall { .. })).collect();
        assert_eq!(stalls.len(), 2, "{ev:?}");
        if let Event::Stall { what, len_us, .. } = stalls[0] {
            assert_eq!(*what, "hdmi frames");
            assert!((1_000_000..=1_040_000).contains(len_us), "{len_us}");
        }
    }

    #[test]
    fn stall_at_the_end_is_reported() {
        let ev = analyse(&stream(2000, 1500..2000), 50_000, 1000);
        assert!(ev.iter().any(|e| matches!(e, Event::Stall { what: "hdmi frames (still)", .. })), "{ev:?}");
    }

    #[test]
    fn gap_flag_clock_and_marker() {
        let mut s = stream(400, 0..0);
        for r in s.iter_mut().skip(200) {
            r.t_us += 300_000; // the VPU missed 300 ms
            r.dispstat |= 1 << 17;
            r.pllb = 31 << 20; // 595 MHz
        }
        for (i, r) in s.iter_mut().enumerate() {
            r.marker = if i < 100 { 1 } else if i < 390 { 2 } else { 3 };
        }
        let ev = analyse(&s, 50_000, 1000);
        assert!(ev.iter().any(|e| matches!(e, Event::Gap { len_us: 301_000, .. })), "{ev:?}");
        assert!(ev.iter().any(|e| matches!(e, Event::Flag { what: "hvs1 underflow (EUFLOW)", .. })));
        assert!(ev.iter().any(|e| matches!(e, Event::Clock { from_mhz: 1400, to_mhz: 595, .. })));
        assert!(ev.iter().any(|e| matches!(e, Event::Stall { what: "marker", .. })), "{ev:?}");
    }

    #[test]
    fn marker_never_written_is_quiet() {
        let ev = analyse(&stream(3000, 0..0), 50_000, 1000);
        assert!(!ev.iter().any(|e| matches!(e, Event::Stall { what: "marker" | "marker (still)", .. })));
    }

    #[test]
    fn arm_divider_and_short_gaps() {
        // PLLB 1200 MHz (ndiv 62, frac 0x80000), ARM channel / 2: 600 MHz
        let r = Record { pllb: 2 << 30 | 62 << 20 | 0x8_0000, ..stream(1, 0..0)[0] };
        assert_eq!(r.arm_mhz().round() as u32, 600);
        assert_eq!(Record { pllb: 1 << 30 | 62 << 20 | 0x8_0000, ..r }.arm_mhz().round() as u32, 1200);
        // a 20 ms gap is below a 50 ms threshold
        let mut s = stream(400, 0..0);
        for r in s.iter_mut().skip(200) {
            r.t_us += 20_000;
        }
        assert!(!analyse(&s, 50_000, 1000).iter().any(|e| matches!(e, Event::Gap { .. })));
        assert!(analyse(&s, 10_000, 1000).iter().any(|e| matches!(e, Event::Gap { .. })));
    }

}
