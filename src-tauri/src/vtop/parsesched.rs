use scraper::{Html, Selector};
use serde;
use serde::Deserialize;
use serde_json;

pub fn parse_semid_schedule(html: String) -> String {
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

pub fn parse_schedule(html: String) -> String {
    #[derive(serde::Serialize, Deserialize)]
    struct Course {
        serial: String,
        slot: String,
        course_name: String,
        course_code: String,
        course_type: String,
        course_id: String,
        exam_date: String,
        exam_session: String,
        reporting_time: String,
        exam_time: String,
        venue: String,
        seat_location: String,
        seat_no: String,
    }
    #[derive(serde::Serialize, Deserialize)]
    struct Exam {
        exam_type: String,
        course: Vec<Course>,
    }
    let document = Html::parse_document(&html);
    let rows_selector = Selector::parse("tr").unwrap();
    let mut exams: Vec<Exam> = Vec::new();

    let mut off_set = -1;
    for row in document.select(&rows_selector).skip(2) {
        let cells: Vec<_> = row.select(&Selector::parse("td").unwrap()).collect();
        if cells.len() < 3 {
            let n = cells[0]
                .text()
                .collect::<Vec<_>>()
                .join("")
                .trim()
                .replace("\t", "")
                .replace("\n", "");
            let exam = Exam {
                exam_type: n,
                course: Vec::new(),
            };
            exams.push(exam);
            off_set += 1;
        } else if cells.len() > 12 {
            let course = Course {
                serial: cells[0]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                slot: cells[5]
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
                course_code: cells[1]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                course_type: cells[3]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                course_id: cells[4]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                exam_date: cells[6]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                exam_session: cells[7]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                reporting_time: cells[8]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                exam_time: cells[9]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                venue: cells[10]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                seat_location: cells[11]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                seat_no: cells[12]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
            };
            exams[off_set as usize].course.push(course);
        }
    }
    let json_data = serde_json::to_string_pretty(&exams).unwrap();
    return json_data;
}
