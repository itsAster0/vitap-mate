use scraper::{Html, Selector};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::super::types::*;

pub fn parse_attendance(html: String, sem: String) -> AttendanceData {
    let document = Html::parse_document(&html);
    let rows_selector = Selector::parse("tr").unwrap();
    let mut courses: Vec<AttendanceRecord> = Vec::new();
    for row in document.select(&rows_selector).skip(1) {
        let cells: Vec<_> = row.select(&Selector::parse("td").unwrap()).collect();
        let max = 9;
        if cells.len() > max {
            let cell9 = cells[max].html();
            let infocell = cell9.split(",").collect::<Vec<_>>();
            let course_id: String = infocell[2].to_string().replace("'", "");
            let course_type: String = infocell[3].split(")").collect::<Vec<_>>()[0]
                .to_string()
                .replace("'", "");
            let mut index_vec = 0;

            let mut next = || {
                let v = cells[index_vec].clone();
                index_vec += 1;
                v.text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", "")
            };

            let course = AttendanceRecord {
                serial: next(),
                category: next(),
                course_name: next(),
                course_code: next(),
                faculty_detail: next(),
                classes_attended: next(),
                total_classes: next(),
                attendance_percentage: next(),
                attendence_fat_cat: "0".into(),
                //  cells[8]
                //     .text()
                //     .collect::<Vec<_>>()
                //     .join("")
                //     .trim()
                //     .replace("\t", "")
                //     .replace("\n", ""),
                debar_status: next(),
                course_id, //edit the top index for these
                course_type,
            };

            courses.push(course);
        }
    }
    AttendanceData {
        records: courses,
        semester_id: sem,
        update_time: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::new(1, 0))
            .as_secs(),
    }
}

pub fn parse_full_attendance(
    html: String,
    sem: String,
    course_id: String,
    course_type: String,
) -> FullAttendanceData {
    let document = Html::parse_document(&html);
    let rows_selector = Selector::parse("tr").unwrap();
    let mut attendance_lists: Vec<FullAttendanceRecord> = Vec::new();
    for row in document.select(&rows_selector).skip(3) {
        let cells: Vec<_> = row.select(&Selector::parse("td").unwrap()).collect();
        if cells.len() > 5 {
            let attendance_list = FullAttendanceRecord {
                serial: cells[0]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                date: cells[1]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                slot: cells[2]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                day_time: cells[3]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                status: cells[4]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                remark: cells[5]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
            };

            attendance_lists.push(attendance_list);
        }
    }
    FullAttendanceData {
        records: attendance_lists,
        semester_id: sem,
        update_time: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::new(1, 0))
            .as_secs(),
        course_id,
        course_type,
    }
}
