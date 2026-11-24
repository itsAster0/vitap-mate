//! The agenda built from the weekly timetable, and each course's next class.

use std::collections::{HashMap, HashSet};

use vtop_core::types::TimetableData;

use super::{
    ClassDay, ClassKind, ClassSlot, Course, DAYS, DayClass, ExamStart, NextClass, Timetable,
    TimetableDay, credits, minutes, person_name, weekday,
};
use crate::pages::calendar::SemesterCalendar;

/// `count` days of the weekly timetable from epoch day `from_day`, shaped by
/// the academic calendar when there is one (holidays and exams have no
/// classes, labs on Lab FAT days are the lab exam, working days follow the
/// weekday they name), like the Flutter agenda.
pub(crate) fn timetable_days(
    slots: &[ClassSlot],
    from_day: i32,
    count: i32,
    calendar: Option<&SemesterCalendar>,
) -> Vec<TimetableDay> {
    let calendar = calendar.filter(|calendar| !calendar.is_empty());
    // The weekdays each course has labs on, for its Lab FAT day.
    let lab_days = |code: &str| -> Vec<i32> {
        slots
            .iter()
            .filter(|slot| slot.kind == ClassKind::Lab && slot.code == code)
            .map(|slot| slot.day)
            .collect()
    };
    let state = |slot: &ClassSlot, day: i32| match calendar {
        Some(calendar) => {
            let lab = slot.kind == ClassKind::Lab;
            calendar.class_day(
                day,
                lab,
                &if lab {
                    lab_days(&slot.code)
                } else {
                    Vec::new()
                },
            )
        }
        None => ClassDay::Held,
    };
    let schedule = |day: i32| -> Vec<&ClassSlot> {
        let weekday = calendar.map_or_else(|| weekday(day), |calendar| calendar.weekday_of(day));
        slots.iter().filter(|slot| slot.day == weekday).collect()
    };
    let last_day = calendar.and_then(SemesterCalendar::last_class_day);
    // Days from `from_day` to the last class day on which a class meets.
    let class_days: Vec<i32> = last_day.map_or_else(Vec::new, |last| {
        (from_day..=last)
            .filter(|&day| {
                schedule(day)
                    .iter()
                    .any(|slot| state(slot, day) != ClassDay::Off)
            })
            .collect()
    });
    // The day each course (code, kind) meets for the last time, looking back
    // from the last class day.
    let mut last_class: HashMap<(&str, ClassKind), i32> = HashMap::new();
    if let (Some(last), Some(first)) = (
        last_day,
        calendar.and_then(SemesterCalendar::first_class_day),
    ) {
        let courses = slots
            .iter()
            .map(|slot| (slot.code.as_str(), slot.kind))
            .collect::<HashSet<_>>()
            .len();
        let mut day = last;
        while day >= first && last_class.len() < courses {
            for slot in schedule(day) {
                if state(slot, day) == ClassDay::Held {
                    last_class
                        .entry((slot.code.as_str(), slot.kind))
                        .or_insert(day);
                }
            }
            day -= 1;
        }
    }
    (from_day..from_day + count.max(0))
        .map(|day| {
            let classes: Vec<DayClass> = schedule(day)
                .into_iter()
                .map(|slot| {
                    let state = state(slot, day);
                    DayClass {
                        slot: slot.clone(),
                        state,
                        last_class: state == ClassDay::Held
                            && last_class.get(&(slot.code.as_str(), slot.kind)) == Some(&day),
                        posted: None,
                    }
                })
                .collect();
            let (mark, title) = calendar
                .and_then(|calendar| calendar.mark_on(day))
                .map_or((None, None), |(mark, title)| (Some(mark), Some(title)));
            TimetableDay {
                day,
                holds_theory: match calendar {
                    Some(calendar) if calendar.covers(day) => calendar.holds_classes(day, false),
                    _ => !classes.is_empty(),
                },
                classes,
                mark,
                title,
                last_class_day: last_day == Some(day),
                papers: Vec::new(),
                class_days_left: (class_days.len() - class_days.partition_point(|&d| d <= day))
                    as i32,
                upcoming_exam: calendar
                    .and_then(|calendar| calendar.upcoming_exam(day))
                    .map(|(day, name)| ExamStart { name, day }),
            }
        })
        .collect()
}

/// The next class of each course from `now_unix` (local time at
/// `utc_offset_minutes`), soonest first; a class today counts once it has
/// not started yet. Without a calendar, the weekly timetable a week ahead.
/// With one, four weeks ahead by its rules, as in [`timetable_days`]:
/// holidays, exams and no-class days skipped, working days following
/// another day's order, labs stopping at the Lab FAT.
pub(crate) fn next_classes(
    slots: &[ClassSlot],
    now_unix: i64,
    utc_offset_minutes: i32,
    calendar: Option<&SemesterCalendar>,
) -> Vec<NextClass> {
    let calendar = calendar.filter(|calendar| !calendar.is_empty());
    let local = now_unix + i64::from(utc_offset_minutes) * 60;
    let today = local.div_euclid(86_400) as i32;
    let minute = (local.rem_euclid(86_400) / 60) as i32;
    let lab_days = |code: &str| -> Vec<i32> {
        slots
            .iter()
            .filter(|slot| slot.kind == ClassKind::Lab && slot.code == code)
            .map(|slot| slot.day)
            .collect()
    };
    let mut next: Vec<NextClass> = Vec::new();
    for days_ahead in 0..=if calendar.is_some() { 28 } else { 7 } {
        let day = today + days_ahead;
        let order = calendar.map_or_else(|| weekday(day), |calendar| calendar.weekday_of(day));
        let mut meeting: Vec<&ClassSlot> = slots
            .iter()
            .filter(|slot| slot.day == order && (days_ahead > 0 || slot.start > minute))
            .collect();
        meeting.sort_by_key(|slot| slot.start);
        for slot in meeting {
            if next
                .iter()
                .any(|known| known.code == slot.code && known.kind == slot.kind)
            {
                continue;
            }
            let lab = slot.kind == ClassKind::Lab;
            let held = calendar.is_none_or(|calendar| {
                calendar.class_day(
                    day,
                    lab,
                    &if lab {
                        lab_days(&slot.code)
                    } else {
                        Vec::new()
                    },
                ) == ClassDay::Held
            });
            if held {
                next.push(NextClass {
                    code: slot.code.clone(),
                    kind: slot.kind,
                    days_ahead,
                    start: slot.start,
                });
            }
        }
    }
    next.sort_by_key(|class| (class.days_ahead, class.start));
    next
}

impl From<TimetableData> for Timetable {
    fn from(data: TimetableData) -> Self {
        let mut slots: Vec<ClassSlot> = data
            .slots
            .into_iter()
            .filter_map(|slot| {
                Some(ClassSlot {
                    day: DAYS
                        .iter()
                        .position(|day| slot.day.trim().eq_ignore_ascii_case(day))?
                        as i32,
                    start: minutes(&slot.start_time)?,
                    end: minutes(&slot.end_time)?,
                    code: slot.course_code,
                    name: slot.name,
                    kind: slot.kind.into(),
                    room: slot.room_no,
                    block: slot.block,
                    slot: slot.slot,
                    faculty: person_name(&slot.faculty),
                    credits: credits(&slot.credits),
                })
            })
            .collect();
        slots.sort_by_key(|slot| (slot.day, slot.start));
        // Join back-to-back periods of the same lab into one session.
        let mut merged: Vec<ClassSlot> = Vec::with_capacity(slots.len());
        for slot in slots {
            match merged.last_mut() {
                Some(prev)
                    if prev.day == slot.day
                        && prev.kind == ClassKind::Lab
                        && slot.kind == ClassKind::Lab
                        && prev.code == slot.code =>
                {
                    prev.end = slot.end;
                    prev.slot = format!("{}+{}", prev.slot, slot.slot);
                }
                _ => merged.push(slot),
            }
        }
        let courses = data
            .courses
            .into_iter()
            .map(|course| Course {
                kind: if course.course_type.to_lowercase().contains("lab") {
                    ClassKind::Lab
                } else {
                    ClassKind::Theory
                },
                credits: credits(&course.credits),
                code: course.course_code,
                name: course.name,
            })
            .collect();
        Self {
            semester_id: data.semester_id,
            updated_at: data.update_time as i64,
            slots: merged,
            courses,
        }
    }
}
