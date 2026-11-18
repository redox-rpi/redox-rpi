//! Cron schedules: five fields (minute hour day-of-month month day-of-week)
//! with `*`, numbers, ranges `a-b`, steps `/n` and lists `a,b`, plus the
//! shortcuts `@hourly`, `@daily`, `@weekly`, `@monthly` and `@yearly`. Times
//! are in UTC. As in Vixie cron, when both day fields are restricted, a day
//! matches if either does.

/// One field as a bit set: bit n is set if value n matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Field {
    bits: u64,
    any: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    minute: Field,
    hour: Field,
    day: Field,
    month: Field,
    weekday: Field,
}

fn parse_field(text: &str, min: u32, max: u32) -> Result<Field, String> {
    let mut bits = 0u64;
    for part in text.split(',') {
        let (range, step) = match part.split_once('/') {
            Some((range, step)) => {
                let step: u32 = step.parse().map_err(|_| format!("bad step in {part:?}"))?;
                if step == 0 {
                    return Err(format!("step 0 in {part:?}"));
                }
                (range, step)
            }
            None => (part, 1),
        };
        let (from, to) = if range == "*" {
            (min, max)
        } else if let Some((a, b)) = range.split_once('-') {
            let a = a.parse().map_err(|_| format!("bad number in {part:?}"))?;
            let b = b.parse().map_err(|_| format!("bad number in {part:?}"))?;
            (a, b)
        } else {
            let a = range.parse().map_err(|_| format!("bad number in {part:?}"))?;
            // "5/15" means from 5 to the end, every 15.
            (a, if step > 1 { max } else { a })
        };
        if from < min || to > max || from > to {
            return Err(format!("{part:?} is outside {min}-{max}"));
        }
        for value in (from..=to).step_by(step as usize) {
            bits |= 1 << value;
        }
    }
    Ok(Field { bits, any: text == "*" })
}

impl Field {
    fn has(&self, value: u32) -> bool {
        self.bits & (1 << value) != 0
    }
}

impl Schedule {
    pub fn parse(text: &str) -> Result<Schedule, String> {
        let text = match text.trim() {
            "@hourly" => "0 * * * *",
            "@daily" | "@midnight" => "0 0 * * *",
            "@weekly" => "0 0 * * 0",
            "@monthly" => "0 0 1 * *",
            "@yearly" | "@annually" => "0 0 1 1 *",
            other => other,
        };
        let fields: Vec<&str> = text.split_whitespace().collect();
        let [minute, hour, day, month, weekday] = fields[..] else {
            return Err(format!("{text:?} does not have five fields"));
        };
        let mut weekday = parse_field(weekday, 0, 7)?;
        // 7 is Sunday too.
        if weekday.has(7) {
            weekday.bits |= 1;
        }
        Ok(Schedule {
            minute: parse_field(minute, 0, 59)?,
            hour: parse_field(hour, 0, 23)?,
            day: parse_field(day, 1, 31)?,
            month: parse_field(month, 1, 12)?,
            weekday,
        })
    }

    fn day_matches(&self, days: i64) -> bool {
        let (_, month, day) = civil_from_days(days);
        if !self.month.has(month) {
            return false;
        }
        // 1970-01-01 was a Thursday.
        let weekday = (days + 4).rem_euclid(7) as u32;
        match (self.day.any, self.weekday.any) {
            (true, true) => true,
            (false, true) => self.day.has(day),
            (true, false) => self.weekday.has(weekday),
            (false, false) => self.day.has(day) || self.weekday.has(weekday),
        }
    }

    /// The first matching minute after `unix` (seconds), as Unix time, with
    /// the fields matched against local time: `offset(t)` is how many
    /// seconds local time is ahead of UTC at Unix time t. None if nothing
    /// matches within ten years (such as February 30th).
    ///
    /// When the clocks go forward, the skipped times do not run; when they
    /// go back, the repeated hour matches twice (timerd runs it once).
    pub fn next_after(&self, unix: u64, offset: &dyn Fn(u64) -> i64) -> Option<u64> {
        // Minutes of Unix time; each one is checked in local time.
        let mut minute = unix / 60 + 1;
        let limit = minute + 10 * 366 * 1440;
        while minute < limit {
            let local = (minute * 60) as i64 + offset(minute * 60);
            let local_minute = local.div_euclid(60);
            let of_day = local_minute.rem_euclid(1440) as u64;
            if !self.day_matches(local_minute.div_euclid(1440)) {
                // To local midnight, but stop an hour early: the offset may
                // change on the way.
                let to_midnight = 1440 - of_day;
                minute += if to_midnight > 120 { to_midnight - 60 } else { to_midnight };
                continue;
            }
            if !self.hour.has((of_day / 60) as u32) {
                minute += 60 - of_day % 60;
                continue;
            }
            if self.minute.has((of_day % 60) as u32) {
                return Some(minute * 60);
            }
            minute += 1;
        }
        None
    }
}

/// (year, month, day) of a day count since 1970 (after Howard Hinnant).
pub fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(month <= 2), month, day)
}

/// "YYYY-MM-DD HH:MM:SS" in UTC.
pub fn format_utc(unix: u64) -> String {
    let (year, month, day) = civil_from_days((unix / 86_400) as i64);
    let secs = unix % 86_400;
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}",
        secs / 3600,
        secs / 60 % 60,
        secs % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2026-10-28 08:15:00 UTC, a Wednesday.
    const NOW: u64 = 1_793_175_300;

    fn next(schedule: &str, after: u64) -> String {
        format_utc(Schedule::parse(schedule).unwrap().next_after(after, &|_| 0).unwrap())
    }

    /// Europe/Berlin in 2026: CEST (+2 h) from 29 March 01:00 UTC to
    /// 25 October 01:00 UTC, CET (+1 h) otherwise.
    fn berlin(t: u64) -> i64 {
        if (1_774_746_000..1_792_890_000).contains(&t) { 7200 } else { 3600 }
    }

    /// The next run in Berlin local time, written as UTC plus offset.
    fn next_berlin(schedule: &str, after: u64) -> String {
        let due = Schedule::parse(schedule).unwrap().next_after(after, &berlin).unwrap();
        format!("{} +{}", format_utc((due as i64 + berlin(due)) as u64), berlin(due) / 3600)
    }

    #[test]
    fn next_runs() {
        assert_eq!(format_utc(NOW), "2026-10-28 08:15:00");
        assert_eq!(next("* * * * *", NOW), "2026-10-28 08:16:00");
        assert_eq!(next("* * * * *", NOW + 59), "2026-10-28 08:16:00");
        assert_eq!(next("*/10 * * * *", NOW), "2026-10-28 08:20:00");
        assert_eq!(next("30 3 * * *", NOW), "2026-10-29 03:30:00");
        assert_eq!(next("@hourly", NOW), "2026-10-28 09:00:00");
        assert_eq!(next("@monthly", NOW), "2026-11-01 00:00:00");
        assert_eq!(next("0 0 * * 0", NOW), "2026-11-01 00:00:00"); // Sunday
        assert_eq!(next("0 0 * * 7", NOW), "2026-11-01 00:00:00");
        assert_eq!(next("0 12 * * 1-5", NOW), "2026-10-28 12:00:00");
        assert_eq!(next("15 8 * * 3", NOW), "2026-11-04 08:15:00");
        assert_eq!(next("0 0 29 2 *", NOW), "2028-02-29 00:00:00");
        // Both day fields: the 1st or a Friday.
        assert_eq!(next("0 0 1 * 5", NOW), "2026-10-30 00:00:00");
        assert_eq!(next("5/20 * * * *", NOW), "2026-10-28 08:25:00");
        assert_eq!(next("0,30 9-10 * * *", NOW), "2026-10-28 09:00:00");
    }

    #[test]
    fn local_time() {
        // NOW is 09:15 CET.
        assert_eq!(next_berlin("30 3 * * *", NOW), "2026-10-29 03:30:00 +1");
        assert_eq!(next_berlin("0 12 * * *", NOW), "2026-10-28 12:00:00 +1");
        // Spring: 02:30 does not exist on 29 March, so the 30th.
        let march = 1_774_652_400; // 2026-03-28 00:00 CET
        assert_eq!(next_berlin("30 2 * * *", march), "2026-03-28 02:30:00 +1");
        assert_eq!(next_berlin("30 2 * * *", march + 86_400), "2026-03-30 02:30:00 +2");
        // Midnight just after the switch is not skipped.
        assert_eq!(next_berlin("15 0 30 3 *", march), "2026-03-30 00:15:00 +2");
        assert_eq!(next_berlin("0 4 * * *", march + 86_400), "2026-03-29 04:00:00 +2");
        // Autumn: 02:30 local comes twice on 25 October, first in CEST.
        let october = 1_792_879_200; // 2026-10-24 22:00 UTC, 25 Oct 00:00 CEST
        let first = Schedule::parse("30 2 * * *").unwrap().next_after(october, &berlin).unwrap();
        let second = Schedule::parse("30 2 * * *").unwrap().next_after(first, &berlin).unwrap();
        assert_eq!(second - first, 3600);
        assert_eq!(next_berlin("15 0 26 10 *", october), "2026-10-26 00:15:00 +1");
    }

    #[test]
    fn errors() {
        assert!(Schedule::parse("* * * *").is_err());
        assert!(Schedule::parse("60 * * * *").is_err());
        assert!(Schedule::parse("*/0 * * * *").is_err());
        assert!(Schedule::parse("0 0 0 * *").is_err());
        assert_eq!(Schedule::parse("0 0 30 2 *").unwrap().next_after(NOW, &|_| 0), None);
    }
}
