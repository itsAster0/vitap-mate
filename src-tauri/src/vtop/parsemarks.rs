use scraper::{Html, Selector};
use serde;
use serde::Deserialize;
use serde_json;

pub fn parse_semid_marks(html: String) -> String {
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

pub fn parse_marks(html: String) -> String {
    #[derive(serde::Serialize, Deserialize, Clone)]
    struct Course {
        serial: String,
        coursecode: String,
        coursetitle: String,
        coursetype: String,
        faculity: String,
        slot: String,
        marks: Vec<String>,
    }
    let document = Html::parse_document(&html);
    let mut courses: Vec<Course> = Vec::new();

    let mut course = Course {
        serial: "".to_string(),
        coursecode: "".to_string(),
        coursetitle: "".to_string(),
        coursetype: "".to_string(),
        faculity: "".to_string(),
        slot: "".to_string(),
        marks: vec![],
    };

    let mut bmarks = false;
    for row in document.select(&Selector::parse("tr.tableContent").unwrap()) {
        let cells: Vec<_> = row.select(&Selector::parse("td").unwrap()).collect();
        if bmarks {
            let marks_el: Vec<_> = cells[0]
                .select(&Selector::parse("tr.tableContent-level1").unwrap())
                .collect();
            let mut marks_vec = vec![];
            for i in marks_el {
                let mut marks: String = "".to_string();
                let mk: Vec<_> = i.select(&Selector::parse("td").unwrap()).collect();
                for p in mk {
                    let c = p
                        .text()
                        .collect::<Vec<_>>()
                        .join("")
                        .trim()
                        .replace("\t", "")
                        .replace("\n", "");

                    if marks.is_empty() {
                        marks = c;
                    } else {
                        marks = marks + ":" + &c;
                    }
                }
                marks_vec.push(marks);
            }
            course.marks = marks_vec;

            courses.push(course.clone());
            course = Course {
                serial: "".to_string(),
                coursecode: "".to_string(),
                coursetitle: "".to_string(),
                coursetype: "".to_string(),
                faculity: "".to_string(),
                slot: "".to_string(),
                marks: vec![],
            };
        } else {
            course = Course {
                serial: cells[0]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                coursecode: cells[2]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                coursetitle: cells[3]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                coursetype: cells[4]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                faculity: cells[6]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                slot: cells[7]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                marks: vec![],
            }
        }

        bmarks = !bmarks
    }
    let json_data = serde_json::to_string_pretty(&courses).unwrap();
    return json_data;
}
