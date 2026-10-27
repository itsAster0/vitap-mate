//! MCP prompts: ready-made requests that clients offer as slash commands.
//! Each is text only, telling the agent which tools to use; none calls VTOP
//! itself.

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{PromptMessage, Role};
use rmcp::{prompt, prompt_router, schemars};
use serde::Deserialize;

use crate::tools::VtopTools;

#[derive(Deserialize, schemars::JsonSchema)]
pub struct SkipArgs {
    #[schemars(
        description = "The day, e.g. tomorrow, Friday or 14/11/2026. Defaults to tomorrow."
    )]
    pub day: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct CourseArgs {
    #[schemars(
        description = "Course code or title, e.g. CSE3005 or Compiler Design. Leave out for every course."
    )]
    pub course: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
pub struct OutingArgs {
    #[schemars(description = "general (a leave) or weekend. Ask the student when left out.")]
    pub kind: Option<String>,
}

fn user(text: impl Into<String>) -> Vec<PromptMessage> {
    vec![PromptMessage::new_text(Role::User, text.into())]
}

/// `value`, or `fallback` when it is missing or blank.
fn or(value: Option<String>, fallback: &str) -> String {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

#[prompt_router(vis = "pub(crate)")]
impl VtopTools {
    #[prompt(
        name = "weekly-briefing",
        description = "The week ahead: classes each day, attendance at risk, exams, deadlines and mentor messages."
    )]
    async fn weekly_briefing(&self) -> Vec<PromptMessage> {
        user(
            "Give me my VTOP briefing for the next 7 days, starting today (India time).

1. Classes: for each day, my classes from get_timetable, after checking \
get_academic_calendar for holidays, exam days, lab FATs and day-order changes. Say \
plainly when a day has no classes.
2. Attendance: from get_attendance, every course under 80%, with how many more \
classes I must attend to reach 75%, or how many I can still miss and stay at 75%.
3. Exams: anything in get_exam_schedule or marked in get_academic_calendar within the \
next 14 days, with date, time, venue and seat.
4. Mentor: messages from get_mentor this week, if any.
5. Outings: pending or accepted requests from get_general_outing and \
get_weekend_outing that fall in this week.

Keep it short: a few lines per day, then the rest as bullets. Leave out a section \
with nothing in it.",
        )
    }

    #[prompt(
        name = "can-i-skip",
        description = "Whether missing a day's classes keeps every course at 75% attendance."
    )]
    async fn can_i_skip(&self, Parameters(args): Parameters<SkipArgs>) -> Vec<PromptMessage> {
        let day = or(args.day, "tomorrow");
        user(format!(
            "Can I skip my classes {day}?

1. Work out the date in India time, then check get_academic_calendar: if it is a \
holiday or there are no classes, say so and stop.
2. List that day's classes from get_timetable. A lab session (two L slots) counts as \
one class.
3. For each of those courses, from get_attendance, take attended and total classes \
and work out the percentage after missing that day's classes (total goes up, attended \
does not). VTOP rounds, so 74.5% counts as 75%.
4. Answer per course: safe to miss, or not, with the percentage before and after and \
how many classes of margin are left. Warn if the next CAT or FAT is close, since \
attendance is checked before each exam.

Then give a one-line answer: skip all, skip some (which), or skip none."
        ))
    }

    #[prompt(
        name = "marks-needed",
        description = "What the student needs in the FAT to pass and to reach each grade."
    )]
    async fn marks_needed(&self, Parameters(args): Parameters<CourseArgs>) -> Vec<PromptMessage> {
        let course = or(args.course, "each of my current courses");
        user(format!(
            "What do I need in the FAT for {course}?

1. Use the current semester from get_semesters, and get_marks for the weighted marks \
I have so far and the FAT's weightage.
2. Passing needs at least 40% in the FAT itself and enough in total; say the FAT \
mark needed to pass.
3. If get_grades has grades for the course, use get_grade_details for the grade \
ranges and say the FAT mark needed for each grade. Otherwise say grading is usually \
relative, so cutoffs are only known later, and give what each total out of 100 \
needs instead (e.g. 90, 80, 70, 60, 50).
4. Show the FAT mark both as out of the FAT's maximum and as weighted marks. Say when \
a grade is out of reach.

Keep it as a short table per course."
        ))
    }

    #[prompt(
        name = "exam-prep",
        description = "The next exam for a course or all courses, what it covers, and a study plan up to it."
    )]
    async fn exam_prep(&self, Parameters(args): Parameters<CourseArgs>) -> Vec<PromptMessage> {
        let course = or(args.course, "my courses");
        user(format!(
            "Help me prepare for my next exam in {course}.

1. Find the next exam from get_exam_schedule (CATs and theory FATs) and \
get_academic_calendar (lab FATs), with date, time and venue.
2. Work out what it covers: CAT-1 and CAT-2 cover about two modules each, the FAT the \
full syllabus. Use get_courses, get_course_classes and get_course_detail for the \
lecture plan and topics taught so far; get_course_file can fetch the syllabus or \
lecture materials if I ask.
3. From get_marks, say how I did in earlier assessments of the course.
4. Make a day-by-day study plan from today up to the exam, fitted around my classes \
from get_timetable."
        ))
    }

    #[prompt(
        name = "apply-outing",
        description = "Apply for a general or weekend outing, step by step, confirming before anything is sent."
    )]
    async fn apply_outing(&self, Parameters(args): Parameters<OutingArgs>) -> Vec<PromptMessage> {
        let kind = or(args.kind, "an outing (ask me whether general or weekend)");
        user(format!(
            "I want to apply for {kind}.

1. Load the form with get_general_outing or get_weekend_outing. If VTOP is not taking \
applications, tell me its notice and stop. Mention any request of mine already \
pending.
2. Ask me for what is missing, offering only what the form allows: for a general \
outing, place, purpose, leaving and return dates and times within the allowed hours; \
for a weekend outing, a place and time slot from the form's options, the date, \
purpose and my contact number.
3. Call the apply tool without confirmation and show me the request it returns, \
word for word, with the weekday of each date.
4. Send it with the confirmation code only after I say yes. If I change anything, \
check it again first. Then tell me what VTOP answered."
        ))
    }
}
