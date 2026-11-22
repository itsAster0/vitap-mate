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
pub(super) const CLOSE_CALL_SPARE: i32 = 3;
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
    })
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
    Some(WrappedHabits {
        worst_weekday,
        worst_weekday_misses: worst_misses.unwrap_or(0),
        misses: misses.iter().sum(),
        early_percent: percent(early_attended, early.len() as i32),
        early_classes: early.len() as i32,
        first_slot_days: days_at(slots.iter().map(|s| s.start).min(), |c| c.start),
        last_slot_days: days_at(slots.iter().map(|s| s.end).max(), |c| c.end),
        misses_by_weekday: misses.to_vec(),
    })
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
            faculty.push((name.clone(), 1));
        }
    }
    let (room, room_minutes) = most(rooms).unzip();
    let (faculty, faculty_classes) = most(faculty).unzip();
    (room.is_some() || faculty.is_some()).then(|| WrappedPlaces {
        room,
        room_minutes: room_minutes.unwrap_or(0),
        faculty,
        faculty_classes: faculty_classes.unwrap_or(0),
    })
}
