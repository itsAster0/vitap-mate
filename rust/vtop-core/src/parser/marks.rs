//! Continuous assessment marks.
//!
//! VTOP lists each course as a `tr.tableContent` row followed by another
//! `tr.tableContent` row whose first cell nests the course's mark rows.

use std::sync::LazyLock;

use scraper::{ElementRef, Html, Selector};

use super::{compact_cell, selector, TD};
use crate::now_unix;
use crate::types::{MarksData, MarksRecord, MarksRecordEach};

static CONTENT_ROW: LazyLock<Selector> = LazyLock::new(|| selector("tr.tableContent"));
static MARK_ROW: LazyLock<Selector> = LazyLock::new(|| selector("tr.tableContent-level1"));

fn course_row(cells: &[ElementRef<'_>]) -> MarksRecord {
    let cell = |index: usize| compact_cell(cells.get(index));
    MarksRecord {
        serial: cell(0),
        coursecode: cell(2),
        coursetitle: cell(3),
        coursetype: cell(4),
        faculity: cell(6),
        slot: cell(7),
        marks: Vec::new(),
    }
}

fn mark_rows(container: Option<&ElementRef<'_>>) -> Vec<MarksRecordEach> {
    let Some(container) = container else {
        return Vec::new();
    };
    container
        .select(&MARK_ROW)
        .map(|row| {
            let cells: Vec<_> = row.select(&TD).collect();
            let cell = |index: usize| compact_cell(cells.get(index));
            MarksRecordEach {
                serial: cell(0),
                markstitle: cell(1),
                maxmarks: cell(2),
                weightage: cell(3),
                status: cell(4),
                scoredmark: cell(5),
                weightagemark: cell(6),
                remark: cell(7),
            }
        })
        .collect()
}

pub fn parse_marks(html: &str, sem: &str) -> MarksData {
    let document = Html::parse_document(html);
    let mut records = Vec::new();
    let mut course: Option<MarksRecord> = None;

    for (index, row) in document.select(&CONTENT_ROW).enumerate() {
        let cells: Vec<_> = row.select(&TD).collect();
        if index.is_multiple_of(2) {
            course = Some(course_row(&cells));
        } else {
            let mut record = course.take().unwrap_or_else(|| course_row(&[]));
            record.marks = mark_rows(cells.first());
            records.push(record);
        }
    }

    MarksData {
        records,
        semester_id: sem.to_string(),
        update_time: now_unix(),
    }
}
