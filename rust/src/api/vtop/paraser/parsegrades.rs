use super::super::types::*;
use scraper::{ElementRef, Html, Selector};
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

fn clean_grade_range_text(input: String) -> String {
    input.replace('#', "").trim().to_string()
}

fn looks_like_grade_range(input: &str) -> bool {
    input.contains('>') || input.contains('<')
}

fn extract_course_id(button_cell: &ElementRef, onclick_sel: &Selector) -> String {
    if let Some(button) = button_cell.select(onclick_sel).next() {
        if let Some(onclick) = button.value().attr("onclick") {
            if let Some(start_idx) = onclick.find("getGradeViewDetails(") {
                let sliced = &onclick[start_idx..];
                if let Some(first_quote) = sliced.find('\'') {
                    let right = &sliced[first_quote + 1..];
                    if let Some(second_quote) = right.find('\'') {
                        return right[..second_quote].to_string();
                    }
                }
            }
        }
    }
    String::new()
}

pub fn parse_grade_view(html: String, sem: String) -> GradeViewData {
    let document = Html::parse_document(&html);
    let tr_sel = Selector::parse("tr").unwrap();
    let td_sel = Selector::parse("td").unwrap();
    let option_sel = Selector::parse("select#semesterSubId option").unwrap();
    let button_sel = Selector::parse("button[onclick]").unwrap();

    let mut semesters = Vec::new();
    for option in document.select(&option_sel) {
        if let Some(id) = option.value().attr("value") {
            let id_trim = id.trim();
            if id_trim.is_empty() {
                continue;
            }
            let name = extract_text(Some(&option));
            semesters.push(SemesterInfo {
                id: id_trim.to_string(),
                name,
            });
        }
    }

    let mut courses = Vec::new();
    for row in document.select(&tr_sel) {
        let cells: Vec<_> = row.select(&td_sel).collect();
        if cells.len() < 8 {
            continue;
        }
        let has_button = cells
            .last()
            .map(|cell| cell.select(&button_sel).next().is_some())
            .unwrap_or(false);
        if !has_button {
            continue;
        }
        let serial = extract_text(cells.first());
        if serial.parse::<u32>().is_err() {
            continue;
        }

        let course_id = extract_course_id(cells.last().unwrap(), &button_sel);
        courses.push(GradeCourseRecord {
            serial,
            course_code: extract_text(cells.get(1)),
            course_title: extract_text(cells.get(2)),
            course_type: extract_text(cells.get(3)),
            grading_type: extract_text(cells.get(4)),
            grand_total: extract_text(cells.get(5)),
            grade: extract_text(cells.get(6)),
            course_id,
        });
    }

    GradeViewData {
        courses,
        semesters,
        semester_id: sem,
        update_time: now_unix(),
    }
}

pub fn parse_grade_view_details(html: String, sem: String, course_id: String) -> GradeDetailsData {
    let document = Html::parse_document(&html);
    let table_sel = Selector::parse("table").unwrap();
    let tr_sel = Selector::parse("tr").unwrap();
    let td_sel = Selector::parse("td").unwrap();
    let th_sel = Selector::parse("th").unwrap();
    let output_sel = Selector::parse("output").unwrap();

    let mut class_number = String::new();
    let mut class_course_type = String::new();
    let mut grand_total = String::new();
    let mut marks = Vec::new();
    let mut grade_ranges = Vec::new();

    // Extract grade ranges from the "Range of Grades" distribution table if present.
    // Parse by TD column index first so inline '#' markers do not shift grade mapping.
    // Fallback to span parsing while filtering out marker-only values.
    for table in document.select(&table_sel) {
        let table_text = extract_text(Some(&table));
        if !table_text.contains("Range of Grades") {
            continue;
        }
        for row in table.select(&tr_sel) {
            let cells: Vec<_> = row.select(&td_sel).collect();
            let labels = ["S", "A", "B", "C", "D", "E", "F"];
            let mut parsed = Vec::new();

            if cells.len() >= 11 {
                for (i, label) in labels.iter().enumerate() {
                    let txt = clean_grade_range_text(extract_text(cells.get(4 + i)));
                    if txt.is_empty() || !looks_like_grade_range(&txt) {
                        continue;
                    }
                    parsed.push(GradeRange {
                        grade: (*label).to_string(),
                        range: txt,
                    });
                }
            }

            if parsed.len() < 7 {
                parsed.clear();
                let span_sel = Selector::parse("span").unwrap();
                let mut cleaned_spans = Vec::new();
                for sp in row.select(&span_sel) {
                    let txt = clean_grade_range_text(extract_text(Some(&sp)));
                    if txt.is_empty() || !looks_like_grade_range(&txt) {
                        continue;
                    }
                    cleaned_spans.push(txt);
                }
                if cleaned_spans.len() >= 7 {
                    for (i, label) in labels.iter().enumerate() {
                        parsed.push(GradeRange {
                            grade: (*label).to_string(),
                            range: cleaned_spans[i].clone(),
                        });
                    }
                }
            }

            if parsed.len() >= 6 {
                grade_ranges = parsed;
                break;
            }
        }
        if !grade_ranges.is_empty() {
            break;
        }
    }

    for table in document.select(&table_sel) {
        let ths: Vec<_> = table.select(&th_sel).collect();
        if ths.len() < 2 {
            continue;
        }

        let header_line = extract_text(ths.first());
        if !header_line.contains("Class Number") {
            continue;
        }

        class_number = header_line
            .split("Class Number")
            .nth(1)
            .unwrap_or("")
            .replace(':', "")
            .trim()
            .to_string();
        class_course_type = extract_text(ths.get(1))
            .split("Course Type")
            .nth(1)
            .unwrap_or("")
            .replace(':', "")
            .trim()
            .to_string();

        for row in table.select(&tr_sel) {
            let outputs: Vec<_> = row.select(&output_sel).collect();
            if outputs.len() == 7 {
                marks.push(GradeDetailMark {
                    serial: extract_text(outputs.first()),
                    mark_title: extract_text(outputs.get(1)),
                    max_mark: extract_text(outputs.get(2)),
                    weightage: extract_text(outputs.get(3)),
                    status: extract_text(outputs.get(4)),
                    scored_mark: extract_text(outputs.get(5)),
                    weightage_mark: extract_text(outputs.get(6)),
                });
                continue;
            }

            let tds: Vec<_> = row.select(&td_sel).collect();
            if !tds.is_empty() {
                let th_text = row
                    .select(&th_sel)
                    .map(|e| extract_text(Some(&e)))
                    .collect::<Vec<_>>()
                    .join(" ");
                if th_text.contains("Total") {
                    if let Some(last_th) = row.select(&th_sel).last() {
                        grand_total = extract_text(Some(&last_th));
                    }
                }
            }
        }
        break;
    }

    GradeDetailsData {
        semester_id: sem,
        course_id,
        class_number,
        class_course_type,
        grand_total,
        marks,
        grade_ranges,
        update_time: now_unix(),
    }
}
