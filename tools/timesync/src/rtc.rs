//! DS3231 real-time clock on I2C bus 1, address 0x68, through the
//! `bcm2835-i2cd` driver (experiment `i2c`): a read or write at offset N of
//! `/scheme/i2c/1/68` starts at register N.
//!
//! Registers 0x00-0x06 hold the time in BCD (seconds, minutes, hours,
//! weekday, date, month with the century bit, year). Bit 7 of the status
//! register 0x0f (OSF) is set when the oscillator stopped, for example
//! after the battery ran out; the time is then not valid.

use std::fs::{File, OpenOptions};
use std::io;
use std::os::unix::fs::FileExt;

const DEVICE: &str = "/scheme/i2c/1/68";
const STATUS: u64 = 0x0f;
const OSF: u8 = 0x80;

fn bcd(byte: u8) -> Option<u32> {
    let (high, low) = (byte >> 4, byte & 0x0f);
    (high < 10 && low < 10).then_some(u32::from(high) * 10 + u32::from(low))
}

fn to_bcd(value: u32) -> u8 {
    ((value / 10) << 4 | value % 10) as u8
}

/// Unix time of seven time registers, None if a field is out of range.
fn decode(r: &[u8; 7]) -> Option<u64> {
    // Bit 7 of the seconds is the DS1307/DS1338 clock-halt bit.
    if r[0] & 0x80 != 0 {
        return None;
    }
    let second = bcd(r[0])?;
    let minute = bcd(r[1])?;
    let hour = if r[2] & 0x40 != 0 {
        // 12-hour mode: bit 5 is PM.
        let h = bcd(r[2] & 0x1f)?;
        if !(1..=12).contains(&h) {
            return None;
        }
        h % 12 + if r[2] & 0x20 != 0 { 12 } else { 0 }
    } else {
        bcd(r[2] & 0x3f)?
    };
    let day = bcd(r[4])?;
    let month = bcd(r[5] & 0x1f)?;
    let year = 2000 + bcd(r[6])? + if r[5] & 0x80 != 0 { 100 } else { 0 };
    if second > 59 || minute > 59 || hour > 23 || !(1..=31).contains(&day) || !(1..=12).contains(&month) {
        return None;
    }
    let days = crate::days_from_civil(i64::from(year), i64::from(month), i64::from(day));
    u64::try_from(days * 86_400 + i64::from(hour * 3600 + minute * 60 + second)).ok()
}

/// Seven time registers of a Unix time, 24-hour mode.
fn encode(unix: u64) -> [u8; 7] {
    let days = (unix / 86_400) as i64;
    let secs = (unix % 86_400) as u32;
    let (year, month, day) = crate::civil_from_days(days);
    // DS3231 weekday 1-7; 1970-01-01 was a Thursday, counted here as 1 = Sunday.
    let weekday = ((days + 4).rem_euclid(7) + 1) as u32;
    let century = if year >= 2100 { 0x80 } else { 0 };
    [
        to_bcd(secs % 60),
        to_bcd(secs / 60 % 60),
        to_bcd(secs / 3600),
        to_bcd(weekday),
        to_bcd(day),
        to_bcd(month) | century,
        to_bcd((year % 100) as u32),
    ]
}

/// The RTC's time. Ok(None) when there is no RTC; an error when it is there
/// but its time is not valid.
pub fn read() -> io::Result<Option<u64>> {
    let file = match File::open(DEVICE) {
        Ok(file) => file,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err),
    };
    let mut regs = [0u8; 7];
    match file.read_exact_at(&mut regs, 0) {
        Ok(()) => {}
        // ENXIO: nothing answers at 0x68.
        Err(err) if err.raw_os_error() == Some(6) => return Ok(None),
        Err(err) => return Err(err),
    }
    let mut status = [0u8];
    file.read_exact_at(&mut status, STATUS)?;
    if status[0] & OSF != 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "its oscillator stopped (battery?)"));
    }
    decode(&regs)
        .map(Some)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("invalid time registers {regs:02x?}")))
}

/// Set the RTC and clear OSF. Ok(false) when there is no RTC.
pub fn write(unix: u64) -> io::Result<bool> {
    let file = match OpenOptions::new().read(true).write(true).open(DEVICE) {
        Ok(file) => file,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(err),
    };
    match file.write_all_at(&encode(unix), 0) {
        Ok(()) => {}
        Err(err) if err.raw_os_error() == Some(6) => return Ok(false),
        Err(err) => return Err(err),
    }
    let mut status = [0u8];
    file.read_exact_at(&mut status, STATUS)?;
    if status[0] & OSF != 0 {
        file.write_all_at(&[status[0] & !OSF], STATUS)?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        // 2026-10-29 10:58:16 UTC, a Thursday.
        let unix = 1_793_271_496;
        let regs = encode(unix);
        assert_eq!(regs, [0x16, 0x58, 0x10, 0x05, 0x29, 0x10, 0x26]);
        assert_eq!(decode(&regs), Some(unix));
        // 12-hour mode, 10 PM.
        assert_eq!(decode(&[0x16, 0x58, 0x40 | 0x20 | 0x10, 5, 0x29, 0x10, 0x26]), Some(unix + 12 * 3600));
        assert_eq!(decode(&[0x80, 0, 0, 1, 1, 1, 0]), None); // halted
        assert_eq!(decode(&[0x5a, 0, 0, 1, 1, 1, 0x26]), None); // not BCD
        assert_eq!(decode(&[0, 0, 0, 1, 0, 1, 0x26]), None); // day 0
    }
}
