//! The course page: the course list, a course's classes and a class's
//! lecture plan. Tables are read by their header text so a reordered
//! column does not shift the data.

use std::sync::LazyLock;

use scraper::{ElementRef, Html, Selector};

use super::{selector, word_text, TD, TR};
use crate::now_unix;
use crate::types::{
    CourseLecture, CourseMaterial, CoursePageClass, CoursePageClasses, CoursePageCourse,
    CoursePageCourses, CoursePageDetail,
};

static COURSE_OPTION: LazyLock<Selector> = LazyLock::new(|| selector("select#courseCode option"));
static TABLE: LazyLock<Selector> = LazyLock::new(|| selector("table"));
static DOWNLOAD_LINK: LazyLock<Selector> = LazyLock::new(|| selector("a[href*='vtopDownload']"));
static ANCHOR: LazyLock<Selector> = LazyLock::new(|| selector("a"));
static SPAN: LazyLock<Selector> = LazyLock::new(|| selector("span"));
static INPUT: LazyLock<Selector> = LazyLock::new(|| selector("input[id]"));
static BUTTON: LazyLock<Selector> = LazyLock::new(|| selector("button[onclick]"));

pub fn parse_courses(html: &str, semester_id: &str) -> CoursePageCourses {
    let document = Html::parse_document(html);
    let courses = document
        .select(&COURSE_OPTION)
        .filter_map(|option| {
            let id = option.value().attr("value")?.trim().to_string();
            if id.is_empty() {
                return None;
            }
            // "CSE4007 - Digital Image Processing - ETH"; the title itself
            // may hold " - ", so take the ends.
            let label = word_text(&option);
            let parts: Vec<&str> = label.split(" - ").collect();
            let (code, title, course_type) = match parts.as_slice() {
                [code, middle @ .., kind] if !middle.is_empty() => {
                    (code.to_string(), middle.join(" - "), kind.to_string())
                }
                _ => (String::new(), label.clone(), String::new()),
            };
            Some(CoursePageCourse {
                id,
                code,
                title,
                course_type,
            })
        })
        .collect();
    CoursePageCourses {
        semester_id: semester_id.to_string(),
        courses,
        update_time: now_unix(),
    }
}

pub fn parse_classes(html: &str, semester_id: &str, course_id: &str) -> CoursePageClasses {
    let document = Html::parse_document(html);
    let classes = document
        .select(&TABLE)
        .find_map(|table| {
            let rows = header_rows(&table);
            rows.iter()
                .any(|row| row.has("faculty") && row.has("class id"))
                .then_some(rows)
        })
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            // processViewStudentCourseDetail('AP2026272','70435','AP2026272000042')
            let args = row.view_args.clone()?;
            let [_, erp_id, class_id] = args.as_slice() else {
                return None;
            };
            let (faculty, faculty_school) = split_faculty(&row.cell("faculty"));
            Some(CoursePageClass {
                class_id: class_id.clone(),
                erp_id: erp_id.clone(),
                class_group: row.cell("class group"),
                course_code: row.cell("course code"),
                course_title: row.cell("course title"),
                course_type: row.cell("course type"),
                slot: row.cell("slot"),
                faculty,
                faculty_school,
            })
        })
        .collect();
    CoursePageClasses {
        semester_id: semester_id.to_string(),
        course_id: course_id.to_string(),
        classes,
        update_time: now_unix(),
    }
}

pub fn parse_detail(html: &str, semester_id: &str, erp_id: &str) -> CoursePageDetail {
    let document = Html::parse_document(html);
    let tables: Vec<Vec<HeaderRow>> = document.select(&TABLE).map(|t| header_rows(&t)).collect();

    let info = tables
        .iter()
        .flatten()
        .find(|row| row.has("class id") && row.has("faculty"));
    let (faculty, faculty_school) =
        split_faculty(&info.map(|r| r.cell("faculty")).unwrap_or_default());
    let class = CoursePageClass {
        class_id: input_value(&document, "classId"),
        erp_id: erp_id.to_string(),
        class_group: info.map(|r| r.cell("class group")).unwrap_or_default(),
        course_code: info.map(|r| r.cell("course code")).unwrap_or_default(),
        course_title: info.map(|r| r.cell("course title")).unwrap_or_default(),
        course_type: info.map(|r| r.cell("course type")).unwrap_or_default(),
        slot: info.map(|r| r.cell("slot")).unwrap_or_default(),
        faculty,
        faculty_school,
    };

    let lectures = tables
        .iter()
        .flatten()
        .filter(|row| row.has("lecture topic"))
        .filter_map(|row| {
            let serial = row.cell("sl.no.");
            serial.parse::<u32>().ok()?;
            let date_cell = row.element("lecture date")?;
            Some(CourseLecture {
                serial,
                date: lecture_date(&date_cell),
                day: row.cell("lecture day"),
                topic: row.cell("lecture topic"),
                materials: row
                    .element("reference material")
                    .map(|cell| materials(&cell))
                    .unwrap_or_default(),
            })
        })
        .collect();

    let link_titled = |title: &str| {
        document
            .select(&DOWNLOAD_LINK)
            .find(|link| word_text(link).eq_ignore_ascii_case(title))
            .and_then(|link| download_path(&link))
            .unwrap_or_default()
    };
    let syllabus_path = document
        .select(&DOWNLOAD_LINK)
        .filter_map(|link| download_path(&link))
        .find(|path| path.starts_with("courseSyllabusDownload/"))
        .unwrap_or_default();

    CoursePageDetail {
        semester_id: semester_id.to_string(),
        class,
        course_id: input_value(&document, "courseId"),
        all_materials_path: link_titled("Download ALL Materials"),
        general_materials_path: link_titled("Download General Materials"),
        syllabus_path,
        has_course_plan: document
            .select(&ANCHOR)
            .any(|a| word_text(&a).eq_ignore_ascii_case("Download Course Plan")),
        lectures,
        update_time: now_unix(),
    }
}

/// A body row of a table whose first row is the header, keyed by
/// lower-cased header text.
struct HeaderRow<'a> {
    headers: std::rc::Rc<Vec<String>>,
    cells: Vec<ElementRef<'a>>,
    /// The arguments of the row's View button, if it has one.
    view_args: Option<Vec<String>>,
}

impl<'a> HeaderRow<'a> {
    fn has(&self, header: &str) -> bool {
        self.headers.iter().any(|h| h == header)
    }

    fn element(&self, header: &str) -> Option<ElementRef<'a>> {
        let index = self.headers.iter().position(|h| h == header)?;
        self.cells.get(index).copied()
    }

    fn cell(&self, header: &str) -> String {
        self.element(header)
            .map(|c| word_text(&c))
            .unwrap_or_default()
    }
}

fn header_rows<'a>(table: &ElementRef<'a>) -> Vec<HeaderRow<'a>> {
    let mut rows = table.select(&TR);
    let Some(header) = rows.next() else {
        return Vec::new();
    };
    let headers = std::rc::Rc::new(
        header
            .select(&TD)
            .map(|cell| word_text(&cell).to_lowercase())
            .collect::<Vec<_>>(),
    );
    rows.filter_map(|row| {
        let cells: Vec<ElementRef<'a>> = row.select(&TD).collect();
        if cells.len() < headers.len() {
            return None;
        }
        let view_args = row
            .select(&BUTTON)
            .filter_map(|button| button.value().attr("onclick"))
            .find_map(|call| call_args(call, "processViewStudentCourseDetail("));
        Some(HeaderRow {
            headers: headers.clone(),
            cells,
            view_args,
        })
    })
    .collect()
}

/// The quoted arguments of `name(...)` in `script`.
fn call_args(script: &str, name: &str) -> Option<Vec<String>> {
    let start = script.find(name)? + name.len();
    let inner = &script[start..start + script[start..].find(')')?];
    Some(
        inner
            .split(',')
            .map(|arg| arg.trim().trim_matches('\'').trim().to_string())
            .collect(),
    )
}

/// "70435 - Sucharitha M - SENSE" → ("Sucharitha M", "SENSE").
fn split_faculty(value: &str) -> (String, String) {
    let parts: Vec<&str> = value.split(" - ").map(str::trim).collect();
    match parts.as_slice() {
        [_, name, school] => (name.to_string(), school.to_string()),
        [_, name] => (name.to_string(), String::new()),
        _ => (value.to_string(), String::new()),
    }
}

/// The first span holds `14-07-2026`; returned as `2026-07-14`.
fn lecture_date(cell: &ElementRef<'_>) -> String {
    let raw = cell
        .select(&SPAN)
        .next()
        .map(|span| word_text(&span))
        .unwrap_or_else(|| word_text(cell));
    let parts: Vec<&str> = raw.split('-').collect();
    match parts.as_slice() {
        [day, month, year] if year.len() == 4 => format!("{year}-{month}-{day}"),
        _ => raw,
    }
}

fn materials(cell: &ElementRef<'_>) -> Vec<CourseMaterial> {
    cell.select(&DOWNLOAD_LINK)
        .filter_map(|link| {
            Some(CourseMaterial {
                label: word_text(&link),
                path: download_path(&link)?,
            })
        })
        .collect()
}

/// `javascript:vtopDownload('downloadPdf/…')` → `downloadPdf/…`.
fn download_path(link: &ElementRef<'_>) -> Option<String> {
    let href = link.value().attr("href")?;
    call_args(href, "vtopDownload(")?
        .into_iter()
        .next()
        .filter(|path| !path.is_empty())
}

fn input_value(document: &Html, id: &str) -> String {
    document
        .select(&INPUT)
        .find(|input| input.value().attr("id") == Some(id))
        .and_then(|input| input.value().attr("value"))
        .map(|value| value.trim().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_course_list() {
        let html = r##"<select id="courseCode">
            <option value="" selected="selected">--Choose Course --</option>
            <option value="AP2026272000055">CSE4007 - Digital Image Processing - ETH</option>
            <option value="AP1">ABC1001 - Design - Build - Test - ELA</option>
            </select>"##;
        let data = parse_courses(html, "AP2026272");
        assert_eq!(data.courses.len(), 2);
        assert_eq!(data.courses[0].id, "AP2026272000055");
        assert_eq!(data.courses[0].code, "CSE4007");
        assert_eq!(data.courses[0].title, "Digital Image Processing");
        assert_eq!(data.courses[0].course_type, "ETH");
        assert_eq!(data.courses[1].title, "Design - Build - Test");
    }

    #[test]
    fn reads_every_class_of_a_course() {
        let html = r##"<table>
            <tr><td><b>Sl.No.</b></td><td><b>Class Group</b></td><td><b>Course Code</b></td>
              <td><b>Course Title</b></td><td><b>Course Type</b></td><td><b>Class Id</b></td>
              <td><b>Slot</b></td><td><b>Faculty</b></td><td><b>Action</b></td></tr>
            <tr><td>1</td><td>General (Semester)</td><td>CSE4007</td>
              <td>Digital Image Processing</td><td>Embedded Theory</td>
              <td>AP2026272000042</td><td>A1/TA1</td><td>70435 - Sucharitha M - SENSE</td>
              <td><button type="button" onclick="javascript: processViewStudentCourseDetail(&#39;AP2026272&#39;,&#39;70435&#39;,&#39;AP2026272000042&#39;);">View</button></td></tr>
            </table>"##;
        let data = parse_classes(html, "AP2026272", "AP2026272000055");
        assert_eq!(data.classes.len(), 1);
        let class = &data.classes[0];
        assert_eq!(class.class_id, "AP2026272000042");
        assert_eq!(class.erp_id, "70435");
        assert_eq!(class.slot, "A1/TA1");
        assert_eq!(class.faculty, "Sucharitha M");
        assert_eq!(class.faculty_school, "SENSE");
        assert_eq!(class.course_type, "Embedded Theory");
    }

    #[test]
    fn reads_a_lecture_plan() {
        let html = r##"
            <a href="javascript:vtopDownload(&#39;academics/common/allCourseMeterialDownload/1/1/AP2026272/AP2026272000046&#39;)">
              <span></span> Download ALL Materials</a>
            <input type="hidden" id="classId" value="AP2026272000046" />
            <input type="hidden" id="courseId" value="AM_CSE4007_00100" />
            <table><tr><td><b>Class Group</b></td><td><b>Course Code</b></td><td><b>Course Title</b></td>
              <td><b>Course Type</b></td><td><b>Class Id</b></td><td><b>Slot</b></td><td><b>Faculty</b></td></tr>
              <tr><td>General (Semester)</td><td>CSE4007</td><td>Digital Image Processing</td>
              <td>Embedded Theory</td><td>AP2026272000046</td><td>A2+TA2</td>
              <td>70435 - Sucharitha M - SENSE</td></tr></table>
            <a href="javascript:vtopDownload(&#39;academics/common/allCourseMeterialDownload/2/1/AP2026272/AP2026272000046&#39;)">
              Download General Materials</a>
            <table><tr><td>Syllabus</td><td><a href="javascript:vtopDownload(&#39;courseSyllabusDownload/AM_CSE4007_00100/ETH&#39;)"><span>Download</span></a></td></tr></table>
            <table><tr><td><b>Sl.No.</b></td><td><b>Lecture Date</b></td><td><b>Lecture Day</b></td>
              <td><b>Lecture Topic</b></td><td><b>Reference Material</b></td></tr>
              <tr><td>3</td><td><span>17-07-2026</span><br/><span>[17-Jul-2026]</span></td>
                <td>FRI</td><td>Light, Brightness
                  adaption</td>
                <td><p><a class="btn btn-link" href="javascript:vtopDownload(&#39;downloadPdf/AP2026272/AP2026272000046/19/17-07-2026&#39;)"><span>Reference Material I</span></a></p>
                    <p><a class="btn btn-link" href="javascript:vtopDownload(&#39;downloadPdf/AP2026272/AP2026272000046/20/17-07-2026&#39;)"><span>Reference Material II</span></a></p></td></tr>
              <tr><td>6</td><td><span>24-07-2026</span></td><td>FRI</td><td>Perspective Projection</td><td></td></tr>
            </table>
            <a onclick="javascript: coursePlanExcelDownload('AP2026272','AP2026272000046');" href="javascript:void(0);">Download Course Plan</a>"##;
        let data = parse_detail(html, "AP2026272", "70435");
        assert_eq!(data.class.class_id, "AP2026272000046");
        assert_eq!(data.class.faculty, "Sucharitha M");
        assert_eq!(data.class.slot, "A2+TA2");
        assert_eq!(data.course_id, "AM_CSE4007_00100");
        assert!(data
            .all_materials_path
            .ends_with("/1/1/AP2026272/AP2026272000046"));
        assert!(data
            .general_materials_path
            .ends_with("/2/1/AP2026272/AP2026272000046"));
        assert_eq!(
            data.syllabus_path,
            "courseSyllabusDownload/AM_CSE4007_00100/ETH"
        );
        assert!(data.has_course_plan);
        assert_eq!(data.lectures.len(), 2);
        let lecture = &data.lectures[0];
        assert_eq!(lecture.date, "2026-07-17");
        assert_eq!(lecture.day, "FRI");
        assert_eq!(lecture.topic, "Light, Brightness adaption");
        assert_eq!(lecture.materials.len(), 2);
        assert_eq!(lecture.materials[1].label, "Reference Material II");
        assert_eq!(
            lecture.materials[0].path,
            "downloadPdf/AP2026272/AP2026272000046/19/17-07-2026"
        );
        assert!(data.lectures[1].materials.is_empty());
    }
}
