//! The requests for each signed-in VTOP page, byte for byte what the old
//! client sent, and machines for the pages that take more than one request.
//! Each `fn` returns the [`Request`]; run it with [`OnePost`] and parse the
//! page with [`crate::parser`].
//!
//! [`OnePost`]: crate::machine::OnePost

use crate::error::VtopResult;
use crate::http::{Request, Response, url_encode};
use crate::inputs::{BiometricDate, CourseId, CourseType, SemesterId};
use crate::machine::{Machine, Signed, Step, signed_page};
use crate::now_unix;
use crate::parser::calendar;
use crate::types::{AcademicCalendarData, CalendarEntry};

/// The calendar's "All Class Group (Combined)" option, which VTOP selects
/// by default.
const CALENDAR_CLASS_GROUP: &str = "COMB";

pub fn semesters(s: &Signed) -> Request {
    menu(s, "/vtop/academics/common/StudentTimeTable")
}

pub fn timetable(s: &Signed, semester: &SemesterId) -> Request {
    semester_post(s, "/vtop/processViewTimeTable", semester)
}

pub fn attendance(s: &Signed, semester: &SemesterId) -> Request {
    semester_post(s, "/vtop/processViewStudentAttendance", semester)
}

fn semester_post(s: &Signed, path: &str, semester: &SemesterId) -> Request {
    Request::post(
        s.url(path),
        format!(
            "_csrf={}&semesterSubId={}&authorizedID={}",
            s.csrf,
            semester.as_str(),
            s.registration_number
        ),
    )
}

pub fn full_attendance(
    s: &Signed,
    semester: &SemesterId,
    course: &CourseId,
    course_type: &CourseType,
) -> Request {
    Request::post(
        s.url("/vtop/processViewAttendanceDetail"),
        format!(
            "_csrf={}&semesterSubId={}&registerNumber={}&courseId={}&courseType={}&authorizedID={}",
            s.csrf,
            semester.as_str(),
            s.registration_number,
            course.as_str(),
            course_type.as_str(),
            s.registration_number
        ),
    )
}

/// The semester's calendar page: the class groups and one button per month.
pub fn calendar_months(s: &Signed, semester: &SemesterId) -> Request {
    Request::post(
        s.url("/vtop/getDateForSemesterPreview"),
        format!(
            "_csrf={}&paramReturnId=getDateForSemesterPreview&semSubId={}&authorizedID={}",
            s.csrf,
            semester.as_str(),
            s.registration_number
        ),
    )
}

/// One month of the calendar; `cal_date` is a `01-OCT-2026` from
/// [`calendar::parse_calendar_months`].
pub fn calendar_month(s: &Signed, semester: &SemesterId, cal_date: &str) -> Request {
    Request::post(
        s.url("/vtop/processViewCalendar"),
        format!(
            "_csrf={}&calDate={}&semSubId={}&classGroupId={}&authorizedID={}",
            s.csrf,
            url_encode(cal_date),
            semester.as_str(),
            CALENDAR_CLASS_GROUP,
            s.registration_number
        ),
    )
}

/// A menu page (`verifyMenu=true`), as the side menu opens it.
fn menu(s: &Signed, path: &str) -> Request {
    Request::post(
        s.url(path),
        format!(
            "verifyMenu=true&authorizedID={}&_csrf={}&nocache=@(new Date().getTime())",
            s.registration_number, s.csrf,
        ),
    )
}

pub fn profile(s: &Signed) -> Request {
    menu(s, "/vtop/studentsRecord/StudentProfileAllView")
}

pub fn mentor_details(s: &Signed) -> Request {
    menu(s, "/vtop/proctor/viewProctorDetails")
}

pub fn mentor_messages(s: &Signed) -> Request {
    menu(s, "/vtop/proctor/viewMessagesSendByProctor")
}

pub fn grade_history(s: &Signed) -> Request {
    menu(s, "/vtop/examinations/examGradeView/StudentGradeHistory")
}

/// An examinations page: a multipart form of the semester.
fn semester_form(s: &Signed, path: &str, semester: &SemesterId) -> Request {
    Request::multipart(
        s.url(path),
        &[
            ("authorizedID", &s.registration_number),
            ("semesterSubId", semester.as_str()),
            ("_csrf", &s.csrf),
        ],
    )
}

pub fn marks(s: &Signed, semester: &SemesterId) -> Request {
    semester_form(s, "/vtop/examinations/doStudentMarkView", semester)
}

pub fn exam_schedule(s: &Signed, semester: &SemesterId) -> Request {
    semester_form(
        s,
        "/vtop/examinations/doSearchExamScheduleForStudent",
        semester,
    )
}

pub fn grade_view(s: &Signed, semester: &SemesterId) -> Request {
    semester_form(
        s,
        "/vtop/examinations/examGradeView/doStudentGradeView",
        semester,
    )
}

pub fn grade_view_details(s: &Signed, semester: &SemesterId, course: &CourseId) -> Request {
    Request::form(
        s.url("/vtop/examinations/examGradeView/getGradeViewDetails"),
        &[
            ("authorizedID", &s.registration_number),
            ("x", "codex"),
            ("semesterSubId", semester.as_str()),
            ("courseId", course.as_str()),
            ("_csrf", &s.csrf),
        ],
    )
}

pub fn biometric(s: &Signed, date: &BiometricDate) -> Request {
    Request::form(
        s.url("/vtop/getStudBioHistory"),
        &[
            ("_csrf", &s.csrf),
            ("fromDate", date.as_str()),
            ("authorizedID", &s.registration_number),
        ],
    )
}

/// Every month of the semester's calendar: the month list, then one request
/// per month, sorted by date.
pub struct AcademicCalendar {
    signed: Signed,
    semester: SemesterId,
    /// Months still to fetch, last first; `None` until the list arrives.
    months: Option<Vec<String>>,
    /// The month the last request asked for.
    asked: String,
    entries: Vec<CalendarEntry>,
}

impl AcademicCalendar {
    pub fn new(signed: Signed, semester: SemesterId) -> Self {
        Self {
            signed,
            semester,
            months: None,
            asked: String::new(),
            entries: Vec::new(),
        }
    }

    fn next(&mut self) -> Step<AcademicCalendarData> {
        let Some(cal_date) = self.months.as_mut().and_then(Vec::pop) else {
            let mut entries = std::mem::take(&mut self.entries);
            entries.sort_by(|a, b| a.date.cmp(&b.date));
            return Step::Done(Ok(AcademicCalendarData {
                entries,
                semester_id: self.semester.as_str().to_string(),
                update_time: now_unix(),
            }));
        };
        let request = calendar_month(&self.signed, &self.semester, &cal_date);
        self.asked = cal_date;
        Step::Send(request)
    }

    fn read(&mut self, response: &Response) -> VtopResult<()> {
        let page = signed_page(response)?;
        match &self.months {
            None => {
                let mut months = calendar::parse_calendar_months(&page);
                months.reverse();
                self.months = Some(months);
            }
            Some(_) => self
                .entries
                .extend(calendar::parse_calendar_month(&page, &self.asked)),
        }
        Ok(())
    }
}

impl Machine for AcademicCalendar {
    type Output = AcademicCalendarData;

    fn start(&mut self) -> Step<AcademicCalendarData> {
        Step::Send(calendar_months(&self.signed, &self.semester))
    }

    fn feed(&mut self, response: Response) -> Step<AcademicCalendarData> {
        match self.read(&response) {
            Ok(()) => self.next(),
            Err(error) => Step::Done(Err(error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::VtopError;

    fn signed() -> Signed {
        Signed {
            base_url: "https://vtop.example".into(),
            csrf: "c".into(),
            registration_number: "22BCE0001".into(),
        }
    }

    fn page(html: &str) -> Response {
        Response {
            status: 200,
            headers: Vec::new(),
            url: "https://vtop.example/vtop/x".into(),
            body: html.as_bytes().to_vec(),
        }
    }

    fn body(request: &Request) -> &str {
        std::str::from_utf8(&request.body).unwrap()
    }

    #[test]
    fn bodies_match_the_old_client() {
        let s = signed();
        let sem = SemesterId::parse("AP2026").unwrap();
        assert_eq!(
            body(&timetable(&s, &sem)),
            "_csrf=c&semesterSubId=AP2026&authorizedID=22BCE0001"
        );
        assert_eq!(
            timetable(&s, &sem).url,
            "https://vtop.example/vtop/processViewTimeTable"
        );
        assert_eq!(
            body(&semesters(&s)),
            "verifyMenu=true&authorizedID=22BCE0001&_csrf=c&nocache=@(new Date().getTime())"
        );
        assert_eq!(
            body(&calendar_month(&s, &sem, "01-OCT-2026")),
            "_csrf=c&calDate=01-OCT-2026&semSubId=AP2026&classGroupId=COMB&authorizedID=22BCE0001"
        );
    }

    #[test]
    fn calendar_asks_for_each_month_in_order() {
        let months = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/calendar_months.html"
        ))
        .unwrap();
        let listed = calendar::parse_calendar_months(&months);
        assert!(listed.len() > 1, "fixture lists months");
        let mut machine = AcademicCalendar::new(signed(), SemesterId::parse("AP2026").unwrap());
        assert!(
            matches!(machine.start(), Step::Send(ref r) if r.url.ends_with("/getDateForSemesterPreview"))
        );
        let mut step = machine.feed(page(&months));
        let mut asked = Vec::new();
        while let Step::Send(request) = step {
            asked.push(
                body(&request)
                    .split('&')
                    .nth(1)
                    .unwrap()
                    .trim_start_matches("calDate=")
                    .to_string(),
            );
            step = machine.feed(page("<table></table>"));
        }
        assert_eq!(
            asked,
            listed.iter().map(|m| url_encode(m)).collect::<Vec<_>>()
        );
        assert!(matches!(step, Step::Done(Ok(_))));
    }

    #[test]
    fn calendar_stops_on_an_expired_session() {
        let mut machine = AcademicCalendar::new(signed(), SemesterId::parse("AP2026").unwrap());
        machine.start();
        let expired = Response {
            status: 302,
            headers: vec![("Location".into(), "/vtop/login".into())],
            url: "https://vtop.example/vtop/x".into(),
            body: Vec::new(),
        };
        assert!(matches!(
            machine.feed(expired),
            Step::Done(Err(VtopError::SessionExpired))
        ));
    }
}
