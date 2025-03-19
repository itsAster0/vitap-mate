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
    let mut classes = vec![];
    let document = Html::parse_document(&html);
    let selector = Selector::parse(r#"select[name="courseCode"] option"#).unwrap();
    for element in document.select(&selector).skip(1) {
        if let Some(value) = element.value().attr("value") {
            if let Some(name) = element.text().next() {
                classes.push(format!(
                    "{}:{}",
                    name.trim().to_string(),
                    value.trim().to_string()
                ));
            }
        }
    }
    return serde_json::to_string(&classes).unwrap();
}