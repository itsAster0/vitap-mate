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
    let mut marks=false;
    for row in document.select(&rows_selector).skip(1) {
        let cells: Vec<_> = row.select(&Selector::parse("td").unwrap()).collect();
        
        marks = !marks
    }
    let json_data = serde_json::to_string_pretty(&courses).unwrap();
    return json_data;
}
