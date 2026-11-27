//! Wrapped's mixtape: five tracks picked from the report's facts by one table of rules. Each
//! rule names its slot, priority and the fact it draws on (its [`StatKey`]); the picker fills
//! five positions from their slots, one track per fact, and breaks near ties by a hash of the
//! student and semester so the same semester always plays the same tape. Drop new rules into
//! [`RULES`].

use super::*;

/// Rules above this priority are facts; at or below it, fallbacks.
const FALLBACK: i32 = 25;
/// A mixtape needs this many facts (distinct [`StatKey`]s above [`FALLBACK`]).
pub(super) const MIXTAPE_MIN: usize = 3;
/// Candidates this close to a position's best are a tie, broken by the hash.
const TIE: i32 = 3;
/// A position left empty borrows an unused fact from any slot with this priority, or
/// [`EDGE_BORROW`] for the opener and the closer.
const BORROW: i32 = 60;
const EDGE_BORROW: i32 = 75;
/// "No skips" on the tape with a run this long.
pub(super) const NO_SKIPS: i32 = 20;

/// The fact a track draws on: one track per key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum StatKey {
    Early8Am,
    CampusClock,
    DayHalf,
    SlotSpan,
    SemSpan,
    CloseCall,
    Comeback,
    Streak,
    OnDuty,
    NeverMissed,
    WorstWeekday,
    AttOverall,
    LabTime,
    TopRoom,
    TopFaculty,
    BusiestDay,
    TotalHours,
    HostelNaps,
    MovesFloors,
    MovesHops,
    HostelOut,
    OutingsTrips,
    OutingsStay,
    Crunch,
    CatChange,
    FullMarks,
    Assessments,
    ExamsCount,
    ExamHours,
    CampusLate,
    Holidays,
    CourseLoad,
    GradesGpa,
    PrevGpa,
    Roi,
    Cgpa,
    BestMarks,
    PrevAtt,
    CampusDays,
    SemOutro,
}

impl StatKey {
    /// From hostel gates or outings: left out with Hide hostel and outings.
    fn hostel(self) -> bool {
        matches!(
            self,
            Self::HostelNaps | Self::HostelOut | Self::OutingsTrips | Self::OutingsStay
        )
    }
}

/// A matched rule's title, stat line and length in seconds.
type Track = (String, String, i32);

/// One rule: in `slot` at `priority`, the track when the facts match. `grades` rules are left
/// out with Hide grades.
pub(super) struct Rule {
    pub(super) id: &'static str,
    pub(super) slot: WrappedTrackSlot,
    pub(super) priority: i32,
    pub(super) grades: bool,
    pub(super) key: StatKey,
    pub(super) track: fn(&WrappedReport) -> Option<Track>,
}

/// The five positions in play order, each with the slots it takes tracks from.
const POSITIONS: [&[WrappedTrackSlot]; 5] = [
    &[WrappedTrackSlot::Opener],
    &[WrappedTrackSlot::Attendance],
    &[WrappedTrackSlot::Grind, WrappedTrackSlot::Campus],
    &[WrappedTrackSlot::Exams, WrappedTrackSlot::Wildcard],
    &[WrappedTrackSlot::Results, WrappedTrackSlot::Closer],
];

use StatKey as K;
use WrappedTrackSlot as S;

/// Every rule. Titles are original; stat lines carry the fact.
pub(super) const RULES: &[Rule] = &[
    // Opener.
    Rule {
        id: "op_snooze",
        slot: S::Opener,
        priority: 95,
        grades: false,
        key: K::Early8Am,
        track: |r| {
            let p = early(r).filter(|&p| p < 70)?;
            some(
                "8 AM Alarm (Snoozed Mix)",
                format!("8 AM classes: {p}%"),
                dur_pct(p),
            )
        },
    },
    Rule {
        id: "op_sunrise",
        slot: S::Opener,
        priority: 92,
        grades: false,
        key: K::Early8Am,
        track: |r| {
            let p = early(r).filter(|&p| p >= 85)?;
            some(
                "Before the First Bell",
                format!("8 AM classes: {p}%"),
                8 * 60,
            )
        },
    },
    Rule {
        id: "op_early_gate",
        slot: S::Opener,
        priority: 86,
        grades: false,
        key: K::CampusClock,
        track: |r| {
            let at = r.campus.as_ref()?.earliest_entry.as_deref()?;
            (minute_of(at)? < 7 * 60 + 30).then_some(())?;
            some(
                "Campus Before Coffee",
                format!("Earliest scan: {at}"),
                dur_clock(at)?,
            )
        },
    },
    Rule {
        id: "op_morning",
        slot: S::Opener,
        priority: 78,
        grades: false,
        key: K::DayHalf,
        track: |r| {
            let d = r
                .day_half
                .as_ref()
                .filter(|d| d.morning && d.percent >= 65)?;
            some(
                "Morning Slot Overture",
                format!("{}% of classes before noon", d.percent),
                dur_pct(d.percent),
            )
        },
    },
    Rule {
        id: "op_afternoon",
        slot: S::Opener,
        priority: 78,
        grades: false,
        key: K::DayHalf,
        track: |r| {
            let d = r
                .day_half
                .as_ref()
                .filter(|d| !d.morning && d.percent >= 65)?;
            some(
                "After-Lunch Shift",
                format!("{}% of classes from 2 PM", d.percent),
                2 * 60,
            )
        },
    },
    Rule {
        id: "op_span_days",
        slot: S::Opener,
        priority: 70,
        grades: false,
        key: K::SlotSpan,
        track: |r| {
            let h = r
                .habits
                .as_ref()
                .filter(|h| h.first_slot_days >= 12 && h.last_slot_days >= 12)?;
            some(
                "First Bell to Dusk",
                format!(
                    "{} early · {} late days",
                    h.first_slot_days, h.last_slot_days
                ),
                dur_pair(h.first_slot_days, h.last_slot_days),
            )
        },
    },
    Rule {
        id: "op_fallback",
        slot: S::Opener,
        priority: 15,
        grades: false,
        key: K::SemSpan,
        track: |r| {
            (r.class_day_count > 0).then_some(())?;
            some(
                "Day One on VTOP",
                format!(
                    "{} · {} class days",
                    plural(r.courses, "course", "courses"),
                    r.class_day_count
                ),
                dur_secs(r.class_day_count),
            )
        },
    },
    // Attendance.
    Rule {
        id: "att_under75",
        slot: S::Attendance,
        priority: 96,
        grades: false,
        key: K::CloseCall,
        track: |r| {
            let c = r.close_call.as_ref().filter(|c| c.spare.is_none())?;
            some(
                "Negotiating With 75%",
                format!(
                    "{} · {}% → {}%",
                    short_course(&c.course),
                    c.percent,
                    c.final_percent
                ),
                dur_pct(c.percent),
            )
        },
    },
    Rule {
        id: "att_wire",
        slot: S::Attendance,
        priority: 94,
        grades: false,
        key: K::CloseCall,
        track: |r| {
            let c = r.close_call.as_ref()?;
            let spare = c.spare.filter(|&s| s <= 2)?;
            some(
                &format!("Living on {}%", c.percent),
                format!("{} · {spare} to spare", short_course(&c.course)),
                dur_pct(c.percent),
            )
        },
    },
    Rule {
        id: "att_comeback",
        slot: S::Attendance,
        priority: 90,
        grades: false,
        key: K::Comeback,
        track: |r| {
            let c = r
                .comeback
                .as_ref()
                .filter(|c| c.final_percent - c.low_percent >= 8)?;
            some(
                "Second-Half Plot Twist",
                format!("{}% → {}% overall", c.low_percent, c.final_percent),
                dur_pct(c.final_percent),
            )
        },
    },
    Rule {
        id: "att_iron",
        slot: S::Attendance,
        priority: 88,
        grades: false,
        key: K::Streak,
        track: |r| {
            let s = r
                .streak
                .as_ref()
                .filter(|s| s.length >= 25 && s.total > 0 && s.length * 100 >= s.total * 15)?;
            some(
                &format!("No-Skip Run ({} Straight)", s.length),
                format!("{} classes in a row", s.length),
                dur_secs(s.length),
            )
        },
    },
    Rule {
        id: "att_od",
        slot: S::Attendance,
        priority: 85,
        grades: false,
        key: K::OnDuty,
        track: |r| {
            let o = r.on_duty.as_ref().filter(|o| o.classes >= 12)?;
            some(
                "Officially On Duty",
                format!(
                    "{} OD classes across {}",
                    o.classes,
                    plural(o.days, "day", "days")
                ),
                dur_pair(o.days, o.classes),
            )
        },
    },
    Rule {
        id: "att_untouched",
        slot: S::Attendance,
        priority: 82,
        grades: false,
        key: K::NeverMissed,
        track: |r| {
            let first = r.never_missed.first()?;
            let more = match r.never_missed.len() {
                1 => String::new(),
                n => format!(" (+{})", n - 1),
            };
            some(
                "Zero-Miss Club",
                format!("{}{more} · 100%", short_course(first)),
                dur_pct(100),
            )
        },
    },
    Rule {
        id: "att_worst_day",
        slot: S::Attendance,
        priority: 74,
        grades: false,
        key: K::WorstWeekday,
        track: |r| {
            let h = r.habits.as_ref().filter(|h| h.worst_weekday_misses >= 5)?;
            let day = weekday(h.worst_weekday?)?;
            some(
                &format!("{day} Vanishing Act"),
                format!(
                    "{} of {} misses on {day}s",
                    h.worst_weekday_misses, h.misses
                ),
                dur_pair(h.worst_weekday_misses, h.misses),
            )
        },
    },
    Rule {
        id: "att_perf_wks",
        slot: S::Attendance,
        priority: 68,
        grades: false,
        key: K::Streak,
        track: |r| {
            let weeks = r.streak.as_ref()?.perfect_weeks;
            (weeks >= 4).then_some(())?;
            some(
                "Full-Week Frequency",
                format!("{weeks} zero-miss weeks"),
                dur_pair(weeks, 0),
            )
        },
    },
    Rule {
        id: "att_regular",
        slot: S::Attendance,
        priority: 60,
        grades: false,
        key: K::AttOverall,
        track: |r| {
            let a = r.attendance.as_ref().filter(|a| a.percent >= 90)?;
            some("Front-Row Regular", overall(a), dur_pct(a.percent))
        },
    },
    Rule {
        id: "att_fallback",
        slot: S::Attendance,
        priority: 20,
        grades: false,
        key: K::AttOverall,
        track: |r| {
            let a = r.attendance.as_ref()?;
            some("Showing Up (Mostly)", overall(a), dur_pct(a.percent))
        },
    },
    // Grind.
    Rule {
        id: "gr_lab_rat",
        slot: S::Grind,
        priority: 92,
        grades: false,
        key: K::LabTime,
        track: |r| {
            let t = r
                .time
                .as_ref()
                .filter(|t| t.minutes > 0 && t.lab_minutes * 100 >= t.minutes * 35)?;
            let share = t.lab_minutes * 100 / t.minutes;
            some(
                "Lab Rat Anthem",
                format!("{} h in labs ({share}%)", hours(t.lab_minutes)),
                dur_pct(share),
            )
        },
    },
    Rule {
        id: "gr_room_rent",
        slot: S::Grind,
        priority: 75,
        grades: false,
        key: K::TopRoom,
        track: |r| {
            let p = r.places.as_ref().filter(|p| p.room_minutes >= 1500)?;
            let room = p.room.as_deref()?;
            let h = hours(p.room_minutes);
            some(
                &format!("Rent-Free in {room}"),
                format!("{h} h inside {room}"),
                dur_secs(h),
            )
        },
    },
    Rule {
        id: "gr_faculty",
        slot: S::Grind,
        priority: 70,
        grades: false,
        key: K::TopFaculty,
        track: |r| {
            let p = r.places.as_ref().filter(|p| p.faculty_minutes >= 1500)?;
            let h = hours(p.faculty_minutes);
            some(
                &format!("Co-Starring {}", short_faculty(p.faculty.as_deref()?)),
                format!("{h} h of class together"),
                dur_secs(h),
            )
        },
    },
    Rule {
        id: "gr_busy_day",
        slot: S::Grind,
        priority: 62,
        grades: false,
        key: K::BusiestDay,
        track: |r| {
            let t = r.time.as_ref()?;
            let n = *t.classes_by_weekday.get(t.busiest_weekday as usize)?;
            (n >= 40).then_some(())?;
            let day = weekday(t.busiest_weekday)?;
            some(
                &format!("{day} Heavy Rotation"),
                format!("Busiest day · {n} classes"),
                dur_secs(n),
            )
        },
    },
    Rule {
        id: "gr_hours_fb",
        slot: S::Grind,
        priority: 20,
        grades: false,
        key: K::TotalHours,
        track: |r| {
            let t = r.time.as_ref().filter(|t| t.minutes > 0)?;
            let h = hours(t.minutes);
            some(
                "Lecture Hall Static",
                format!("{h} h across {} classes", t.classes),
                dur_secs(h),
            )
        },
    },
    // Campus.
    Rule {
        id: "ca_naps",
        slot: S::Campus,
        priority: 91,
        grades: false,
        key: K::HostelNaps,
        track: |r| {
            let n = r.hostel.as_ref()?.naps;
            (n >= 5).then_some(())?;
            some(
                "Hostel Nap Interlude",
                format!("{n} pit stops between classes"),
                dur_pair(n, 30),
            )
        },
    },
    Rule {
        id: "ca_stairs",
        slot: S::Campus,
        priority: 89,
        grades: false,
        key: K::MovesFloors,
        track: |r| {
            let m = r.moves.as_ref().filter(|m| m.floors >= 150)?;
            some(
                &format!("{} Floors Up", m.floors),
                format!(
                    "{} floors climbed · {}",
                    m.floors,
                    plural(m.rooms, "room", "rooms")
                ),
                dur_secs(m.floors),
            )
        },
    },
    Rule {
        id: "ca_hops",
        slot: S::Campus,
        priority: 86,
        grades: false,
        key: K::MovesHops,
        track: |r| {
            let m = r.moves.as_ref().filter(|m| m.most_hops >= 4)?;
            let title = match m.blocks.as_slice() {
                [a, b, ..] => format!("{}-to-{} Sprint", a.block, b.block),
                _ => "Block-Hopping Tempo".to_string(),
            };
            some(
                &title,
                format!(
                    "{} block hops on {}",
                    m.most_hops,
                    day_label(m.most_hops_day)
                ),
                dur_pair(m.most_hops, m.rooms),
            )
        },
    },
    Rule {
        id: "ca_hostel_out",
        slot: S::Campus,
        priority: 84,
        grades: false,
        key: K::HostelOut,
        track: |r| {
            let t = r.hostel.as_ref()?.out_tenths;
            (t >= 85).then_some(())?;
            some(
                "Gate-to-Gate Shift",
                format!("{}.{} h a day out of the hostel", t / 10, t % 10),
                dur_pair(t / 10, (t % 10) * 6),
            )
        },
    },
    Rule {
        id: "ca_outing",
        slot: S::Campus,
        priority: 82,
        grades: false,
        key: K::OutingsTrips,
        track: |r| {
            let o = r.outings.as_ref()?;
            let trips = o.leaves + o.weekends;
            let place = titled(
                o.top_place
                    .as_deref()
                    .filter(|p| !p.trim().is_empty() && trips >= 5)?
                    .trim(),
            );
            let title = if place.chars().count() <= 10 {
                format!("Warden Approved: {place}")
            } else {
                format!("Outing Pass: {}", fit(&place, 12))
            };
            some(
                &title,
                format!(
                    "{trips} trips · {} away",
                    plural(o.days_away, "day", "days")
                ),
                dur_pair(o.leaves, o.weekends),
            )
        },
    },
    Rule {
        id: "ca_lifer",
        slot: S::Campus,
        priority: 76,
        grades: false,
        key: K::OutingsStay,
        track: |r| {
            let n = r.outings.as_ref()?.longest_stay_days;
            (n >= 45).then_some(())?;
            some(
                "No-Outing Residency",
                format!("{n} straight days on campus"),
                dur_secs(n),
            )
        },
    },
    // Exams.
    Rule {
        id: "ex_double",
        slot: S::Exams,
        priority: 94,
        grades: false,
        key: K::Crunch,
        track: |r| {
            let c = r.crunch.as_ref().filter(|c| c.most_in_a_day >= 2)?;
            some(
                "Double-Header Panic",
                format!("{} papers on {}", c.most_in_a_day, day_label(c.most_day)),
                dur_pair(c.papers, c.most_in_a_day),
            )
        },
    },
    Rule {
        id: "ex_crunch",
        slot: S::Exams,
        priority: 90,
        grades: false,
        key: K::Crunch,
        track: |r| {
            let c = r.crunch.as_ref().filter(|c| c.papers >= 4)?;
            some(
                &format!("{} Papers in 7 Days", c.papers),
                format!("Crunch week · {}", span(c.first_day, c.last_day)),
                dur_pair(c.papers, 7),
            )
        },
    },
    Rule {
        id: "ex_cat_up",
        slot: S::Exams,
        priority: 88,
        grades: true,
        key: K::CatChange,
        track: |r| {
            let c = r.marks.as_ref()?.cat_change.filter(|&c| c >= 5)?;
            some(
                "Open-Book Redemption",
                format!("+{c} pts from CAT-1 to CAT-2"),
                dur_pair(2, c),
            )
        },
    },
    Rule {
        id: "ex_full_marks",
        slot: S::Exams,
        priority: 86,
        grades: true,
        key: K::FullMarks,
        track: |r| {
            let f = r.full_marks.as_ref().filter(|f| f.count >= 3)?;
            some(
                "Full Marks (No Notes)",
                format!("{} perfect scores out of {}", f.count, f.of),
                dur_pair(f.count, f.of),
            )
        },
    },
    Rule {
        id: "ex_quizzes",
        slot: S::Exams,
        priority: 80,
        grades: false,
        key: K::Assessments,
        track: |r| {
            let a = r.assessments.as_ref()?;
            let quizzes = a
                .by_kind
                .iter()
                .find(|k| k.kind == "quizzes")
                .map_or(0, |k| k.count);
            (quizzes >= 8 || a.total >= 25).then_some(())?;
            let top = a.by_kind.first()?;
            let title = if quizzes >= 8 {
                "Quiz-Week Loop"
            } else {
                "Assessment Marathon"
            };
            some(
                title,
                format!("{} graded tasks · {} {}", a.total, top.count, top.kind),
                dur_secs(a.total),
            )
        },
    },
    Rule {
        id: "ex_cat_dip",
        slot: S::Exams,
        priority: 74,
        grades: true,
        key: K::CatChange,
        track: |r| {
            let c = r.marks.as_ref()?.cat_change.filter(|&c| c <= -5)?;
            some(
                "Open-Book Plot Twist",
                format!("CAT-1 → CAT-2: {c} pts (lore)"),
                dur_pair(2, -c),
            )
        },
    },
    Rule {
        id: "ex_venue",
        slot: S::Exams,
        priority: 72,
        grades: false,
        key: K::ExamsCount,
        track: |r| {
            let e = r.exams.as_ref().filter(|e| e.cats + e.fats >= 6)?;
            let venue = e
                .top_venue
                .as_deref()
                .filter(|v| !v.trim().is_empty() && *v != "-")?;
            some(
                &format!("Assigned Seat: {venue}"),
                format!(
                    "{} · {} sat",
                    plural(e.cats, "CAT", "CATs"),
                    plural(e.fats, "FAT", "FATs")
                ),
                dur_pair(e.cats, e.fats),
            )
        },
    },
    // Wildcard.
    Rule {
        id: "wc_exam_hrs",
        slot: S::Wildcard,
        priority: 84,
        grades: false,
        key: K::ExamHours,
        track: |r| {
            let m = r.exams.as_ref()?.minutes;
            (m >= 900).then_some(())?;
            some(
                "Under the Hall Clock",
                format!("{} h in exam halls", hours(m)),
                dur_secs(hours(m)),
            )
        },
    },
    Rule {
        id: "wc_late_scan",
        slot: S::Wildcard,
        priority: 78,
        grades: false,
        key: K::CampusLate,
        track: |r| {
            let at = r.campus.as_ref()?.latest_return.as_deref()?;
            (minute_of(at)? >= 21 * 60).then_some(())?;
            some(
                "After-Hours Gate Scan",
                format!("Latest campus scan: {at}"),
                dur_clock(at)?,
            )
        },
    },
    Rule {
        id: "wc_holidays",
        slot: S::Wildcard,
        priority: 52,
        grades: false,
        key: K::Holidays,
        track: |r| {
            (r.holidays >= 15).then_some(())?;
            some(
                "Festival Break Interlude",
                format!("{} holidays on the calendar", r.holidays),
                dur_secs(r.holidays),
            )
        },
    },
    Rule {
        id: "wc_course_fb",
        slot: S::Wildcard,
        priority: 15,
        grades: false,
        key: K::CourseLoad,
        track: |r| {
            (r.courses > 0).then_some(())?;
            some(
                "FFCS Slot-War Survivor",
                format!(
                    "{} on the timetable",
                    plural(r.courses, "course", "courses")
                ),
                dur_pair(r.courses, r.class_day_count),
            )
        },
    },
    // Results.
    Rule {
        id: "re_s_record",
        slot: S::Results,
        priority: 98,
        grades: true,
        key: K::GradesGpa,
        track: |r| {
            let g = r.grades.as_ref().filter(|g| g.s_record)?;
            some(
                "All-Time S-Grade Run",
                format!("{} S grades · personal record", g.s_count),
                dur_gpa(g.gpa),
            )
        },
    },
    Rule {
        id: "re_gpa_rank",
        slot: S::Results,
        priority: 95,
        grades: true,
        key: K::GradesGpa,
        track: |r| {
            let d = r
                .degree
                .as_ref()
                .filter(|d| d.rank <= 2 && d.semesters.len() >= 3)?;
            let gpa = d.semesters.iter().find(|s| s.current)?.gpa;
            let title = if d.rank == 1 {
                "Career-High Outro"
            } else {
                "Personal Best (Almost)"
            };
            some(
                title,
                format!("GPA #{} of {} · {gpa:.2}", d.rank, d.semesters.len()),
                dur_gpa(gpa),
            )
        },
    },
    Rule {
        id: "re_nine_pt",
        slot: S::Results,
        priority: 93,
        grades: true,
        key: K::GradesGpa,
        track: |r| {
            let g = r.grades.as_ref().filter(|g| g.gpa >= 9.0)?;
            some(
                "Nine-Point Season",
                format!(
                    "GPA {:.2} · {}",
                    g.gpa,
                    plural(g.s_count, "S grade", "S grades")
                ),
                dur_gpa(g.gpa),
            )
        },
    },
    Rule {
        id: "re_glow_up",
        slot: S::Results,
        priority: 90,
        grades: true,
        key: K::PrevGpa,
        track: |r| {
            let p = r.previous.as_ref()?;
            let change = p.gpa_change.filter(|&c| c >= 0.25)?;
            some(
                "Upgraded From Last Sem",
                format!("+{change:.2} GPA vs {}", season(&p.semester_name)),
                dur_gpa(r.grades.as_ref()?.gpa),
            )
        },
    },
    Rule {
        id: "re_roi",
        slot: S::Results,
        priority: 88,
        grades: true,
        key: K::Roi,
        track: |r| {
            let roi = r.roi.as_ref()?;
            let best = roi
                .points
                .get(roi.best as usize)
                .filter(|p| p.score - p.attendance >= 12)?;
            some(
                "Minimum Effort, Max Score",
                format!(
                    "{}: {}% att → {}",
                    short_course(&best.course),
                    best.attendance,
                    best.score
                ),
                dur_pct(best.score),
            )
        },
    },
    Rule {
        id: "re_cgpa_up",
        slot: S::Results,
        priority: 85,
        grades: true,
        key: K::Cgpa,
        track: |r| {
            let c = r.cgpa.as_ref()?;
            let before = c.before.filter(|&b| c.after - b >= 0.05)?;
            some(
                "CGPA on the Climb",
                format!(
                    "{before:.2} → {:.2} CGPA · {} cr",
                    c.after,
                    credits(c.credits_total)
                ),
                dur_gpa(c.after),
            )
        },
    },
    Rule {
        id: "re_best_crs",
        slot: S::Results,
        priority: 79,
        grades: true,
        key: K::BestMarks,
        track: |r| {
            let m = r.marks.as_ref().filter(|m| m.best_course_percent >= 80)?;
            some(
                &format!("Carry Job: {}", short_course(m.best_course.as_deref()?)),
                format!("{}% in marks", m.best_course_percent),
                dur_pct(m.best_course_percent),
            )
        },
    },
    Rule {
        id: "re_grades_fb",
        slot: S::Results,
        priority: 66,
        grades: true,
        key: K::GradesGpa,
        track: |r| {
            let g = r.grades.as_ref()?;
            let earned = r
                .cgpa
                .as_ref()
                .filter(|c| c.credits_semester > 0.0)
                .map_or_else(String::new, |c| {
                    format!(" · {} cr", credits(c.credits_semester))
                });
            some(
                "Credits in the Bank",
                format!("GPA {:.2}{earned}", g.gpa),
                dur_gpa(g.gpa),
            )
        },
    },
    // Closer.
    Rule {
        id: "cl_att_up",
        slot: S::Closer,
        priority: 72,
        grades: false,
        key: K::PrevAtt,
        track: |r| {
            let p = r.previous.as_ref()?;
            let change = p.attendance_change.filter(|&c| c >= 3)?;
            some(
                "Better Than Last Sem",
                format!("+{change}% attendance vs {}", season(&p.semester_name)),
                dur_pct(r.attendance.as_ref()?.percent),
            )
        },
    },
    Rule {
        id: "cl_campus_d",
        slot: S::Closer,
        priority: 64,
        grades: false,
        key: K::CampusDays,
        track: |r| {
            let n = r.campus.as_ref()?.days_on_campus;
            (n >= 50).then_some(())?;
            some(
                "Stamped at the Gate",
                format!("{n} days scanned on campus"),
                dur_secs(n),
            )
        },
    },
    Rule {
        id: "cl_outro_fb",
        slot: S::Closer,
        priority: 15,
        grades: false,
        key: K::SemOutro,
        track: |r| {
            let days = r.last_day - r.first_day + 1;
            (days > 0).then_some(())?;
            some(
                &format!("{} Credits Roll", stamp(r)),
                format!("{days} days wrapped"),
                dur_secs(days),
            )
        },
    },
];

/// A rule that matched.
struct Candidate {
    id: &'static str,
    slot: WrappedTrackSlot,
    priority: i32,
    key: StatKey,
    track: Track,
}

/// The tape: five tracks, one per position and fact, or none under [`MIXTAPE_MIN`] facts.
/// `regno` seeds the near-tie breaks and the cover; `grades` false leaves out the grade rules
/// (Hide grades) and `hostel` false the hostel and outing ones (Hide hostel and outings), which
/// also changes the album's title and genre.
pub(super) fn mixtape(
    report: &WrappedReport,
    regno: &str,
    grades: bool,
    hostel: bool,
) -> Option<WrappedMixtape> {
    let seed = fnv1a(
        FNV_OFFSET,
        format!("{regno}:{}", report.semester_id).as_bytes(),
    );
    let candidates: Vec<Candidate> = RULES
        .iter()
        .filter(|rule| (grades || !rule.grades) && (hostel || !rule.key.hostel()))
        .filter_map(|rule| {
            Some(Candidate {
                id: rule.id,
                slot: rule.slot,
                priority: rule.priority,
                key: rule.key,
                track: (rule.track)(report)?,
            })
        })
        .collect();
    let facts: HashSet<StatKey> = candidates
        .iter()
        .filter(|c| c.priority > FALLBACK)
        .map(|c| c.key)
        .collect();
    if facts.len() < MIXTAPE_MIN {
        return None;
    }
    let mut used: HashSet<StatKey> = HashSet::new();
    // The best candidate `allowed`, near ties broken by the hash, then rule order.
    let pick = |used: &HashSet<StatKey>, allowed: &dyn Fn(&Candidate) -> bool| {
        let open: Vec<(usize, &Candidate)> = candidates
            .iter()
            .enumerate()
            .filter(|(_, c)| !used.contains(&c.key) && allowed(c))
            .collect();
        let best = open.iter().map(|(_, c)| c.priority).max()?;
        open.into_iter()
            .filter(|(_, c)| c.priority >= best - TIE)
            .max_by_key(|&(at, c)| (fnv1a(seed, c.id.as_bytes()), std::cmp::Reverse(at)))
            .map(|(_, c)| c)
    };
    let mut picked: [Option<&Candidate>; 5] = [None; 5];
    // A: each position's own slots, facts only.
    for (spot, slots) in picked.iter_mut().zip(POSITIONS) {
        *spot = pick(&used, &|c| slots.contains(&c.slot) && c.priority > FALLBACK);
        used.extend(spot.map(|c| c.key));
    }
    // B: an empty position borrows a strong fact from any slot; the opener and closer only a
    // stronger one.
    let last = POSITIONS.len() - 1;
    for (at, spot) in picked.iter_mut().enumerate() {
        if spot.is_some() {
            continue;
        }
        let floor = if at == 0 || at == last {
            EDGE_BORROW
        } else {
            BORROW
        };
        *spot = pick(&used, &|c| c.priority >= floor);
        used.extend(spot.map(|c| c.key));
    }
    // C: its own slots' fallback, else any fallback left.
    for (spot, slots) in picked.iter_mut().zip(POSITIONS) {
        if spot.is_some() {
            continue;
        }
        *spot = pick(&used, &|c| slots.contains(&c.slot))
            .or_else(|| pick(&used, &|c| c.priority <= FALLBACK));
        used.extend(spot.map(|c| c.key));
    }
    let tracks: Vec<WrappedTrack> = picked
        .iter()
        .enumerate()
        .map(|(n, c)| {
            let c = (*c)?;
            let (title, stat_line, secs) = c.track.clone();
            Some(WrappedTrack {
                number: n as i32 + 1,
                slot: c.slot,
                title,
                stat_line,
                duration: clock_of(secs),
                duration_secs: secs,
            })
        })
        .collect::<Option<_>>()?;
    let total_secs = tracks.iter().map(|t| t.duration_secs).sum();
    let title = title(&report.traits, grades, hostel);
    Some(WrappedMixtape {
        album_title: album(report, title, seed),
        genre: genre(report, grades, hostel).to_string(),
        stamp: stamp(report),
        total_duration: clock_of(total_secs),
        total_secs,
        cover_seed: seed,
        cover_tone: tone(title),
        no_skips: report.streak.as_ref().is_some_and(|s| s.length >= NO_SKIPS),
        tracks,
    })
}

/// The album's name from the title (Hide grades' title when `grades` is off), one of two by
/// the seed.
fn album(report: &WrappedReport, title: WrappedTitle, seed: u64) -> String {
    let pair: (&str, &str) = match title {
        WrappedTitle::Comeback => ("The Comeback Cut", "75% Redemption EP"),
        WrappedTitle::IronStreak => ("No-Skip Season", "Zero-Debar Tape"),
        WrappedTitle::EarlyRegular => ("8 AM Slot Deluxe", "First Bell Sessions"),
        WrappedTitle::Topper => ("Nine-Pointer Tape", "S-Grade Sessions"),
        WrappedTitle::Explorer => ("Warden Approved EP", "Out of Gate 1"),
        WrappedTitle::LabRat => ("Lab-Coat Era", "100-Minute Loops"),
        WrappedTitle::Regular => ("Clockwork Sem", "Daily Check-In EP"),
        WrappedTitle::GhostGenius => ("Bunk and Ace EP", "Back-Bench Brilliance"),
        WrappedTitle::ExamClutch => ("Clutch Season", "CAT-2 Comeback Tape"),
        WrappedTitle::Perfectionist => ("Full Marks Only", "Out of Out EP"),
        WrappedTitle::ClubCaptain => ("On-Duty Anthems", "OD Club Mix"),
        WrappedTitle::Stairmaster => ("Ten Floors Up", "Stairwell Sessions"),
        WrappedTitle::NightOwl => ("After 2 PM", "Late Slot Sessions"),
        WrappedTitle::EdgeWalker => ("The 75% Cut", "Debar-Line Edition"),
        WrappedTitle::SteadyCruiser if report.close_call.is_some() => {
            ("The 75% Cut", "Debar-Line Edition")
        }
        WrappedTitle::SteadyCruiser => {
            return if seed.is_multiple_of(2) {
                "VTOP Receipts".to_string()
            } else {
                format!("Side A: {}", stamp(report))
            };
        }
    };
    if seed.is_multiple_of(2) {
        pair.0
    } else {
        pair.1
    }
    .to_string()
}

/// The first genre the facts match.
fn genre(r: &WrappedReport, grades: bool, hostel: bool) -> &'static str {
    let gpa = r.grades.as_ref().filter(|_| grades);
    let rules: [(bool, &'static str); 11] = [
        (gpa.is_some_and(|g| g.s_record), "Honor-Roll Synth"),
        (gpa.is_some_and(|g| g.gpa >= 9.0), "Nine-Pointer Synth"),
        (
            r.close_call
                .as_ref()
                .is_some_and(|c| c.spare.is_none_or(|s| s == 0)),
            "75% Debar-Line Punk",
        ),
        (
            r.comeback
                .as_ref()
                .is_some_and(|c| c.final_percent - c.low_percent >= 8),
            "Redemption-Arc Indie",
        ),
        (
            r.time
                .as_ref()
                .is_some_and(|t| t.minutes > 0 && t.lab_minutes * 100 >= t.minutes * 35),
            "Lab-Coat Ambient",
        ),
        (early(r).is_some_and(|p| p < 70), "8 AM FFCS-Regret Lo-Fi"),
        (
            hostel && r.hostel.as_ref().is_some_and(|h| h.naps >= 5),
            "Hostel-Nap Dream Pop",
        ),
        (
            hostel
                && r.outings
                    .as_ref()
                    .is_some_and(|o| o.leaves + o.weekends >= 6),
            "Warden-Approved Indie",
        ),
        (
            r.on_duty.as_ref().is_some_and(|o| o.classes >= 12),
            "On-Duty Club Mix",
        ),
        (
            r.crunch.as_ref().is_some_and(|c| c.papers >= 5),
            "Open-Book Panic-Pop",
        ),
        (
            r.attendance.as_ref().is_some_and(|a| a.percent >= 90),
            "Lo-Fi Attendance Beats",
        ),
    ];
    rules
        .into_iter()
        .find(|(matched, _)| *matched)
        .map_or("Campus Tape Lo-Fi", |(_, genre)| genre)
}

/// The cover's lead colour, from the title.
fn tone(title: WrappedTitle) -> WrappedMixtapeTone {
    match title {
        WrappedTitle::Comeback | WrappedTitle::GhostGenius | WrappedTitle::NightOwl => {
            WrappedMixtapeTone::Accent
        }
        WrappedTitle::IronStreak
        | WrappedTitle::EarlyRegular
        | WrappedTitle::Explorer
        | WrappedTitle::ClubCaptain => WrappedMixtapeTone::Warning,
        WrappedTitle::Topper | WrappedTitle::Regular | WrappedTitle::Perfectionist => {
            WrappedMixtapeTone::Success
        }
        WrappedTitle::LabRat | WrappedTitle::Stairmaster | WrappedTitle::ExamClutch => {
            WrappedMixtapeTone::Lab
        }
        WrappedTitle::SteadyCruiser | WrappedTitle::EdgeWalker => WrappedMixtapeTone::Danger,
    }
}

/// "FALL '26": the season and the year classes started.
pub(super) fn stamp(r: &WrappedReport) -> String {
    let season = records::Semester::new(String::new(), r.semester_name.clone()).season;
    let short = match season.to_lowercase().as_str() {
        "fall" => "FALL".to_string(),
        "winter" => "WIN".to_string(),
        "summer" => "SUM".to_string(),
        "" => "SEM".to_string(),
        other => other.chars().take(4).collect::<String>().to_uppercase(),
    };
    let date = records::civil_date(i64::from(r.first_day));
    format!("{short} '{}", date.get(2..4).unwrap_or("00"))
}

/// "Winter" of "Winter Semester 2024-25".
fn season(name: &str) -> String {
    let season = records::Semester::new(String::new(), name.to_string()).season;
    fit(&season, 16)
}

/// The 8 AM rate over 10 classes or more.
fn early(r: &WrappedReport) -> Option<i32> {
    let h = r.habits.as_ref().filter(|h| h.early_classes >= 10)?;
    h.early_percent
}

fn overall(a: &WrappedAttendance) -> String {
    format!("{}% overall · {}/{}", a.percent, a.attended, a.counted)
}

fn some(title: &str, stat: String, secs: i32) -> Option<Track> {
    Some((title.to_string(), stat, secs))
}

// Lengths: seconds that read as the fact.

/// 64% is 6:04, 100% is 10:00.
pub(super) fn dur_pct(p: i32) -> i32 {
    let p = p.clamp(0, 100);
    (p / 10) * 60 + p % 10
}

/// A GPA as decimal minutes: 9.05 is 9:03, 9.5 is 9:30.
pub(super) fn dur_gpa(gpa: f64) -> i32 {
    (gpa.clamp(0.0, 10.0) * 60.0).round() as i32
}

/// Two numbers as minutes and seconds: 5 papers in 7 days is 5:07.
pub(super) fn dur_pair(a: i32, b: i32) -> i32 {
    a.clamp(0, 19) * 60 + b.clamp(0, 59)
}

/// A count as seconds: 137 is 2:17.
pub(super) fn dur_secs(n: i32) -> i32 {
    n.clamp(35, 1199)
}

/// "7:12 AM" as 7:12; "10:42 PM" as 10:42.
pub(super) fn dur_clock(at: &str) -> Option<i32> {
    let minute = minute_of(at)?;
    let hour = match (minute / 60) % 12 {
        0 => 12,
        h => h,
    };
    Some(hour * 60 + minute % 60)
}

/// "7:12 AM" as minutes since midnight.
pub(super) fn minute_of(at: &str) -> Option<i32> {
    let (clock, half) = at.trim().split_once(' ')?;
    let (h, m) = clock.split_once(':')?;
    let (h, m): (i32, i32) = (h.parse().ok()?, m.parse().ok()?);
    if !(1..=12).contains(&h) || !(0..60).contains(&m) {
        return None;
    }
    let pm = match half {
        "AM" => false,
        "PM" => true,
        _ => return None,
    };
    Some((h % 12 + if pm { 12 } else { 0 }) * 60 + m)
}

/// Seconds as "m:ss".
pub(super) fn clock_of(secs: i32) -> String {
    let secs = secs.max(0);
    format!("{}:{:02}", secs / 60, secs % 60)
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// 64-bit FNV-1a of `bytes` from `basis`: the same on every phone and every run.
pub(super) fn fnv1a(basis: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(basis, |hash, &b| {
        (hash ^ u64::from(b)).wrapping_mul(FNV_PRIME)
    })
}

// Words.

fn hours(minutes: i32) -> i32 {
    (minutes + 30) / 60
}

fn weekday(day: i32) -> Option<&'static str> {
    [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ]
    .get(usize::try_from(day).ok()?)
    .copied()
}

/// "1 class", "3 classes".
fn plural(n: i32, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// 24.0 as "24", 24.5 as "24.5".
fn credits(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

/// `text` cut at a word boundary to `max` characters (or the first word's first `max`).
pub(super) fn fit(text: &str, max: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out = String::new();
    for word in text.split_whitespace() {
        let next = if out.is_empty() {
            word.to_string()
        } else {
            format!("{out} {word}")
        };
        if next.chars().count() > max {
            break;
        }
        out = next;
    }
    if out.is_empty() {
        text.chars().take(max).collect()
    } else {
        out
    }
}

/// A course name short enough for a track: "Introduction to " and "Fundamentals of " off the
/// front, " and its Applications" off the end, then cut to 20 characters without a dangling
/// "and" or "of".
pub(super) fn short_course(name: &str) -> String {
    let name = titled(name.trim());
    let mut rest = name.as_str();
    for prefix in [
        "Introduction to ",
        "Introduction To ",
        "Fundamentals of ",
        "Fundamentals Of ",
    ] {
        rest = rest.strip_prefix(prefix).unwrap_or(rest);
    }
    for suffix in [
        " and its Applications",
        " And Its Applications",
        " and Its Applications",
    ] {
        rest = rest.strip_suffix(suffix).unwrap_or(rest);
    }
    let cut = fit(rest, 20);
    let mut words: Vec<&str> = cut.split_whitespace().collect();
    while words.len() > 1
        && words.last().is_some_and(|w| {
            ["and", "of", "to", "the", "for", "in", "&"].contains(&w.to_lowercase().as_str())
        })
    {
        words.pop();
    }
    words.join(" ")
}

/// A faculty's name for a title: VTOP's "NAME - SCHOOL" without the school, then the last
/// words that fit 14 characters.
pub(super) fn short_faculty(name: &str) -> String {
    let name = titled(name.split(" - ").next().unwrap_or(name).trim());
    let words: Vec<&str> = name.split_whitespace().collect();
    let mut out = String::new();
    for word in words.iter().rev() {
        let next = if out.is_empty() {
            (*word).to_string()
        } else {
            format!("{word} {out}")
        };
        if next.chars().count() > 14 {
            break;
        }
        out = next;
    }
    if out.is_empty() { fit(&name, 14) } else { out }
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// "21 Nov".
fn day_label(day: i32) -> String {
    let (d, m) = day_parts(day);
    format!("{d} {m}")
}

/// "18 to 24 Nov", or "28 Nov to 4 Dec" across months.
pub(super) fn span(from: i32, to: i32) -> String {
    let (a, b) = (day_parts(from), day_parts(to));
    if a.1 == b.1 {
        format!("{} to {} {}", a.0, b.0, b.1)
    } else {
        format!("{} {} to {} {}", a.0, a.1, b.0, b.1)
    }
}

/// The day of the month and the month's short name.
fn day_parts(day: i32) -> (i32, &'static str) {
    let date = records::civil_date(i64::from(day));
    let month: usize = date.get(5..7).and_then(|m| m.parse().ok()).unwrap_or(1);
    let of_month = date.get(8..10).and_then(|d| d.parse().ok()).unwrap_or(1);
    let name = month.checked_sub(1).and_then(|m| MONTHS.get(m)).copied();
    (of_month, name.unwrap_or("Jan"))
}

/// "GUNTUR" as "Guntur"; text with lower case letters stays.
pub(super) fn titled(text: &str) -> String {
    if text.chars().any(char::is_lowercase) {
        return text.to_string();
    }
    text.to_lowercase()
        .split(' ')
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}
