//! Wrapped's campus slides: outings, time on campus, hostel and moves, from biometric.

use super::*;

/// A return counts as late after this minute.
pub(super) const EVENING: i32 = 17 * 60;
/// The biometric slides need this many class days with their punches.
pub(super) const BIOMETRIC_MIN_DAYS: i32 = 5;
/// The hostel bar's half hours, from this minute.
pub(super) const HALF_HOURS: usize = 36;
pub(super) const DAY_START: i32 = 6 * 60;

/// Approved requests starting from `first` to `end`, the outing window up to yesterday. A
/// leave's length counts only days before `today`. A leave is one trip: its longest run of days
/// without a punch (`on_campus`: a complete cached biometric day with one), since students
/// apply for more days than they take; without biometric it is the whole leave.
pub(super) fn outings(
    general: Option<&GeneralOutingData>,
    weekend: Option<&WeekendOutingData>,
    first: i32,
    end: i32,
    today: i32,
    on_campus: impl Fn(i32) -> bool,
) -> Option<WrappedOutings> {
    let window = first..=end;
    let approved = |status: &str| outing::standing(status) == OutingStanding::Approved;
    let leaves: Vec<(Option<(i32, i32)>, &str)> = general
        .iter()
        .flat_map(|data| &data.records)
        .filter(|record| approved(&record.status))
        .filter_map(|record| {
            let from = records::epoch_day(&record.from_date).filter(|day| window.contains(day))?;
            let to = records::epoch_day(&record.to_date)
                .unwrap_or(from)
                .min(today - 1)
                .max(from);
            Some((trip(from, to, &on_campus), record.place.trim()))
        })
        .collect();
    let weekends: Vec<&str> = weekend
        .iter()
        .flat_map(|data| &data.records)
        .filter(|record| {
            approved(&record.status)
                && records::epoch_day(&record.date).is_some_and(|day| window.contains(&day))
        })
        .map(|record| record.place.trim())
        .collect();
    if leaves.is_empty() && weekends.is_empty() {
        return None;
    }
    let trips = || leaves.iter().filter_map(|leave| leave.0);
    let away: HashSet<i32> = trips().flat_map(|(from, to)| from..=to.min(end)).collect();
    let (mut stay, mut longest_stay) = (0, 0);
    for day in window {
        stay = if away.contains(&day) { 0 } else { stay + 1 };
        longest_stay = longest_stay.max(stay);
    }
    let places = leaves
        .iter()
        .map(|leave| leave.1)
        .chain(weekends.iter().copied())
        .filter(|place| !place.is_empty())
        .map(|place| (place.to_string(), 1));
    Some(WrappedOutings {
        leaves: leaves.len() as i32,
        weekends: weekends.len() as i32,
        top_place: most(places).map(|(place, _)| place),
        longest_leave_days: trips().map(|(from, to)| to - from + 1).max(),
        days_away: away.len() as i32,
        longest_stay_days: longest_stay,
    })
}

/// The longest run of days from `from` to `to` not `on_campus` (the earliest of equals); none
/// when every day is.
pub(super) fn trip(from: i32, to: i32, on_campus: impl Fn(i32) -> bool) -> Option<(i32, i32)> {
    let (mut best, mut run): (Option<(i32, i32)>, Option<i32>) = (None, None);
    for day in from..=to {
        if on_campus(day) {
            run = None;
            continue;
        }
        let start = *run.get_or_insert(day);
        if best.is_none_or(|(a, b)| day - start > b - a) {
            best = Some((start, day));
        }
    }
    best
}

/// From the cached days of the semester's `total` class days; none without class days.
pub(super) fn campus(days: &[BiometricDay], total: i32) -> Option<WrappedCampus> {
    if total == 0 {
        return None;
    }
    let entries = |day: &BiometricDay| -> Vec<i32> {
        day.punches
            .iter()
            .filter(|punch| punch.kind == PunchKind::Entry)
            .filter_map(|punch| punch.minute)
            .collect()
    };
    let earliest = days
        .iter()
        .filter_map(|day| entries(day).into_iter().min())
        .min();
    let latest = days
        .iter()
        .filter_map(|day| entries(day).into_iter().max())
        .filter(|&minute| minute > EVENING)
        .max();
    Some(WrappedCampus {
        days_cached: days.len() as i32,
        days_total: total,
        earliest_entry: earliest.map(clock),
        latest_return: latest.map(clock),
        days_on_campus: days.iter().filter(|day| !day.punches.is_empty()).count() as i32,
    })
}

/// A class punch's block, room and floor.
pub(super) struct ClassPunch {
    pub(super) block: String,
    pub(super) room: String,
    pub(super) floor: i32,
}

/// "CB-214-FACE" is block CB, room 214, floor 2 (the hundreds); a room starting G ("G05"), or a
/// "GF" part, is the ground floor, and an "<n>F" part gives the floor too. The older form
/// "AB1-GF-(101)" has the room in brackets. None for a gate's IN or OUT ("MH2-FACE-IN-7", and
/// July 2026's "MH2-IN-3-(358)" with the gate's number in brackets) and without a room
/// ("LIB-FACE").
pub(super) fn class_punch(venue: &str) -> Option<ClassPunch> {
    let mut parts = venue.split('-').map(str::trim);
    let block = parts.next().filter(|block| !block.is_empty())?.to_string();
    let rest: Vec<&str> = parts.collect();
    if rest.iter().any(|part| matches!(*part, "IN" | "OUT")) {
        return None;
    }
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let room = rest
        .iter()
        .map(|part| part.trim_start_matches('(').trim_end_matches(')'))
        .find(|part| {
            (part.len() >= 3 && digits(part)) || part.strip_prefix('G').is_some_and(digits)
        })?;
    let floor = rest
        .iter()
        .find_map(|part| match *part {
            "GF" => Some(0),
            part => part.strip_suffix('F').filter(|n| digits(n))?.parse().ok(),
        })
        .unwrap_or_else(|| room.parse::<i32>().map_or(0, |number| number / 100));
    Some(ClassPunch {
        block,
        room: room.to_string(),
        floor,
    })
}

/// A hostel gate's IN or OUT: a place starting MH or LH.
pub(super) fn is_hostel(punch: &Punch) -> bool {
    let place = punch.place.to_uppercase();
    (place.starts_with("MH") || place.starts_with("LH")) && punch.kind != PunchKind::Other
}

/// Hostellers only: none under [`BIOMETRIC_MIN_DAYS`] days out of the hostel.
pub(super) fn hostel(days: &[BiometricDay]) -> Option<WrappedHostel> {
    let (mut out_days, mut out_minutes) = (0, 0);
    let (mut halves, mut naps) = ([0; HALF_HOURS], [0; 7]);
    for day in days {
        // In time order, with a time.
        let punches: Vec<(i32, &Punch)> = day
            .punches
            .iter()
            .rev()
            .filter_map(|punch| Some((punch.minute?, punch)))
            .collect();
        let gates: Vec<(i32, PunchKind)> = punches
            .iter()
            .filter(|(_, punch)| is_hostel(punch))
            .map(|&(minute, punch)| (minute, punch.kind))
            .collect();
        let classes: Vec<i32> = punches
            .iter()
            .filter(|(_, punch)| class_punch(&punch.venue).is_some())
            .map(|&(minute, _)| minute)
            .collect();
        for pair in gates.windows(2) {
            if let &[(entry, PunchKind::Entry), (exit, PunchKind::Exit)] = pair
                && classes.first().is_some_and(|&first| first < entry)
                && classes.last().is_some_and(|&last| last > exit)
                && let Some(naps) = naps.get_mut(records::weekday(day.day) as usize)
            {
                *naps += 1;
            }
        }
        let (mut out, mut left): (Vec<(i32, i32)>, Option<i32>) = (Vec::new(), None);
        for &(minute, kind) in &gates {
            if kind == PunchKind::Exit {
                left.get_or_insert(minute);
            } else if let Some(from) = left.take() {
                out.push((from, minute));
            }
        }
        if out.is_empty() {
            continue;
        }
        out_days += 1;
        out_minutes += out.iter().map(|(from, to)| to - from).sum::<i32>();
        for (at, half) in halves.iter_mut().enumerate() {
            let middle = DAY_START + 30 * at as i32 + 15;
            if out.iter().any(|&(from, to)| from <= middle && middle < to) {
                *half += 1;
            }
        }
    }
    if out_days < BIOMETRIC_MIN_DAYS {
        return None;
    }
    Some(WrappedHostel {
        days: out_days,
        out_tenths: (out_minutes * 10 + 30 * out_days) / (60 * out_days),
        out_by_half_hour: halves.iter().map(|&n| n * 2 >= out_days).collect(),
        naps: naps.iter().sum(),
        nap_weekday: naps
            .iter()
            .enumerate()
            .max_by_key(|&(day, &n)| (n, std::cmp::Reverse(day)))
            .filter(|&(_, &n)| n > 0)
            .map(|(day, _)| day as i32),
    })
}

/// None with class punches on fewer than [`BIOMETRIC_MIN_DAYS`] days.
pub(super) fn moves(days: &[BiometricDay]) -> Option<WrappedMoves> {
    let (mut floors, mut class_days) = (0, 0);
    let mut hops: Option<(i32, i32)> = None;
    let mut rooms: HashSet<(String, String)> = HashSet::new();
    for day in days {
        // In time order, as punches are newest first.
        let punches: Vec<ClassPunch> = day
            .punches
            .iter()
            .rev()
            .filter_map(|punch| class_punch(&punch.venue))
            .collect();
        let Some(first) = punches.first() else {
            continue;
        };
        class_days += 1;
        floors += first.floor;
        let mut changes = 0;
        for pair in punches.windows(2) {
            if let [from, to] = pair {
                floors += (to.floor - from.floor).max(0);
                changes += i32::from(to.block != from.block);
            }
        }
        // Days are in order, so the earliest keeps a tie.
        if hops.is_none_or(|(most, _)| changes > most) {
            hops = Some((changes, day.day));
        }
        rooms.extend(punches.into_iter().map(|p| (p.block, p.room)));
    }
    if class_days < BIOMETRIC_MIN_DAYS {
        return None;
    }
    let (most_hops, most_hops_day) = hops?;
    let mut blocks: Vec<WrappedBlock> = Vec::new();
    for (block, _) in &rooms {
        match blocks.iter_mut().find(|b| b.block == *block) {
            Some(b) => b.rooms += 1,
            None => blocks.push(WrappedBlock {
                block: block.clone(),
                rooms: 1,
            }),
        }
    }
    blocks.sort_by(|a, b| b.rooms.cmp(&a.rooms).then_with(|| a.block.cmp(&b.block)));
    blocks.truncate(3);
    Some(WrappedMoves {
        floors,
        rooms: rooms.len() as i32,
        most_hops,
        most_hops_day,
        blocks,
    })
}

/// "7:12 AM" for minutes since midnight.
pub(super) fn clock(minute: i32) -> String {
    let (hour, minute) = (minute / 60, minute % 60);
    let half = if hour < 12 { "AM" } else { "PM" };
    let hour = match hour % 12 {
        0 => 12,
        hour => hour,
    };
    format!("{hour}:{minute:02} {half}")
}
