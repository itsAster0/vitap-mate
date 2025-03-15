use scraper::{Html, Selector};
use serde;
use serde::Deserialize;
use serde_json;

pub fn parse_semid_attendance(html: String) -> String {
    let mut sem_names_ids = vec![];
    let document = Html::parse_document(&html);
    let selector = Selector::parse(r#"select[name="semesterSubId"] option"#).unwrap();
    for element in document.select(&selector).skip(1) {
        if let Some(value) = element.value().attr("value") {
            if let Some(name) = element.text().next() {
                sem_names_ids.push(format!(
                    "{}:{}",
                    name.trim().replace("- AMR", "").to_string(),
                    value.trim().to_string()
                ));
            }
        }
    }
    return serde_json::to_string(&sem_names_ids).unwrap();
}

pub fn parse_attendance(html: String) -> String {
    #[derive(serde::Serialize, Deserialize)]
    struct Course {
        serial: String,
        category: String,
        course_name: String,
        course_code: String,
        course_type: String,
        faculty_detail: String,
        classes_attended: String,
        total_classes: String,
        attendance_percentage: String,
        attendence_fat_cat: String,
        debar_status: String,
        course_id: String,
    }
    let document = Html::parse_document(&html);
    let rows_selector = Selector::parse("tr").unwrap();
    let mut courses: Vec<Course> = Vec::new();
    for row in document.select(&rows_selector).skip(1) {
        let cells: Vec<_> = row.select(&Selector::parse("td").unwrap()).collect();
        if cells.len() > 10 {
            let cell9 = cells[10].html();
            let infocell = cell9.split(",").collect::<Vec<_>>();
            let course_id: String = infocell[2].to_string().replace("'", "");
            let course_type: String = infocell[3].split(")").collect::<Vec<_>>()[0]
                .to_string()
                .replace("'", "");
            let course = Course {
                serial: cells[0]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                category: cells[1]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                course_name: cells[2]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                course_code: cells[3]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                faculty_detail: cells[4]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                classes_attended: cells[5]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                total_classes: cells[6]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                attendance_percentage: cells[7]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                attendence_fat_cat: cells[8]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                debar_status: cells[9]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                course_id,
                course_type,
            };

            courses.push(course);
        }
    }
    let json_data = serde_json::to_string_pretty(&courses).unwrap();
    return json_data;
}

pub fn parse_full_attendance(html: String) -> String {
    #[derive(serde::Serialize, Deserialize)]
    struct AttendanceList {
        serial: String,
        date: String,
        slot: String,
        day_time: String,
        status: String,
        remark: String,
    }
    let document = Html::parse_document(&html);
    let rows_selector = Selector::parse("tr").unwrap();
    let mut attendance_lists: Vec<AttendanceList> = Vec::new();
    for row in document.select(&rows_selector).skip(1) {
        let cells: Vec<_> = row.select(&Selector::parse("td").unwrap()).collect();
        let attendance_list = AttendanceList {
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
    let json_data = serde_json::to_string_pretty(&attendance_lists).unwrap();
    return json_data;
}
