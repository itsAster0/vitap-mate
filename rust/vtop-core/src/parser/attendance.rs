//! Attendance summary and per-class attendance detail.

use scraper::{ElementRef, Html};

use super::{compact_cell, compact_text, TD, TR};
use crate::now_unix;
use crate::types::{AttendanceData, AttendanceRecord, FullAttendanceData, FullAttendanceRecord};

/// Summary rows have at least this many cells; the last holds the detail
/// link. Rows with exactly this many have no FAT/CAT column.
const SUMMARY_MIN_CELLS: usize = 10;

/// Pulls `(course_id, course_type)` out of the detail link's
/// `processViewAttendanceDetail('sem','regno','course','type')` call.
fn detail_link_args(cell: &ElementRef<'_>) -> Option<(String, String)> {
    let html = cell.html();
    let mut args = html.split(',');
    let course_id = args.nth(2)?.replace('\'', "");
    let course_type = args.next()?.split(')').next()?.replace('\'', "");
    Some((course_id, course_type))
}

fn summary_row(row: ElementRef<'_>) -> Option<AttendanceRecord> {
    let cells: Vec<_> = row.select(&TD).collect();
    if cells.len() < SUMMARY_MIN_CELLS {
        return None;
    }
    let (course_id, course_type) = detail_link_args(cells.last()?)?;
    let has_fat_cat = cells.len() > SUMMARY_MIN_CELLS;
    let mut values = cells.iter();
    let mut next = || compact_cell(values.next());
    Some(AttendanceRecord {
        serial: next(),
        category: next(),
        course_name: next(),
        course_code: next(),
        faculty_detail: next(),
        classes_attended: next(),
        total_classes: next(),
        attendance_percentage: next(),
        attendence_fat_cat: if has_fat_cat { next() } else { "-".to_string() },
        debar_status: next(),
        course_id,
        course_type,
    })
}

pub fn parse_attendance(html: &str, sem: &str) -> AttendanceData {
    let document = Html::parse_document(html);
    AttendanceData {
        records: document
            .select(&TR)
            .skip(1)
            .filter_map(summary_row)
            .collect(),
        semester_id: sem.to_string(),
        update_time: now_unix(),
    }
}

/// The detail page starts with course, faculty and column header rows.
const DETAIL_HEADER_ROWS: usize = 3;

pub fn parse_full_attendance(
    html: &str,
    sem: &str,
    course_id: &str,
    course_type: &str,
) -> FullAttendanceData {
    let document = Html::parse_document(html);
    let records = document
        .select(&TR)
        .skip(DETAIL_HEADER_ROWS)
        .filter_map(|row| {
            let cells: Vec<_> = row.select(&TD).collect();
            if cells.len() <= 5 {
                return None;
            }
            Some(FullAttendanceRecord {
                serial: compact_text(&cells[0]),
                date: compact_text(&cells[1]),
                slot: compact_text(&cells[2]),
                day_time: compact_text(&cells[3]),
                status: compact_text(&cells[4]),
                remark: compact_text(&cells[5]),
            })
        })
        .collect();
    FullAttendanceData {
        records,
        semester_id: sem.to_string(),
        update_time: now_unix(),
        course_id: course_id.to_string(),
        course_type: course_type.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_without_a_detail_link_are_skipped() {
        let html =
            "<table><tr><th>h</th></tr><tr><td>1</td><td>2</td><td>3</td><td>4</td><td>5</td>\
                    <td>6</td><td>7</td><td>8</td><td>9</td><td>no link</td></tr></table>";
        assert!(parse_attendance(html, "S").records.is_empty());
    }
}
