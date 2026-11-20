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
        };
        if h.version != 1 || h.rec_words != 8 {
            return Err(format!("ring version {} with {} words per record: this reader knows version 1, 8 words", h.version, h.rec_words));
        }
        if h.recs == 0 || HEADER_WORDS + (h.recs as usize) * 8 > total_words {
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
}

impl Record {
    pub fn from_words(w: &[u32]) -> Record {
        Record {
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

    /// PLLB multiplier (ndiv + frac / 2^20). With the feedback prediv and
    /// pllb_arm / 2 as on the Pi 3B+ and Zero 2 W, the ARM clock is about
    /// 19.2 MHz times this.
    pub fn pllb_mult(&self) -> f64 {
        f64::from(self.pllb >> 20) + f64::from(self.pllb & 0xf_ffff) / f64::from(1u32 << 20)
    }

    pub fn arm_mhz(&self) -> f64 {
        19.2 * self.pllb_mult()
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
            let at = HEADER_WORDS + (i % h.recs) as usize * 8;
            Record::from_words(&words[at..at + 8])
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
        if dt > 3 * self.period_us.max(1) {
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
        w[1] = 2;
        assert!(Header::parse(&w).is_err());
        let mut w = ring(4, 0);
        w[3] = 100;
        assert!(Header::parse(&w).is_err());
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
}
