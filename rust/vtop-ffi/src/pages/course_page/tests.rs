use vtop_core::types::{CourseLecture, CourseMaterial, TimetableSlot};

use std::fs;
use std::path::PathBuf;

use super::*;

fn class(id: &str, faculty: &str, slot: &str) -> CoursePageClass {
    CoursePageClass {
        class_id: id.into(),
        erp_id: "70001".into(),
        class_group: "General".into(),
        course_code: "CSE2001".into(),
        course_title: "Data Structures".into(),
        course_type: "Embedded Theory".into(),
        slot: slot.into(),
        faculty: faculty.into(),
        faculty_school: "SCOPE".into(),
    }
}

fn timetable(slots: &[(&str, &str, &str)]) -> TimetableData {
    let slots = slots
        .iter()
        .map(|(code, faculty, slot)| {
            serde_json::from_value::<TimetableSlot>(serde_json::json!({
                "serial": "1", "day": "MON", "slot": slot, "course_code": code,
                "course_type": "ETH", "room_no": "", "block": "", "start_time": "",
                "end_time": "", "name": "", "kind": "Theory", "faculty": faculty,
            }))
            .unwrap()
        })
        .collect();
    serde_json::from_value::<TimetableData>(serde_json::json!({
        "slots": [], "semester_id": "AP1", "update_time": 0,
    }))
    .map(|mut data| {
        data.slots = slots;
        data
    })
    .unwrap()
}

fn course(id: &str, code: &str, kind: &str) -> CoursePageCourse {
    CoursePageCourse {
        id: id.into(),
        code: code.into(),
        title: format!("{code} title"),
        course_type: kind.into(),
    }
}

#[test]
fn courses_group_by_code_in_vtop_order() {
    let page = CoursePage::from(CoursePageCourses {
        semester_id: "AP1".into(),
        courses: vec![
            course("1", "CSE2001", "ETH"),
            course("2", "MAT1001", "TH"),
            course("3", "CSE2001", "ELA"),
            course("4", "ENG1001", "SS"),
        ],
        update_time: 9,
    });
    let groups: Vec<_> = page
        .groups
        .iter()
        .map(|g| {
            (
                g.code.as_str(),
                g.parts.iter().map(|p| p.label.as_str()).collect::<Vec<_>>(),
            )
        })
        .collect();
    assert_eq!(
        groups,
        [
            ("CSE2001", vec!["Theory", "Lab"]),
            ("MAT1001", vec!["Theory"]),
            ("ENG1001", vec!["Soft skills"]),
        ]
    );
    assert!(page.groups[0].parts[1].lab && !page.groups[0].parts[0].lab);
    assert_eq!(page.updated_at, 9);
}

#[test]
fn my_class_by_faculty_then_slot() {
    let classes = vec![
        class("C1", "Dr. Rao", "A1+TA1"),
        class("C2", "Dr.  Rao", "B1+TB1"),
        class("C3", "Dr. Iyer", "A1+TA1"),
    ];
    // The faculty teaches two sections: the shared slot settles it.
    let tt = timetable(&[("CSE2001", "dr. rao - SCOPE", "B1"), ("MAT1001", "X", "A1")]);
    assert_eq!(
        my_class(&classes, Some(&tt)).map(|c| c.class_id.as_str()),
        Some("C2")
    );
    // One section by that faculty.
    let tt = timetable(&[("CSE2001", "Dr. Iyer - SCOPE", "C1")]);
    assert_eq!(
        my_class(&classes, Some(&tt)).map(|c| c.class_id.as_str()),
        Some("C3")
    );
    // Unknown faculty, a slot two sections share: unsettled.
    let tt = timetable(&[("CSE2001", "Dr. Nobody", "A1")]);
    assert!(my_class(&classes, Some(&tt)).is_none());
    // Not in the timetable.
    let tt = timetable(&[("MAT1001", "Dr. Rao", "B1")]);
    assert!(my_class(&classes, Some(&tt)).is_none());
    assert!(my_class(&classes, None).is_none());
}

#[test]
fn view_picks_the_asked_then_mine_then_the_only_one() {
    let dir = std::env::temp_dir().join(format!("vtop-ffi-course-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("pages/AP1")).unwrap();
    fs::write(
        dir.join(crate::SEMESTERS),
        r#"{"semesters":[{"id":"AP1","name":"Fall"}],"update_time":1}"#,
    )
    .unwrap();
    fs::write(dir.join(crate::SELECTED), r#""AP1""#).unwrap();
    // Not Core::new: logging is process-wide and another test reads its own log file.
    let core = Core::for_test(&dir);
    let classes = |list: Vec<CoursePageClass>| CoursePageClasses {
        semester_id: "AP1".into(),
        course_id: "X1".into(),
        classes: list,
        update_time: 5,
    };
    let two = || {
        classes(vec![
            class("C1", "Dr. Rao", "A1"),
            class("C2", "Dr. Iyer", "B1"),
        ])
    };
    assert_eq!(core.view("X1", two(), None, true).selected, None);
    assert_eq!(
        core.view("X1", two(), Some("C2"), true).selected.as_deref(),
        Some("C2")
    );
    // An id VTOP no longer lists is ignored.
    assert_eq!(core.view("X1", two(), Some("C9"), true).selected, None);
    let one = core.view(
        "X1",
        classes(vec![class("C1", "Dr. Rao", "A1")]),
        None,
        true,
    );
    assert_eq!(one.selected.as_deref(), Some("C1"));
    assert_eq!(
        (one.course.code.as_str(), one.course.label.as_str()),
        ("CSE2001", "Embedded Theory")
    );
    let tt = timetable(&[("CSE2001", "Dr. Iyer - SCOPE", "B1")]);
    core.write(&format!("pages/AP1/{TIMETABLE}"), &tt);
    let view = core.view("X1", two(), None, true);
    assert_eq!(view.selected.as_deref(), Some("C2"));
    assert_eq!(view.sections.iter().filter(|s| s.mine).count(), 1);
    // Without timetable matching, multi-section courses stay unselected and none are marked mine.
    let raw = core.view("X1", two(), None, false);
    assert_eq!(raw.selected, None);
    assert!(raw.sections.iter().all(|s| !s.mine));

    // Cached: none until the shown section's plan is saved.
    fs::create_dir_all(dir.join("pages/AP1/course_page/classes")).unwrap();
    fs::create_dir_all(dir.join("pages/AP1/course_page/plan")).unwrap();
    core.write(&format!("pages/AP1/{}", classes_file("X1")), &two());
    assert!(
        core.cached_course_view("f".into(), "X1".into(), None, true)
            .is_none()
    );
    // Without timetable matching, no section is selected so classes alone are enough for the picker.
    let picker = core
        .cached_course_view("f".into(), "X1".into(), None, false)
        .unwrap();
    assert_eq!(picker.selected, None);
    assert!(picker.plan.is_none());
    core.write(
        &format!("pages/AP1/{}", plan_file("C2")),
        &CoursePageDetail {
            semester_id: "AP1".into(),
            class: class("C2", "Dr. Iyer", "B1"),
            course_id: "AM_CSE2001".into(),
            all_materials_path: "".into(),
            general_materials_path: "".into(),
            syllabus_path: "courseSyllabusDownload/AM_CSE2001/ETH".into(),
            has_course_plan: true,
            lectures: vec![CourseLecture {
                serial: "1".into(),
                date: "2026-07-14".into(),
                day: "TUE".into(),
                topic: " Arrays ".into(),
                materials: vec![CourseMaterial {
                    label: "Reference Material I".into(),
                    path: "downloadPdf/AP1/C2/1/14-07-2026".into(),
                }],
            }],
            update_time: 3,
        },
    );
    let view = core
        .cached_course_view("f".into(), "X1".into(), None, true)
        .unwrap();
    let plan = view.plan.unwrap();
    assert_eq!(view.updated_at, 3);
    assert_eq!(plan.lectures[0].day, records::epoch_day("2026-07-14"));
    assert_eq!(plan.lectures[0].topic, "Arrays");
    assert_eq!(plan.lectures[0].materials[0].label, "Material I");
    assert!(
        core.cached_course_view("f".into(), "../x".into(), None, true)
            .is_none()
    );
    let _ = fs::remove_dir_all(&dir);
}

/// Answers every request with `head` and `body`; returns the base URL.
/// Answers a `HEAD` with `head` (and, as a server may, the body); anything else gets 405,
/// so a link is only ever made from a `HEAD`.
async fn serve(head: &'static str, body: Vec<u8>) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let body = body.clone();
            tokio::spawn(async move {
                let mut buffer = vec![0u8; 16 * 1024];
                if socket.read(&mut buffer).await.unwrap_or(0) == 0 {
                    return;
                }
                if !buffer.starts_with(b"HEAD ") {
                    let refused = "HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\n\r\n";
                    let _ = socket.write_all(refused.as_bytes()).await;
                    return;
                }
                let head = format!(
                    "HTTP/1.1 200 OK\r\n{head}\r\nContent-Length: {}\r\n\r\n",
                    body.len()
                );
                let _ = socket.write_all(head.as_bytes()).await;
                let _ = socket.write_all(&body).await;
            });
        }
    });
    format!("http://{address}")
}

fn signed_in_core(base: String, name: &str) -> (Core, PathBuf) {
    use vtop_core::{SessionState, VtopClient, VtopConfig};
    let dir = std::env::temp_dir().join(format!("vtop-ffi-file-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let client = VtopClient::builder()
        .config(VtopConfig {
            base_url: base,
            ..VtopConfig::default()
        })
        .with_session(&SessionState {
            cookies: "JSESSIONID=abc".into(),
            csrf_token: Some("csrf-1".into()),
            registration_number: Some("22BCE0001".into()),
            otp_issued_at: None,
            logged_in_at: None,
        })
        .unwrap();
    let session = crate::Session::for_test(
        client,
        crate::Account {
            username: "22BCE0001".into(),
            password: "pw".into(),
            registration_number: None,
        },
    );
    let core = Core::for_test(&dir);
    core.install(session);
    (core, dir)
}

#[tokio::test]
async fn a_file_link_carries_the_session_and_never_its_secrets_in_debug() {
    let base = serve(
        "Content-Type: application/pdf; charset=binary\r\nContent-Disposition: attachment; filename=\"Unit 1.pdf\"",
        vec![0; 1000],
    )
    .await;
    let (core, dir) = signed_in_core(base.clone(), "ok");
    let link = core
        .course_file_link("f".into(), "downloadPdf/AP1/C2/1/14-07-2026".into())
        .await
        .unwrap();
    assert!(
        link.url
            .starts_with(&format!("{base}/vtop/downloadPdf/AP1/C2/1/14-07-2026?")),
        "{}",
        link.url
    );
    assert!(link.url.contains("_csrf=") && !link.cookie.is_empty());
    assert_eq!(
        (link.name.as_str(), link.mime.as_deref()),
        ("Unit 1.pdf", Some("application/pdf"))
    );
    let debug = format!("{link:?}");
    assert!(
        !debug.contains("_csrf") && !debug.contains(&link.cookie),
        "{debug}"
    );
    // Not a course page link: refused before any request.
    assert!(
        core.course_file_link("f".into(), "hostel/x".into())
            .await
            .is_err()
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_file_link_without_a_type_has_none() {
    let link = |file_name: &str, content_type: &str| {
        file_link(
            vtop_core::client::FileLink {
                url: "u".into(),
                cookie: "c".into(),
                file_name: file_name.into(),
                content_type: content_type.into(),
            },
            "course-plan",
        )
    };
    assert_eq!(link("Plan.xls", "").mime, None);
    assert_eq!(link("Plan.xls", " ; charset=binary").mime, None);
    assert_eq!(
        link("Plan.xls", "Application/PDF; charset=binary")
            .mime
            .as_deref(),
        Some("application/pdf")
    );
    assert_eq!(link(" ", "").name, "course-plan");
}

#[tokio::test]
async fn a_plan_link_needs_a_semester() {
    let base = serve("Content-Type: application/vnd.ms-excel", vec![0; 10]).await;
    let (core, dir) = signed_in_core(base, "plan");
    assert_eq!(
        core.course_plan_link("f".into(), "C2".into()).await,
        Err(CoreError::SemesterNeeded)
    );
    fs::write(
        dir.join(crate::SEMESTERS),
        r#"{"semesters":[{"id":"AP1","name":"Fall"}],"update_time":1}"#,
    )
    .unwrap();
    fs::write(dir.join(crate::SELECTED), r#""AP1""#).unwrap();
    let link = core
        .course_plan_link("f".into(), "C2".into())
        .await
        .unwrap();
    assert!(
        link.url
            .contains("CoursePlanExcelDownload?semesterSubId=AP1&classId=C2"),
        "{}",
        link.url
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn names_ids_and_labels() {
    assert!(plain("AP2026A1").is_ok());
    assert!(plain("../etc").is_err() && plain("").is_err());
    assert_eq!(plan_file("a/../b"), "course_page/plan/a____b.json");
    assert_eq!(material_label("reference  Material II"), "Material II");
    assert_eq!(material_label("Notes"), "Notes");
    assert_eq!(type_label("pjt"), "Project");
    assert!(is_lab("Embedded Lab") && !is_lab("ETH"));
}
