//! Wrapped's class slides: attendance, on duty, streaks, habits, time and places.

use super::*;

/// One period of class, in minutes; a lab session is two.
pub(super) const PERIOD_MINUTES: i32 = 50;
/// The comeback counts from this posted class on, and needs this many points.
pub(super) const COMEBACK_FROM: usize = 20;
pub(super) const COMEBACK_POINTS: i32 = 5;
/// The close call counts from a course's 10th counted class (in VTOP's units) on, and is
/// skipped with more than this many classes to spare.
pub(super) const CLOSE_CALL_FROM: i32 = 10;
pub(super) const CLOSE_CALL_SPARE: i32 = 1;
/// The Streak slide needs a run this long.
pub(super) const STREAK_MIN: i32 = 5;
/// The trap slide needs this many absences VTOP put down to a missing punch.
pub(super) const TRAPS_MIN: i32 = 3;
/// A class punch from this many minutes before a class to its end was "punched in".
pub(super) const PUNCH_EARLY: i32 = 15;
/// The guess's ranges: four of this many points, the last open-ended.
pub(super) const GUESS_WIDTH: i32 = 5;
/// VTOP's attendance floor, in percent.
pub(super) const ATTENDANCE_FLOOR: i32 = 75;
/// "8 AM" classes start before this minute.
pub(super) const EARLY_BEFORE: i32 = 9 * 60;
/// Courses on the Courses slide's podium.
pub(super) const TOP_COURSES: usize = 3;
/// The On Duty slide needs this many classes on duty.
pub(super) const ON_DUTY_MIN: i32 = 10;
/// Before noon, and from 2 PM; a side needs this share (percent) of the classes.
pub(super) const NOON: i32 = 12 * 60;
pub(super) const AFTERNOON: i32 = 14 * 60;
pub(super) const DAY_HALF_PERCENT: i32 = 60;
/// The day half's bars: hours from 8.
pub(super) const FIRST_HOUR: i32 = 8;
pub(super) const HOURS: usize = 11;
/// A weekday needs this many class days for the quietest weekday.
pub(super) const QUIET_MIN_DAYS: i32 = 3;

pub(super) fn attendance(
    courses: &[AttendanceCourse],
    classes: &[Taken],
) -> Option<WrappedAttendance> {
    let (percent, attended, counted) = pooled(courses)?;
    let held = || courses.iter().filter(|course| course.counted > 0);
    let least = held().min_by(|a, b| a.percent.total_cmp(&b.percent))?;
    // Best first, more attended classes first among equals; a course's other part (theory,
    // lab) and the least-attended course left out.
    let mut ranked: Vec<&AttendanceCourse> = held().collect();
    ranked.sort_by(|a, b| {
        b.percent
            .total_cmp(&a.percent)
            .then(b.attended.cmp(&a.attended))
    });
    let mut top: Vec<WrappedCourse> = Vec::new();
    for course in ranked {
        if top.len() < TOP_COURSES
            && course.name != least.name
            && !top.iter().any(|c| c.name == course.name)
        {
            top.push(WrappedCourse {
                name: course.name.clone(),
                percent: course.percent.ceil() as i32,
            });
        }
    }
    let (best, best_percent) = top.first().map_or_else(
        || (least.name.clone(), least.percent.ceil() as i32),
        |c| (c.name.clone(), c.percent),
    );
    Some(WrappedAttendance {
        percent,
        attended,
        counted,
        on_duty: classes
            .iter()
            .filter(|(_, class)| class.status == Some(ClassStatus::OnDuty))
            .map(|(course, _)| periods(course))
            .sum(),
        best,
        best_percent,
        least: least.name.clone(),
        least_percent: least.percent.ceil() as i32,
        top,
        guesses: guesses(percent),
        lines: held()
            .map(|course| WrappedCourseLine {
                name: course.name.clone(),
                lab: course.kind == ClassKind::Lab,
                attended: course.attended,
                percent: super::percent(course.attended, course.counted).unwrap_or(0),
            })
            .collect(),
    })
}

/// Four ranges of [`GUESS_WIDTH`] points around `percent`: it falls in the second or third (by
/// its parity, so it isn't always the same pill), moved down to end at 100, with a top range
/// from 90 or more open-ended.
pub(super) fn guesses(percent: i32) -> Vec<WrappedGuess> {
    let percent = percent.clamp(0, 100);
    let holds = (percent.min(99) / GUESS_WIDTH) * GUESS_WIDTH;
    let first = (holds - GUESS_WIDTH * (1 + percent % 2)).clamp(0, 100 - GUESS_WIDTH * 4);
    (0..4)
        .map(|at| {
            let from = first + GUESS_WIDTH * at;
            let to = (at < 3 || from < 90).then_some(from + GUESS_WIDTH - 1);
            let top = to.unwrap_or(100);
            let off = if percent < from {
                percent - from
            } else if percent > top {
                percent - top
            } else {
                0
            };
            WrappedGuess { from, to, off }
        })
        .collect()
}

/// None under [`ON_DUTY_MIN`] classes on duty.
pub(super) fn on_duty(classes: &[Taken]) -> Option<WrappedOnDuty> {
    let with = |status: ClassStatus| {
        classes
            .iter()
            .filter(move |(_, class)| class.status == Some(status))
    };
    let units = |status| with(status).map(|(course, _)| periods(course)).sum::<i32>();
    let od = units(ClassStatus::OnDuty);
    if od < ON_DUTY_MIN {
        return None;
    }
    // By course name, so a course's theory and lab count together; the first seen wins a tie.
    let mut courses =
        sums(with(ClassStatus::OnDuty).map(|(course, _)| (course.name.clone(), periods(course))));
    courses.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    let most = courses.first()?.1;
    Some(WrappedOnDuty {
        classes: od,
        days: with(ClassStatus::OnDuty)
            .map(|(_, class)| class.day)
            .collect::<HashSet<_>>()
            .len() as i32,
        present: units(ClassStatus::Present),
        absent: units(ClassStatus::Absent),
        top: courses
            .into_iter()
            .take(2)
            .take_while(|&(_, n)| n == most)
            .map(|(name, percent)| WrappedCourse { name, percent })
            .collect(),
    })
}

pub(super) fn streak(classes: &[Taken]) -> Option<WrappedStreak> {
    let runs: Vec<_> = classes
        .iter()
        .map(|(_, class)| (class.day, class.start, class.status))
        .collect();
    let (length, first_day, last_day) = records::longest_run(&runs)?;
    if length < STREAK_MIN {
        return None;
    }
    // Where the run sits among the posted classes, in time order: the first window of `length`
    // attended classes from its first day to its last, as `longest_run` keeps the first.
    let mut posted: Vec<&AttendedClass> = classes
        .iter()
        .map(|(_, class)| class)
        .filter(|class| is_counted(class))
        .collect();
    posted.sort_by_key(|class| (class.day, class.start));
    let offset = (0..posted.len())
        .find(|&at| {
            posted.get(at..at + length as usize).is_some_and(|run| {
                run.iter().all(|class| is_attended(class))
                    && run.first().is_some_and(|class| class.day == first_day)
                    && run.last().is_some_and(|class| class.day == last_day)
            })
        })
        .unwrap_or(0);
    // Each week (by its Monday) with a posted class: whether it had no absence.
    let mut weeks: HashMap<i32, bool> = HashMap::new();
    for (_, class) in classes {
        let Some(status) = class.status else {
            continue;
        };
        let perfect = weeks
            .entry(class.day - records::weekday(class.day))
            .or_insert(true);
        *perfect &= status != ClassStatus::Absent;
    }
    Some(WrappedStreak {
        length,
        first_day,
        last_day,
        perfect_weeks: weeks.values().filter(|&&perfect| perfect).count() as i32,
        offset: offset as i32,
        total: posted.len() as i32,
    })
}

/// Courses with a counted class and no absence in any part (theory, lab) of the same code.
pub(super) fn never_missed(courses: &[AttendanceCourse]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for course in courses {
        let mut parts = courses.iter().filter(|part| part.code == course.code);
        if parts.clone().any(|part| part.counted > 0)
            && parts.all(|part| part.attended >= part.counted)
            && !names.contains(&course.name)
        {
            names.push(course.name.clone());
        }
    }
    names
}

pub(super) fn comeback(classes: &[Taken]) -> Option<WrappedComeback> {
    let mut posted: Vec<&Taken> = classes
        .iter()
        .filter(|(_, class)| is_counted(class))
        .collect();
    posted.sort_by_key(|(_, class)| (class.day, class.start));
    let (mut attended, mut counted) = (0, 0);
    let (mut low, mut last): (Option<(i32, i32)>, Option<i32>) = (None, None);
    for (n, (course, class)) in posted.into_iter().enumerate() {
        counted += periods(course);
        if is_attended(class) {
            attended += periods(course);
        }
        if n + 1 < COMEBACK_FROM {
            continue;
        }
        let now = percent(attended, counted)?;
        if low.is_none_or(|(lowest, _)| now < lowest) {
            low = Some((now, class.day));
        }
        last = Some(now);
    }
    let ((low_percent, low_day), final_percent) = (low?, last?);
    (final_percent - low_percent >= COMEBACK_POINTS).then_some(WrappedComeback {
        low_percent,
        low_day,
        final_percent,
    })
}

/// Per course in class order, from its [`CLOSE_CALL_FROM`]th counted class and first absence on:
/// the moment with the least to spare (under 75% first, the lowest of those first), the earliest of equals.
pub(super) fn close_call(classes: &[Taken]) -> Option<WrappedCloseCall> {
    let mut posted: Vec<&Taken> = classes
        .iter()
        .filter(|(_, class)| is_counted(class))
        .collect();
    posted.sort_by_key(|(course, class)| (course.key.as_str(), class.day, class.start));
    let mut best: Option<((bool, i32, i32), WrappedCloseCall)> = None;
    for group in posted.chunk_by(|a, b| a.0.key == b.0.key) {
        let (mut attended, mut counted) = (0, 0);
        let mut low: Option<((bool, i32, i32), WrappedCloseCall)> = None;
        for (course, class) in group {
            let unit = periods(course);
            counted += unit;
            if is_attended(class) {
                attended += unit;
            }
            // Before a course's first absence there's no close call: a full course is always
            // 3 to spare at its 10th class.
            let Some(now) = percent(attended, counted)
                .filter(|_| counted >= CLOSE_CALL_FROM && attended < counted)
            else {
                continue;
            };
            let spare = (now >= ATTENDANCE_FLOOR).then(|| {
                (1..)
                    .take_while(|k| {
                        percent(attended, counted + k * unit).is_some_and(|p| p >= ATTENDANCE_FLOOR)
                    })
                    .count() as i32
            });
            let rank = (spare.is_some(), spare.unwrap_or(now), class.day);
            if low.as_ref().is_none_or(|(lowest, _)| rank < *lowest) {
                let call = WrappedCloseCall {
                    course: course.name.clone(),
                    day: class.day,
                    spare,
                    percent: now,
                    final_percent: 0,
                };
                low = Some((rank, call));
            }
        }
        if let (Some((rank, mut call)), Some(last)) = (low, percent(attended, counted)) {
            call.final_percent = last;
            if best.as_ref().is_none_or(|(lowest, _)| rank < *lowest) {
                best = Some((rank, call));
            }
        }
    }
    best.map(|(_, call)| call)
        .filter(|call| call.spare.is_none_or(|spare| spare <= CLOSE_CALL_SPARE))
}

/// None under [`TRAPS_MIN`] absences with VTOP's missing-punch remark. `days` are the cached
/// complete biometric copies, to see whether a class punch came in time after all.
pub(super) fn traps(classes: &[Taken], days: &[BiometricDay]) -> Option<WrappedTraps> {
    let absent = || {
        classes
            .iter()
            .filter(|(_, class)| class.status == Some(ClassStatus::Absent))
    };
    let trapped: Vec<&Taken> = absent()
        .filter(|(_, class)| {
            let remark = class.remark.to_lowercase();
            remark.contains("no punch") || remark.contains("faculty punch")
        })
        .collect();
    if (trapped.len() as i32) < TRAPS_MIN {
        return None;
    }
    let (mut checked, mut punched) = (0, 0);
    for (_, class) in &trapped {
        let Some(day) = days.iter().find(|day| day.day == class.day) else {
            continue;
        };
        checked += 1;
        let (Some(start), Some(end)) = (class.start, class.end) else {
            continue;
        };
        punched += i32::from(day.punches.iter().any(|punch| {
            punch
                .minute
                .is_some_and(|minute| (start - PUNCH_EARLY..=end).contains(&minute))
                && class_punch(&punch.venue).is_some()
        }));
    }
    let mut by_course = sums(trapped.iter().map(|(course, _)| (course.name.clone(), 1)));
    by_course.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    Some(WrappedTraps {
        trapped: trapped.len() as i32,
        absences: absent().count() as i32,
        checked,
        punched,
        by_course: by_course
            .into_iter()
            .take(3)
            .map(|(name, percent)| WrappedCourse { name, percent })
            .collect(),
    })
}

pub(super) fn habits(classes: &[Taken], slots: &[ClassSlot]) -> Option<WrappedHabits> {
    let posted: Vec<&AttendedClass> = classes
        .iter()
        .map(|(_, class)| class)
        .filter(|class| is_counted(class))
        .collect();
    if posted.is_empty() {
        return None;
    }
    let mut misses = [0; 7];
    for class in posted
        .iter()
        .filter(|class| class.status == Some(ClassStatus::Absent))
    {
        if let Some(count) = misses.get_mut(class.weekday as usize) {
            *count += 1;
        }
    }
    let (worst_weekday, worst_misses) = misses
        .iter()
        .enumerate()
        .max_by_key(|&(day, &n)| (n, std::cmp::Reverse(day)))
        .filter(|&(_, &n)| n > 0)
        .map(|(day, &n)| (day as i32, n))
        .unzip();
    let early: Vec<&&AttendedClass> = posted
        .iter()
        .filter(|class| class.start.is_some_and(|start| start < EARLY_BEFORE))
        .collect();
    let early_attended = early.iter().filter(|class| is_attended(class)).count() as i32;
    // Days with an attended class at the timetable's first start (or last end).
    let days_at = |edge: Option<i32>, at: fn(&AttendedClass) -> Option<i32>| {
        edge.map_or(0, |edge| {
            posted
                .iter()
                .filter(|class| is_attended(class) && at(class) == Some(edge))
                .map(|class| class.day)
                .collect::<HashSet<_>>()
                .len() as i32
        })
    };
    let (classes_per_day_tenths, quietest_weekday) = quietest(&posted);
    Some(WrappedHabits {
        classes_per_day_tenths,
        quietest_weekday,
        worst_weekday,
        worst_weekday_misses: worst_misses.unwrap_or(0),
        misses: misses.iter().sum(),
        early_percent: percent(early_attended, early.len() as i32),
        early_classes: early.len() as i32,
        early_attended,
        first_slot_days: days_at(slots.iter().map(|s| s.start).min(), |c| c.start),
        last_slot_days: days_at(slots.iter().map(|s| s.end).max(), |c| c.end),
        misses_by_weekday: misses.to_vec(),
    })
}

/// Attended classes (a lab session once) on an average class day of each weekday in tenths,
/// Monday first, and the weekday with the fewest (the earliest of equals); weekdays with fewer
/// than [`QUIET_MIN_DAYS`] class days are 0 and left out.
fn quietest(posted: &[&AttendedClass]) -> (Vec<i32>, Option<i32>) {
    // Each weekday's class days and attended classes.
    let mut weekdays: [(HashSet<i32>, i32); 7] = Default::default();
    for class in posted {
        let Some((days, attended)) = usize::try_from(class.weekday)
            .ok()
            .and_then(|at| weekdays.get_mut(at))
        else {
            continue;
        };
        days.insert(class.day);
        if is_attended(class) {
            *attended += 1;
        }
    }
    let average = |(days, attended): &(HashSet<i32>, i32)| {
        let n = days.len() as i32;
        (n >= QUIET_MIN_DAYS).then(|| (attended * 10 + n / 2) / n)
    };
    let averages: Vec<Option<i32>> = weekdays.iter().map(average).collect();
    let held: Vec<(usize, i32)> = averages
        .iter()
        .enumerate()
        .filter_map(|(at, n)| Some((at, (*n)?)))
        .collect();
    let quietest = (held.len() >= 2)
        .then(|| {
            held.iter()
                .min_by_key(|&&(at, n)| (n, at))
                .map(|&(at, _)| at as i32)
        })
        .flatten();
    (averages.iter().map(|n| n.unwrap_or(0)).collect(), quietest)
}

/// None when neither side has [`DAY_HALF_PERCENT`] of the classes.
pub(super) fn day_half(classes: &[Taken]) -> Option<WrappedDayHalf> {
    let starts: Vec<i32> = classes
        .iter()
        .filter(|(_, class)| is_attended(class))
        .filter_map(|(_, class)| class.start)
        .collect();
    let side = |on: fn(i32) -> bool| starts.iter().filter(|&&start| on(start)).count() as i32;
    let (before, after) = (side(|s| s < NOON), side(|s| s >= AFTERNOON));
    let total = starts.len() as i32;
    let classes = before.max(after);
    let percent = (total > 0).then(|| classes * 100 / total)?;
    if percent < DAY_HALF_PERCENT {
        return None;
    }
    let mut by_hour = vec![0; HOURS];
    for start in starts {
        if let Some(n) = usize::try_from(start / 60 - FIRST_HOUR)
            .ok()
            .and_then(|hour| by_hour.get_mut(hour))
        {
            *n += 1;
        }
    }
    Some(WrappedDayHalf {
        morning: before > after,
        percent,
        classes,
        total,
        by_hour,
    })
}

pub(super) fn time(classes: &[Taken]) -> Option<WrappedTime> {
    let (mut count, mut minutes, mut lab_minutes) = (0, 0, 0);
    let (mut by_weekday, mut classes_by_weekday) = ([0; 7], [0; 7]);
    for (course, class) in classes.iter().filter(|(_, class)| is_attended(class)) {
        let length = periods(course) * PERIOD_MINUTES;
        count += 1;
        minutes += length;
        if course.kind == ClassKind::Lab {
            lab_minutes += length;
        }
        if let Some(day) = by_weekday.get_mut(class.weekday as usize) {
            *day += length;
        }
        if let Some(day) = classes_by_weekday.get_mut(class.weekday as usize) {
            *day += periods(course);
        }
    }
    (count > 0).then(|| WrappedTime {
        classes: count,
        minutes,
        lab_minutes,
        busiest_weekday: by_weekday
            .iter()
            .enumerate()
            .max_by_key(|&(day, &m)| (m, std::cmp::Reverse(day)))
            .map_or(0, |(day, _)| day as i32),
        classes_by_weekday: classes_by_weekday.to_vec(),
    })
}

/// "AB1-205" from the block and room.
pub(super) fn room_label(slot: &ClassSlot) -> String {
    match (slot.block.trim(), slot.room.trim()) {
        ("", room) => room.to_string(),
        (block, "") => block.to_string(),
        (block, room) if room.starts_with(block) => room.to_string(),
        (block, room) => format!("{block}-{room}"),
    }
}

pub(super) fn places(classes: &[Taken], slots: &[ClassSlot]) -> Option<WrappedPlaces> {
    let (mut rooms, mut faculty) = (Vec::new(), Vec::new());
    for (course, class) in classes.iter().filter(|(_, class)| is_attended(class)) {
        let mine = || {
            slots
                .iter()
                .filter(|slot| slot.code == course.code && slot.kind == course.kind)
        };
        let Some(slot) = mine().find(|slot| slot.slot == class.slot).or_else(|| {
            mine().find(|slot| slot.day == class.weekday && Some(slot.start) == class.start)
        }) else {
            continue;
        };
        let room = room_label(slot);
        if !room.is_empty() {
            rooms.push((room, slot.end - slot.start));
        }
        let name = if slot.faculty.is_empty() {
            &course.faculty
        } else {
            &slot.faculty
        };
        if !name.is_empty() {
            faculty.push((name.clone(), slot.end - slot.start));
        }
    }
    let (room, room_minutes) = most(rooms).unzip();
    let (faculty, faculty_minutes) = most(faculty).unzip();
    (room.is_some() || faculty.is_some()).then(|| WrappedPlaces {
        room,
        room_minutes: room_minutes.unwrap_or(0),
        faculty,
        faculty_minutes: faculty_minutes.unwrap_or(0),
    })
}
