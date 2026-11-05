//! Biometric history: each day's face and fingerprint punches, newest first,
//! as entries, exits or plain punches with their place. Cached per account
//! and day (`pages/biometric/<epoch day>.json`; signing out removes
//! `pages/`), so a range reuses the days it already has. Port of the Flutter
//! app's `lib/features/biometric/` (`biometric_history_page.dart`).

use std::fs;

use tracing::info;
use vtop_core::VtopError;
use vtop_core::inputs::BiometricDate;
use vtop_core::types::{BiometricData, BiometricRecord};

use crate::{Core, CoreError, PAGES, records, traced, traced_sync};

const DIR: &str = "biometric";

/// The most days a range covers, one VTOP request per day not cached (as
/// the MCP's get_biometric_range).
const MAX_RANGE_DAYS: i32 = 14;

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct BiometricDay {
    /// Days since 1970-01-01, in India.
    pub day: i32,
    /// Unix seconds.
    pub updated_at: i64,
    /// Fetched after the day ended in India: VTOP adds no more punches, so
    /// a range serves it from the cache.
    pub complete: bool,
    /// Newest first.
    pub punches: Vec<Punch>,
    pub entries: i32,
    pub exits: i32,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Punch {
    /// Minutes since midnight; none when VTOP's time doesn't read as one.
    pub minute: Option<i32>,
    /// VTOP's time ("13:16:05"), for when `minute` is none.
    pub time: String,
    pub kind: PunchKind,
    /// "MH2" of "MH2-FACE-IN-5".
    pub place: String,
    /// VTOP's venue, as given.
    pub venue: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum PunchKind {
    Entry,
    Exit,
    Other,
}

#[uniffi::export(async_runtime = "tokio")]
impl Core {
    /// `day`'s cached punches (days since 1970-01-01, in India).
    pub fn cached_biometric(&self, flow: String, day: i32) -> Option<BiometricDay> {
        traced_sync("cached_biometric", &flow, || {
            Ok(self
                .cached_day(day)
                .map(|data| logged(BiometricDay::new(day, &data))))
        })
        .ok()
        .flatten()
    }

    /// Fetches `day`'s punches and caches them.
    pub async fn fetch_biometric(&self, flow: String, day: i32) -> Result<BiometricDay, CoreError> {
        traced("fetch_biometric", &flow, async {
            Ok(logged(BiometricDay::new(day, &self.fetch_day(day).await?)))
        })
        .await
    }

    /// The cached days from `from` to `to` (both included), oldest first;
    /// days never fetched are left out.
    pub fn cached_biometric_range(&self, flow: String, from: i32, to: i32) -> Vec<BiometricDay> {
        traced_sync("cached_biometric_range", &flow, || {
            let days: Vec<_> = range(from, to)?
                .filter_map(|day| {
                    self.cached_day(day)
                        .map(|data| BiometricDay::new(day, &data))
                })
                .collect();
            info!(days = days.len(), "biometric range cached");
            Ok(days)
        })
        .unwrap_or_default()
    }

    /// Every day from `from` to `to` (both included, at most 14), oldest
    /// first: complete days from the cache, the others fetched one at a
    /// time and cached.
    pub async fn fetch_biometric_range(
        &self,
        flow: String,
        from: i32,
        to: i32,
    ) -> Result<Vec<BiometricDay>, CoreError> {
        traced("fetch_biometric_range", &flow, async {
            let mut days = Vec::new();
            let mut fetched = 0;
            for day in range(from, to)? {
                let data = match self.cached_day(day).filter(|data| is_complete(day, data)) {
                    Some(data) => data,
                    None => {
                        fetched += 1;
                        self.fetch_day(day).await?
                    }
                };
                days.push(BiometricDay::new(day, &data));
            }
            info!(
                days = days.len(),
                fetched,
                punches = days.iter().map(|day| day.punches.len()).sum::<usize>(),
                "biometric range"
            );
            Ok(days)
        })
        .await
    }
}

impl Core {
    fn cached_day(&self, day: i32) -> Option<BiometricData> {
        self.read(&format!("{PAGES}/{DIR}/{day}.json"))
    }

    async fn fetch_day(&self, day: i32) -> Result<BiometricData, CoreError> {
        let date = BiometricDate::parse(&vtop_date(day))?;
        let data = signed_in!(self, |client| client.biometric_history(&date))?;
        let _ = fs::create_dir_all(self.dir.join(PAGES).join(DIR));
        self.write(&format!("{PAGES}/{DIR}/{day}.json"), &data);
        Ok(data)
    }
}

/// `from..=to`, checked.
fn range(from: i32, to: i32) -> Result<std::ops::RangeInclusive<i32>, CoreError> {
    if to < from || to - from >= MAX_RANGE_DAYS {
        return Err(VtopError::ConfigurationError(format!(
            "a range is 1 to {MAX_RANGE_DAYS} days, from first to last"
        ))
        .into());
    }
    Ok(from..=to)
}

/// Counts only: never a time or place.
fn logged(day: BiometricDay) -> BiometricDay {
    info!(
        punches = day.punches.len(),
        entries = day.entries,
        exits = day.exits,
        complete = day.complete,
        "biometric"
    );
    day
}

/// Days since 1970-01-01 → VTOP's "DD/MM/YYYY".
fn vtop_date(day: i32) -> String {
    let iso = records::civil_date(i64::from(day)); // "YYYY-MM-DD"
    let mut parts = iso.split('-');
    let (year, month, date) = (
        parts.next().unwrap_or_default(),
        parts.next().unwrap_or_default(),
        parts.next().unwrap_or_default(),
    );
    format!("{date}/{month}/{year}")
}

/// Fetched once `day` was over in India.
fn is_complete(day: i32, data: &BiometricData) -> bool {
    ((data.update_time as i64 + 19_800).div_euclid(86_400)) > i64::from(day)
}

impl BiometricDay {
    fn new(day: i32, data: &BiometricData) -> Self {
        let mut punches: Vec<(i32, Punch)> = data
            .records
            .iter()
            .map(|record| (seconds(&record.punch_time), Punch::from(record)))
            .collect();
        punches.sort_by_key(|(seconds, _)| std::cmp::Reverse(*seconds));
        let punches: Vec<Punch> = punches.into_iter().map(|(_, punch)| punch).collect();
        let count = |kind| punches.iter().filter(|punch| punch.kind == kind).count() as i32;
        Self {
            day,
            updated_at: data.update_time as i64,
            complete: is_complete(day, data),
            entries: count(PunchKind::Entry),
            exits: count(PunchKind::Exit),
            punches,
        }
    }
}

impl From<&BiometricRecord> for Punch {
    fn from(record: &BiometricRecord) -> Self {
        let venue = record.venue.trim().to_string();
        let upper = venue.to_uppercase();
        let kind = if upper.contains("-OUT-") {
            PunchKind::Exit
        } else if upper.contains("-IN-") {
            PunchKind::Entry
        } else {
            PunchKind::Other
        };
        Self {
            minute: minute(&record.punch_time),
            time: record.punch_time.trim().to_string(),
            kind,
            place: venue
                .split('-')
                .next()
                .unwrap_or_default()
                .trim()
                .to_string(),
            venue,
        }
    }
}

/// "13:16:05" → 796; VTOP sometimes drops the minute's leading zero ("13:1").
fn minute(time: &str) -> Option<i32> {
    let mut parts = time.trim().split(':');
    let hour: i32 = parts.next()?.trim().parse().ok()?;
    let minute: i32 = parts.next()?.trim().parse().ok()?;
    ((0..24).contains(&hour) && (0..60).contains(&minute)).then_some(hour * 60 + minute)
}

/// Seconds since midnight, for ordering; unreadable parts count as 0.
fn seconds(time: &str) -> i32 {
    let mut parts = time
        .trim()
        .split(':')
        .map(|part| part.trim().parse::<i32>().unwrap_or(0));
    (0..3).fold(0, |total, _| total * 60 + parts.next().unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(time: &str, venue: &str) -> BiometricRecord {
        BiometricRecord {
            serial: "1".into(),
            punch_date: "18/07/2026".into(),
            punch_time: time.into(),
            venue: venue.into(),
        }
    }

    /// 2026-07-18.
    const DAY: i32 = 20_652;

    fn data(records: Vec<BiometricRecord>, update_time: u64) -> BiometricData {
        BiometricData {
            records,
            requested_date: "18/07/2026".into(),
            update_time,
        }
    }

    #[test]
    fn dates_read_as_vtop_writes_them() {
        assert_eq!(vtop_date(DAY), "18/07/2026");
        assert_eq!(vtop_date(0), "01/01/1970");
        assert!(BiometricDate::parse(&vtop_date(DAY)).is_ok());
    }

    #[test]
    fn punches_sort_newest_first_with_kind_and_place() {
        let day = BiometricDay::new(
            DAY,
            &data(
                vec![
                    record("08:05:00", "AB1-FACE-IN-2"),
                    record("19:38", "MH2-IN-6-(349)"),
                    record("13:1", "MH2-FACE-OUT-5"),
                    record("?", "LIBRARY"),
                ],
                0,
            ),
        );
        let times: Vec<_> = day.punches.iter().map(|p| p.minute).collect();
        assert_eq!(times, [Some(1178), Some(781), Some(485), None]);
        let kinds: Vec<_> = day.punches.iter().map(|p| p.kind).collect();
        assert_eq!(
            kinds,
            [
                PunchKind::Entry,
                PunchKind::Exit,
                PunchKind::Entry,
                PunchKind::Other
            ]
        );
        assert_eq!(day.punches[0].place, "MH2");
        assert_eq!(day.punches[3].place, "LIBRARY");
        assert_eq!(day.punches[3].time, "?");
        assert_eq!((day.entries, day.exits), (2, 1));
    }

    #[test]
    fn a_day_is_complete_once_fetched_after_it_ended_in_india() {
        // 2026-07-18 23:59 IST and 2026-07-19 00:00 IST.
        let last_minute = (i64::from(DAY + 1) * 86_400 - 19_800 - 60) as u64;
        assert!(!is_complete(DAY, &data(vec![], last_minute)));
        assert!(is_complete(DAY, &data(vec![], last_minute + 60)));
    }

    #[test]
    fn ranges_are_one_to_fourteen_days() {
        assert_eq!(range(DAY, DAY).unwrap().count(), 1);
        assert_eq!(range(DAY, DAY + 13).unwrap().count(), 14);
        assert!(range(DAY, DAY + 14).is_err());
        assert!(range(DAY, DAY - 1).is_err());
    }

    #[test]
    fn ranges_read_the_days_cached() {
        let dir = std::env::temp_dir().join(format!("vtop-ffi-bio-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join(PAGES).join(DIR)).unwrap();
        // Not `Core::new`: it would point the log at this directory.
        let core = Core {
            dir: dir.clone(),
            slot: std::sync::Mutex::new(None),
        };
        let ended = (i64::from(DAY + 1) * 86_400) as u64;
        core.write(
            &format!("{PAGES}/{DIR}/{DAY}.json"),
            &data(vec![record("09:00", "AB1-FACE-IN-1")], ended),
        );
        let days = core.cached_biometric_range("f".into(), DAY - 1, DAY + 1);
        assert_eq!(days.len(), 1);
        assert_eq!(
            (days[0].day, days[0].complete, days[0].entries),
            (DAY, true, 1)
        );
        assert_eq!(core.cached_biometric("f".into(), DAY + 1), None);
        let _ = fs::remove_dir_all(&dir);
    }
}
