//! Every signed-in read page, once: its name and arguments (as the app sends them to the
//! engine's `POST /engine/fetch`), the app's cache file, and how to fetch and parse it. A new
//! read page is one [`PageSpec`] here, listed in [`each`], plus its parser: vtop-core runs it
//! (`VtopClient::fetch`), the engine serves everything in [`each`], and the app fetches and
//! caches it by its spec. Names, arguments and files are what old apps and the deployed
//! engine already use; never rename one.

use serde::Serialize;

use crate::error::VtopResult;
use crate::http::{Request, Response};
use crate::inputs::{BiometricDate, CourseId, CourseType, SemesterId};
use crate::machine::{Machine, Signed, Step, signed_page};
use crate::outing::{self, OutingKind};
use crate::parser::{
    attendance, biometric, course_page, exam_schedule, grade_history, grades, hostel, marks,
    mentor, profile, timetable,
};
use crate::types::{
    AcademicCalendarData, AttendanceData, BiometricData, CoursePageClasses, CoursePageCourses,
    CoursePageDetail, ExamScheduleData, FullAttendanceData, GeneralOutingData, GradeDetailsData,
    GradeHistoryData, GradeViewData, MarksData, MentorData, SemesterData, StudentProfile,
    TimetableData, WeekendOutingData,
};
use crate::{course_pages, pages};

/// A page's arguments as they travel: the semester (None for a page not kept per semester)
/// and [`PageSpec::args`] by name.
#[derive(Debug, Clone, Copy, Default)]
pub struct Query<'a> {
    pub semester: Option<&'a str>,
    pub args: &'a [(&'a str, &'a str)],
}

impl Query<'static> {
    pub const NONE: Self = Query {
        semester: None,
        args: &[],
    };
}

impl<'a> Query<'a> {
    pub fn for_semester(semester: &'a str) -> Self {
        Self {
            semester: Some(semester),
            args: &[],
        }
    }

    fn semester_id(&self) -> VtopResult<SemesterId> {
        SemesterId::parse(self.semester.unwrap_or_default())
    }

    fn arg(&self, name: &str) -> &'a str {
        self.args
            .iter()
            .find(|(key, _)| *key == name)
            .map_or("", |(_, value)| *value)
    }
}

pub type BoxMachine<T> = Box<dyn Machine<Output = T> + Send>;

/// A page's machine, built once signed in.
pub type Start<T> = Box<dyn FnOnce(Signed) -> BoxMachine<T> + Send>;

pub struct PageSpec<T> {
    pub name: &'static str,
    pub args: &'static [&'static str],
    /// The app's copy, under `pages/` (`pages/<semester>/` for a semester's page); for a page
    /// kept per course, class or day, the folder of `<key>.json` files.
    pub file: &'static str,
    /// Checks the arguments (before any sign-in), then gives the machine.
    pub build: fn(&Query) -> VtopResult<Start<T>>,
}

/// Each [`PageSpec`] in turn, for a list of every page (the engine's).
pub trait Visit {
    fn page<T: Serialize + Send + 'static>(&mut self, spec: &'static PageSpec<T>);
}

pub fn each(visit: &mut impl Visit) {
    visit.page(&SEMESTERS);
    visit.page(&ATTENDANCE);
    visit.page(&FULL_ATTENDANCE);
    visit.page(&TIMETABLE);
    visit.page(&MARKS);
    visit.page(&EXAM_SCHEDULE);
    visit.page(&GRADES);
    visit.page(&ACADEMIC_CALENDAR);
    visit.page(&GRADE_DETAILS);
    visit.page(&GRADE_HISTORY);
    visit.page(&BIOMETRIC);
    visit.page(&PROFILE);
    visit.page(&MENTOR);
    visit.page(&GENERAL_OUTING);
    visit.page(&WEEKEND_OUTING);
    visit.page(&COURSE_PAGE_COURSES);
    visit.page(&COURSE_PAGE_CLASSES);
    visit.page(&COURSE_PAGE_DETAIL);
}

pub const SEMESTERS: PageSpec<SemesterData> = PageSpec {
    name: "semesters",
    args: &[],
    file: "semesters.json",
    build: |_| {
        Ok(one(
            (),
            |s, _| pages::semesters(s),
            |page, _| timetable::parse_semid_timetable(page),
        ))
    },
};

pub const ATTENDANCE: PageSpec<AttendanceData> = PageSpec {
    name: "attendance",
    args: &[],
    file: "attendance.json",
    build: |q| {
        Ok(one(q.semester_id()?, pages::attendance, |page, sem| {
            attendance::parse_attendance(page, sem.as_str())
        }))
    },
};

pub const FULL_ATTENDANCE: PageSpec<FullAttendanceData> = PageSpec {
    name: "full_attendance",
    args: &["course_id", "course_type"],
    file: "attendance_detail",
    build: |q| {
        let input = (
            q.semester_id()?,
            CourseId::parse(q.arg("course_id"))?,
            CourseType::parse(q.arg("course_type"))?,
        );
        Ok(one(
            input,
            |s, (sem, course, kind)| pages::full_attendance(s, sem, course, kind),
            |page, (sem, course, kind)| {
                attendance::parse_full_attendance(
                    page,
                    sem.as_str(),
                    course.as_str(),
                    kind.as_str(),
                )
            },
        ))
    },
};

pub const TIMETABLE: PageSpec<TimetableData> = PageSpec {
    name: "timetable",
    args: &[],
    file: "timetable.json",
    build: |q| {
        Ok(one(q.semester_id()?, pages::timetable, |page, sem| {
            timetable::parse_timetable(page, sem.as_str())
        }))
    },
};

pub const MARKS: PageSpec<MarksData> = PageSpec {
    name: "marks",
    args: &[],
    file: "marks.json",
    build: |q| {
        Ok(one(q.semester_id()?, pages::marks, |page, sem| {
            marks::parse_marks(page, sem.as_str())
        }))
    },
};

pub const EXAM_SCHEDULE: PageSpec<ExamScheduleData> = PageSpec {
    name: "exam_schedule",
    args: &[],
    file: "exam_schedule.json",
    build: |q| {
        Ok(one(q.semester_id()?, pages::exam_schedule, |page, sem| {
            exam_schedule::parse_schedule(page, sem.as_str())
        }))
    },
};

pub const GRADES: PageSpec<GradeViewData> = PageSpec {
    name: "grades",
    args: &[],
    file: "grades.json",
    build: |q| {
        Ok(one(q.semester_id()?, pages::grade_view, |page, sem| {
            grades::parse_grade_view(page, sem.as_str())
        }))
    },
};

/// The month list, then one request per month.
pub const ACADEMIC_CALENDAR: PageSpec<AcademicCalendarData> = PageSpec {
    name: "academic_calendar",
    args: &[],
    file: "calendar.json",
    build: |q| {
        let semester = q.semester_id()?;
        Ok(Box::new(move |s| -> BoxMachine<_> {
            Box::new(pages::AcademicCalendar::new(s, semester))
        }))
    },
};

pub const GRADE_DETAILS: PageSpec<GradeDetailsData> = PageSpec {
    name: "grade_details",
    args: &["course_id"],
    file: "grade_details",
    build: |q| {
        let input = (q.semester_id()?, CourseId::parse(q.arg("course_id"))?);
        Ok(one(
            input,
            |s, (sem, course)| pages::grade_view_details(s, sem, course),
            |page, (sem, course)| {
                grades::parse_grade_view_details(page, sem.as_str(), course.as_str())
            },
        ))
    },
};

pub const GRADE_HISTORY: PageSpec<GradeHistoryData> = PageSpec {
    name: "grade_history",
    args: &[],
    file: "grade_history.json",
    build: |_| {
        Ok(one(
            (),
            |s, _| pages::grade_history(s),
            |page, _| grade_history::parse_grade_history(page),
        ))
    },
};

pub const BIOMETRIC: PageSpec<BiometricData> = PageSpec {
    name: "biometric",
    args: &["date"],
    file: "biometric",
    build: |q| {
        Ok(one(
            BiometricDate::parse(q.arg("date"))?,
            pages::biometric,
            |page, date| biometric::parse_biometric(page, date.as_str()),
        ))
    },
};

pub const PROFILE: PageSpec<StudentProfile> = PageSpec {
    name: "profile",
    args: &[],
    file: "profile.json",
    build: |_| {
        Ok(one(
            (),
            |s, _| pages::profile(s),
            |page, _| profile::parse_profile(page),
        ))
    },
};

/// The mentor's details, then their messages.
pub const MENTOR: PageSpec<MentorData> = PageSpec {
    name: "mentor",
    args: &[],
    file: "mentor.json",
    build: |_| {
        Ok(Box::new(|s| {
            Pages::boxed(
                vec![pages::mentor_details(&s), pages::mentor_messages(&s)],
                |found| mentor::parse_mentor(nth(&found, 0), nth(&found, 1)),
            )
        }))
    },
};

pub const GENERAL_OUTING: PageSpec<GeneralOutingData> = PageSpec {
    name: "general_outing",
    args: &[],
    file: "outing_general.json",
    build: |_| {
        Ok(one(
            (),
            |s, _| outing::list(s, OutingKind::General),
            |page, _| hostel::parse_general_outing(page),
        ))
    },
};

pub const WEEKEND_OUTING: PageSpec<WeekendOutingData> = PageSpec {
    name: "weekend_outing",
    args: &[],
    file: "outing_weekend.json",
    build: |_| {
        Ok(one(
            (),
            |s, _| outing::list(s, OutingKind::Weekend),
            |page, _| hostel::parse_weekend_outing(page),
        ))
    },
};

pub const COURSE_PAGE_COURSES: PageSpec<CoursePageCourses> = PageSpec {
    name: "course_page_courses",
    args: &[],
    file: "course_page.json",
    build: |q| {
        Ok(one(q.semester_id()?, course_pages::courses, |page, sem| {
            course_page::parse_courses(page, sem.as_str())
        }))
    },
};

pub const COURSE_PAGE_CLASSES: PageSpec<CoursePageClasses> = PageSpec {
    name: "course_page_classes",
    args: &["course_id"],
    file: "course_page/classes",
    build: |q| {
        let input = (
            q.semester_id()?,
            course_pages::plain_id(q.arg("course_id"), "course id")?.to_string(),
        );
        Ok(one(
            input,
            |s, (sem, course)| course_pages::classes(s, sem, course),
            |page, (sem, course)| course_page::parse_classes(page, sem.as_str(), course),
        ))
    },
};

pub const COURSE_PAGE_DETAIL: PageSpec<CoursePageDetail> = PageSpec {
    name: "course_page_detail",
    args: &["erp_id", "class_id"],
    file: "course_page/plan",
    build: |q| {
        let input = (
            q.semester_id()?,
            course_pages::plain_id(q.arg("erp_id"), "faculty id")?.to_string(),
            course_pages::plain_id(q.arg("class_id"), "class id")?.to_string(),
        );
        Ok(one(
            input,
            |s, (sem, erp, class)| course_pages::detail(s, sem, erp, class),
            |page, (sem, erp, _)| course_page::parse_detail(page, sem.as_str(), erp),
        ))
    },
};

/// A page of one request, `parse`d with the checked arguments `input`.
fn one<I: Send + 'static, T: 'static>(
    input: I,
    request: fn(&Signed, &I) -> Request,
    parse: fn(&str, &I) -> T,
) -> Start<T> {
    Box::new(move |s| {
        let request = request(&s, &input);
        Pages::boxed(vec![request], move |found| parse(nth(&found, 0), &input))
    })
}

fn nth(pages: &[String], n: usize) -> &str {
    pages.get(n).map_or("", String::as_str)
}

/// Signed-in requests sent in turn, their pages read with [`signed_page`] and parsed together.
struct Pages<T> {
    requests: std::vec::IntoIter<Request>,
    found: Vec<String>,
    parse: Option<Box<dyn FnOnce(Vec<String>) -> T + Send>>,
}

impl<T: 'static> Pages<T> {
    fn boxed(
        requests: Vec<Request>,
        parse: impl FnOnce(Vec<String>) -> T + Send + 'static,
    ) -> BoxMachine<T> {
        Box::new(Self {
            requests: requests.into_iter(),
            found: Vec::new(),
            parse: Some(Box::new(parse)),
        })
    }
}

impl<T> Machine for Pages<T> {
    type Output = T;

    fn start(&mut self) -> Step<T> {
        match (self.requests.next(), self.parse.take()) {
            (Some(request), parse) => {
                self.parse = parse;
                Step::Send(request)
            }
            (None, Some(parse)) => Step::Done(Ok(parse(std::mem::take(&mut self.found)))),
            (None, None) => Step::Done(Err(crate::VtopError::ConfigurationError(
                "machine started twice".into(),
            ))),
        }
    }

    fn feed(&mut self, response: Response) -> Step<T> {
        match signed_page(&response) {
            Ok(page) => {
                self.found.push(page);
                self.start()
            }
            Err(error) => Step::Done(Err(error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Names(Vec<&'static str>);

    impl Visit for Names {
        fn page<T: Serialize + Send + 'static>(&mut self, spec: &'static PageSpec<T>) {
            self.0.push(spec.name);
        }
    }

    /// One `name args file` line per spec, args comma-joined (`-` for none).
    struct Contract(Vec<String>);

    impl Visit for Contract {
        fn page<T: Serialize + Send + 'static>(&mut self, spec: &'static PageSpec<T>) {
            let args = if spec.args.is_empty() {
                "-".to_string()
            } else {
                spec.args.join(",")
            };
            self.0.push(format!("{} {args} {}", spec.name, spec.file));
        }
    }

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

    #[test]
    fn every_page_named_once() {
        let mut names = Names(Vec::new());
        each(&mut names);
        let count = names.0.len();
        names.0.sort_unstable();
        names.0.dedup();
        assert_eq!(names.0.len(), count);
    }

    /// Old apps and the deployed engine use each spec's name, args and file, so the checked-in
    /// list only grows at its end.
    #[test]
    fn specs_keep_their_contract() {
        let mut contract = Contract(Vec::new());
        each(&mut contract);
        let snapshot: Vec<&str> = include_str!("../tests/spec_contract.txt")
            .lines()
            .filter(|line| !line.is_empty())
            .collect();
        for (at, line) in snapshot.iter().enumerate() {
            assert_eq!(
                contract.0.get(at).map(String::as_str),
                Some(*line),
                "spec line {} changed or removed: never rename or drop a spec's name, args or \
                 file (old apps and the deployed engine use them); new specs go at the end of \
                 spec::each",
                at + 1,
            );
        }
        let added = contract.0.get(snapshot.len()..).unwrap_or_default();
        assert!(
            added.is_empty(),
            "new specs: append these lines to vtop-proto/tests/spec_contract.txt in the same \
             change:\n{}",
            added.join("\n"),
        );
    }

    #[test]
    fn bad_arguments_fail_before_any_request() {
        let date = Query {
            semester: None,
            args: &[("date", "2026-10-20")],
        };
        assert!((BIOMETRIC.build)(&date).is_err());
        assert!((ATTENDANCE.build)(&Query::NONE).is_err());
        let class = Query {
            semester: Some("AP1"),
            args: &[("erp_id", "1"), ("class_id", "../x")],
        };
        assert!((COURSE_PAGE_DETAIL.build)(&class).is_err());
    }

    #[test]
    fn mentor_asks_for_both_pages_then_parses() {
        let mut machine = (MENTOR.build)(&Query::NONE).unwrap()(signed());
        assert!(
            matches!(machine.start(), Step::Send(ref r) if r.url.ends_with("/viewProctorDetails"))
        );
        assert!(
            matches!(machine.feed(page("<table></table>")), Step::Send(ref r) if r.url.ends_with("/viewMessagesSendByProctor"))
        );
        assert!(matches!(
            machine.feed(page("<table></table>")),
            Step::Done(Ok(_))
        ));
    }

    #[test]
    fn one_page_sends_the_request_it_always_did() {
        let mut machine = (TIMETABLE.build)(&Query::for_semester("AP2026")).unwrap()(signed());
        let Step::Send(request) = machine.start() else {
            panic!("no request");
        };
        assert_eq!(
            request.url,
            "https://vtop.example/vtop/processViewTimeTable"
        );
        assert_eq!(
            request.body,
            b"_csrf=c&semesterSubId=AP2026&authorizedID=22BCE0001"
        );
        assert!(
            matches!(machine.feed(page("<html></html>")), Step::Done(Ok(data)) if data.semester_id == "AP2026")
        );
    }
}
