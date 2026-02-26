use super::super::types::*;
use scraper::{ElementRef, Html, Selector};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::new(1, 0))
        .as_secs()
}

fn extract_text(el: Option<&ElementRef>) -> String {
    el.map(|e| e.text().collect::<Vec<_>>().join(" "))
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn parse_grade_history(html: String) -> GradeHistoryData {
    let document = Html::parse_document(&html);
    let table_sel = Selector::parse("table.customTable").unwrap();
    let row_sel = Selector::parse("tr").unwrap();
    let td_sel = Selector::parse("td").unwrap();
    let row_content_sel = Selector::parse("tr.tableContent").unwrap();
    let nested_row_sel = Selector::parse("tr.tableContent-level1").unwrap();

    let empty_student = GradeHistoryStudentInfo {
        reg_no: String::new(),
        name: String::new(),
        programme_branch: String::new(),
        programme_mode: String::new(),
        study_system: String::new(),
        gender: String::new(),
        year_joined: String::new(),
        edu_status: String::new(),
        school: String::new(),
        campus: String::new(),
    };
    let empty_cgpa = GradeHistoryCgpa {
        credits_registered: String::new(),
        credits_earned: String::new(),
        cgpa: String::new(),
        s_grades: String::new(),
        a_grades: String::new(),
        b_grades: String::new(),
        c_grades: String::new(),
        d_grades: String::new(),
        e_grades: String::new(),
        f_grades: String::new(),
        n_grades: String::new(),
    };

    let mut student = empty_student.clone();
    let mut records = Vec::<GradeHistoryRecord>::new();
    let mut cgpa = empty_cgpa.clone();

    let tables: Vec<_> = document.select(&table_sel).collect();
    if let Some(first_table) = tables.first() {
        let first_content = first_table.select(&row_content_sel).next();
        if let Some(row) = first_content {
            let cells: Vec<_> = row.select(&td_sel).collect();
            if cells.len() >= 10 {
                student = GradeHistoryStudentInfo {
                    reg_no: extract_text(cells.get(0)),
                    name: extract_text(cells.get(1)),
                    programme_branch: extract_text(cells.get(2)),
                    programme_mode: extract_text(cells.get(3)),
                    study_system: extract_text(cells.get(4)),
                    gender: extract_text(cells.get(5)),
                    year_joined: extract_text(cells.get(6)),
                    edu_status: extract_text(cells.get(7)),
                    school: extract_text(cells.get(8)),
                    campus: extract_text(cells.get(9)),
                };
            }
        }
    }

    let mut attempts_map: HashMap<String, Vec<GradeHistoryAttempt>> = HashMap::new();
    if let Some(effective_table) = tables.get(1) {
        for row in effective_table.select(&row_content_sel) {
            if let Some(id) = row.value().attr("id") {
                if !id.starts_with("detailsView_") {
                    continue;
                }
                let course_code = id.replacen("detailsView_", "", 1);
                let mut attempts = Vec::new();
                for nested in row.select(&nested_row_sel) {
                    let cells: Vec<_> = nested.select(&td_sel).collect();
                    if cells.len() < 7 {
                        continue;
                    }
                    attempts.push(GradeHistoryAttempt {
                        course_code: extract_text(cells.get(0)),
                        course_title: extract_text(cells.get(1)),
                        course_type: extract_text(cells.get(2)),
                        credits: extract_text(cells.get(3)),
                        grade: extract_text(cells.get(4)),
                        exam_month: extract_text(cells.get(5)),
                        result_declared: extract_text(cells.get(6)),
                    });
                }
                attempts_map.insert(course_code, attempts);
            }
        }

        for row in effective_table.select(&row_content_sel) {
            if row.value().attr("id").is_some() {
                continue;
            }
            let cells: Vec<_> = row.select(&td_sel).collect();
            if cells.len() < 9 {
                continue;
            }
            let serial = extract_text(cells.get(0));
            if serial.parse::<u32>().is_err() {
                continue;
            }
            let course_code = extract_text(cells.get(1));
            records.push(GradeHistoryRecord {
                serial,
                course_code: course_code.clone(),
                course_title: extract_text(cells.get(2)),
                course_type: extract_text(cells.get(3)),
                credits: extract_text(cells.get(4)),
                grade: extract_text(cells.get(5)),
                exam_month: extract_text(cells.get(6)),
                result_declared: extract_text(cells.get(7)),
                course_distribution: extract_text(cells.get(8)),
                attempts: attempts_map.remove(&course_code).unwrap_or_default(),
            });
        }
    }

    // Parse CGPA details table by header marker.
    for table in document.select(&Selector::parse("table.table").unwrap()) {
        let text = extract_text(Some(&table));
        if !text.contains("CGPA") && !text.contains("Credits Registered") {
            continue;
        }
        let mut body_row: Option<ElementRef> = None;
        for row in table.select(&row_sel) {
            let cells: Vec<_> = row.select(&td_sel).collect();
            if cells.len() >= 11 {
                let first = extract_text(cells.first());
                if first.eq_ignore_ascii_case("Credits Registered") {
                    continue;
                }
                body_row = Some(row);
                break;
            }
        }
        if let Some(row) = body_row {
            let cells: Vec<_> = row.select(&td_sel).collect();
            cgpa = GradeHistoryCgpa {
                credits_registered: extract_text(cells.get(0)),
                credits_earned: extract_text(cells.get(1)),
                cgpa: extract_text(cells.get(2)),
                s_grades: extract_text(cells.get(3)),
                a_grades: extract_text(cells.get(4)),
                b_grades: extract_text(cells.get(5)),
                c_grades: extract_text(cells.get(6)),
                d_grades: extract_text(cells.get(7)),
                e_grades: extract_text(cells.get(8)),
                f_grades: extract_text(cells.get(9)),
                n_grades: extract_text(cells.get(10)),
            };
            break;
        }
    }

    GradeHistoryData {
        student,
        records,
        cgpa,
        update_time: now_unix(),
    }
}
