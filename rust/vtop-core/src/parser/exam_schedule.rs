//! Exam schedule, grouped by exam (CAT1, FAT, ...).

use scraper::Html;

use super::{compact_text, TD, TR};
use crate::now_unix;
use crate::types::{ExamScheduleData, ExamScheduleRecord, PerExamScheduleRecord};

/// Column header and spacer rows before the first exam group.
const HEADER_ROWS: usize = 2;
const RECORD_MIN_CELLS: usize = 13;

pub fn parse_schedule(html: &str, sem: &str) -> ExamScheduleData {
    let document = Html::parse_document(html);
    let mut exams: Vec<PerExamScheduleRecord> = Vec::new();

    for row in document.select(&TR).skip(HEADER_ROWS) {
        let cells: Vec<_> = row.select(&TD).collect();
        if cells.len() < 3 {
            // A one-cell row names the next exam group.
            let Some(title) = cells.first() else {
                continue;
            };
            exams.push(PerExamScheduleRecord {
                exam_type: compact_text(title),
                records: Vec::new(),
            });
        } else if cells.len() >= RECORD_MIN_CELLS {
            let text = |index: usize| compact_text(&cells[index]);
            let record = ExamScheduleRecord {
                serial: text(0),
                slot: text(5),
                course_name: text(2),
                course_code: text(1),
                course_type: text(3),
                course_id: text(4),
                exam_date: text(6),
                exam_session: text(7),
                reporting_time: text(8),
                exam_time: text(9),
                venue: text(10),
                seat_location: text(11),
                seat_no: text(12),
            };
            if exams.is_empty() {
                // A record before any group title: keep it in an unnamed group.
                exams.push(PerExamScheduleRecord {
                    exam_type: String::new(),
                    records: Vec::new(),
                });
            }
            if let Some(group) = exams.last_mut() {
                group.records.push(record);
            }
        }
    }

    ExamScheduleData {
        exams,
        semester_id: sem.to_string(),
        update_time: now_unix(),
    }
}
