//! Per-semester grade view and per-course grade details.

use std::sync::LazyLock;

use scraper::{ElementRef, Html, Selector};

use super::{selector, word_cell, word_text, TD, TH, TR};
use crate::now_unix;
use crate::types::{
    GradeCourseRecord, GradeDetailMark, GradeDetailsData, GradeRange, GradeViewData, SemesterInfo,
};

static SEMESTER_OPTION: LazyLock<Selector> =
    LazyLock::new(|| selector("select#semesterSubId option"));
static DETAILS_BUTTON: LazyLock<Selector> = LazyLock::new(|| selector("button[onclick]"));
static TABLE: LazyLock<Selector> = LazyLock::new(|| selector("table"));
static OUTPUT: LazyLock<Selector> = LazyLock::new(|| selector("output"));
static SPAN: LazyLock<Selector> = LazyLock::new(|| selector("span"));
const GRADE_LABELS: [&str; 7] = ["S", "A", "B", "C", "D", "E", "F"];

fn extract_text(el: Option<&ElementRef>) -> String {
    word_cell(el)
}

fn clean_grade_range_text(input: String) -> String {
    input.replace('#', "").trim().to_string()
}

fn looks_like_grade_range(input: &str) -> bool {
    input.contains('>') || input.contains('<')
}

fn extract_course_id(button_cell: &ElementRef) -> String {
    if let Some(button) = button_cell.select(&DETAILS_BUTTON).next() {
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

pub fn parse_grade_view(html: &str, sem: &str) -> GradeViewData {
    let document = Html::parse_document(html);

    let mut semesters = Vec::new();
    for option in document.select(&SEMESTER_OPTION) {
        if let Some(id) = option.value().attr("value") {
            let id_trim = id.trim();
            if id_trim.is_empty() {
                continue;
            }
            let name = word_text(&option);
            semesters.push(SemesterInfo {
                id: id_trim.to_string(),
                name,
            });
        }
    }

    let mut courses = Vec::new();
    for row in document.select(&TR) {
        let cells: Vec<_> = row.select(&TD).collect();
        if cells.len() < 8 {
            continue;
        }
        let Some(button_cell) = cells
            .last()
            .filter(|cell| cell.select(&DETAILS_BUTTON).next().is_some())
        else {
            continue;
        };
        let serial = extract_text(cells.first());
        if serial.parse::<u32>().is_err() {
            continue;
        }

        let course_id = extract_course_id(button_cell);
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
        semester_id: sem.to_string(),
        update_time: now_unix(),
    }
}

pub fn parse_grade_view_details(html: &str, sem: &str, course_id: &str) -> GradeDetailsData {
    let document = Html::parse_document(html);

    let mut class_number = String::new();
    let mut class_course_type = String::new();
    let mut grand_total = String::new();
    let mut marks = Vec::new();
    let mut grade_ranges = Vec::new();

    // Extract grade ranges from the "Range of Grades" distribution table if present.
    // Parse by TD column index first so inline '#' markers do not shift grade mapping.
    // Fallback to span parsing while filtering out marker-only values.
    for table in document.select(&TABLE) {
        let table_text = extract_text(Some(&table));
        if !table_text.contains("Range of Grades") {
            continue;
        }
        for row in table.select(&TR) {
            let cells: Vec<_> = row.select(&TD).collect();
            let labels = GRADE_LABELS;
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
                let mut cleaned_spans = Vec::new();
                for sp in row.select(&SPAN) {
                    let txt = clean_grade_range_text(extract_text(Some(&sp)));
                    if txt.is_empty() || !looks_like_grade_range(&txt) {
                        continue;
                    }
                    cleaned_spans.push(txt);
                }
                if cleaned_spans.len() >= 7 {
                    for (label, range) in labels.iter().zip(cleaned_spans) {
                        parsed.push(GradeRange {
                            grade: (*label).to_string(),
                            range,
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

    let mut class_numbers = Vec::<String>::new();
    let mut class_types = Vec::<String>::new();
    let mut class_totals = Vec::<String>::new();
    let mut per_class_marks = Vec::<(String, Vec<GradeDetailMark>)>::new();

    for table in document.select(&TABLE) {
        let ths: Vec<_> = table.select(&TH).collect();
        if ths.len() < 2 {
            continue;
        }

        let header_line = extract_text(ths.first());
        if !header_line.contains("Class Number") {
            continue;
        }

        let parsed_class_number = header_line
            .split("Class Number")
            .nth(1)
            .unwrap_or("")
            .replace(':', "")
            .trim()
            .to_string();
        let parsed_class_course_type = extract_text(ths.get(1))
            .split("Course Type")
            .nth(1)
            .unwrap_or("")
            .replace(':', "")
            .trim()
            .to_string();
        let mut parsed_grand_total = String::new();
        let mut table_marks = Vec::<GradeDetailMark>::new();

        for row in table.select(&TR) {
            let outputs: Vec<_> = row.select(&OUTPUT).collect();
            if outputs.len() == 7 {
                table_marks.push(GradeDetailMark {
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

            let tds: Vec<_> = row.select(&TD).collect();
            if !tds.is_empty() {
                let th_text = row
                    .select(&TH)
                    .map(|e| extract_text(Some(&e)))
                    .collect::<Vec<_>>()
                    .join(" ");
                if th_text.contains("Total") {
                    if let Some(last_th) = row.select(&TH).last() {
                        parsed_grand_total = extract_text(Some(&last_th));
                    }
                }
            }
        }

        if !parsed_class_number.is_empty() {
            class_numbers.push(parsed_class_number.clone());
        }
        if !parsed_class_course_type.is_empty() {
            class_types.push(parsed_class_course_type.clone());
        }
        if !parsed_grand_total.is_empty() {
            class_totals.push(parsed_grand_total.clone());
        }
        per_class_marks.push((parsed_class_course_type, table_marks));
    }

    if !class_numbers.is_empty() {
        class_number = class_numbers.join(" | ");
    }
    if !class_types.is_empty() {
        class_course_type = class_types.join(" | ");
    }
    if !class_totals.is_empty() {
        grand_total = class_totals.join(" | ");
    }

    let has_multiple_classes = per_class_marks.len() > 1;
    for (class_type, table_marks) in per_class_marks {
        for mut mark in table_marks {
            if has_multiple_classes && !class_type.is_empty() {
                mark.mark_title = format!("{} • {}", class_type, mark.mark_title);
            }
            marks.push(mark);
        }
    }

    GradeDetailsData {
        semester_id: sem.to_string(),
        course_id: course_id.to_string(),
        class_number,
        class_course_type,
        grand_total,
        marks,
        grade_ranges,
        update_time: now_unix(),
    }
}
