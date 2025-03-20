use scraper::{Html, Selector};
use serde;

pub fn parse_semid(html: String) -> String {
    let mut sem_names_ids = vec![];
    let document = Html::parse_document(&html);
    let selector = Selector::parse(r#"select[name="semesterSubId"] option"#).unwrap();
    for element in document.select(&selector).skip(1) {
        if let Some(value) = element.value().attr("value") {
            if let Some(name) = element.text().next() {
                sem_names_ids.push(format!(
                    "{}:{}",
                    name.trim().to_string(),
                    value.trim().to_string()
                ));
            }
        }
    }
    return serde_json::to_string(&sem_names_ids).unwrap();
}

pub fn parse_courses(html: String) -> String {
    let mut courses = vec![];
    let document = Html::parse_document(&html);
    let selector = Selector::parse(r#"select[name="courseCode"] option"#).unwrap();
    for element in document.select(&selector).skip(1) {
        if let Some(value) = element.value().attr("value") {
            if let Some(name) = element.text().next() {
                courses.push(format!(
                    "{}:{}",
                    name.trim().to_string(),
                    value.trim().to_string()
                ));
            }
        }
    }
    return serde_json::to_string(&courses).unwrap();
}

pub fn parse_classes(html: String) -> String {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Class {
        serial: String,
        course_name: String,
        course_code: String,
        course_type: String,
        faculty_detail: String,
        class_id: String,
        slot: String,
        erpid: String,
    }
    let mut classes: Vec<Class> = vec![];
    let document = Html::parse_document(&html);
    let rows_selector = Selector::parse("tr").unwrap();
    for row in document.select(&rows_selector).skip(1) {
        let cells: Vec<_> = row.select(&Selector::parse("td").unwrap()).collect();
        if cells.len() > 7 {
            let k = cells[7]
                .text()
                .collect::<Vec<_>>()
                .join("")
                .trim()
                .replace("\t", "")
                .replace("\n", "");
            let temp = k.split("-").map(|a| a.trim()).collect::<Vec<_>>();
            let erpid = temp[0].to_string();
            let faculityname = temp[1].to_string();
            let class = Class {
                serial: cells[0]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                course_name: cells[3]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                course_code: cells[2]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                course_type: cells[4]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                faculty_detail: faculityname,
                slot: cells[6]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
                erpid: erpid,
                class_id: cells[5]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", ""),
            };
            classes.push(class);
        }
    }

    return serde_json::to_string(&classes).unwrap();
}

pub fn parse_dlist(html: String) -> String {
    let mut dlist: Vec<CourseTable> = vec![];
    #[derive(serde::Serialize, serde::Deserialize)]
    struct CourseTable {
        serial: String,
        date: String,
        day: String,
        topic: String,
        links: Vec<String>,
    }
    let document = Html::parse_document(&html);
    let all_download_selector = Selector::parse("a.btn.btn-primary").unwrap();
    let mut all_download = vec![];
    let tables_selector = Selector::parse("tbody").unwrap();
    let rows_selector = Selector::parse("tr").unwrap();
    for downlods in document.select(&all_download_selector) {
        let href = downlods.value().attr("href").unwrap().to_string();
        let href_parse = href.split("(").collect::<Vec<_>>()[1];
        all_download.push(href_parse.to_string().replace(")", "").replace("'", ""));
    }
    let all_download_table = CourseTable {
        serial: "-1".to_string(),
        date: "-".to_string(),
        day: "-".to_string(),
        topic: "-".to_string(),
        links: all_download,
    };
    dlist.push(all_download_table);
    let mut table_count = 0;
    let mut table1_links: Vec<String> = vec![];
    for table in document.select(&tables_selector).skip(1) {
        table_count += 1;
        if table_count == 2 {
            let table1 = CourseTable {
                serial: "-2".to_string(),
                date: "-".to_string(),
                day: "-".to_string(),
                topic: "-".to_string(),
                links: table1_links.clone(),
            };
            dlist.push(table1);
        }
        for row in table.select(&rows_selector).skip(1) {
            let cells: Vec<_> = row.select(&Selector::parse("td").unwrap()).collect();
            if table_count == 1 {
                if cells.len() > 1 {
                    let value = cells[0]
                        .text()
                        .collect::<Vec<_>>()
                        .join("")
                        .trim()
                        .replace("\t", "")
                        .replace("\n", "");
                    let p = cells[1].select(&Selector::parse("a").unwrap()).next();
                    match p {
                        Some(p) => {
                            let href = p.value().attr("href").unwrap().to_string();
                            let href_parse = href.split("(").collect::<Vec<_>>()[1];
                            let k = format!(
                                "{}:{}",
                                value,
                                href_parse.to_string().replace(")", "").replace("'", "")
                            );
                            table1_links.push(k);
                        }
                        None => {}
                    }
                }
            } else if table_count == 2 {
                let mut links: Vec<String> = vec![];
                for link in cells[4].select(&Selector::parse("a").unwrap()) {
                    let href = link.value().attr("href").unwrap().to_string();
                    let href_parse = href.split("(").collect::<Vec<_>>()[1];
                    links.push(href_parse.to_string().replace(")", "").replace("'", ""));
                }
                let table = CourseTable {
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
                    day: cells[2]
                        .text()
                        .collect::<Vec<_>>()
                        .join("")
                        .trim()
                        .replace("\t", "")
                        .replace("\n", ""),
                    topic: cells[3]
                        .text()
                        .collect::<Vec<_>>()
                        .join("")
                        .trim()
                        .replace("\t", "")
                        .replace("\n", ""),
                    links: links,
                };
                dlist.push(table);
            }
        }
    }
    return serde_json::to_string(&dlist).unwrap(); // 0 and 1 elements are different 0 rep all download links and 1 rep tables
}
