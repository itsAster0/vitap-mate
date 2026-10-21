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

/// Pieces of a title for a natural order: "Experiment-2" before
/// "Experiment-10", case ignored.
fn natural_key(title: &str) -> Vec<(u8, u64, String)> {
    let mut key = Vec::new();
    let mut chars = title.chars().peekable();
    while let Some(&c) = chars.peek() {
        let digits = c.is_ascii_digit();
        let mut piece = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() != digits {
                break;
            }
            piece.push(c);
            chars.next();
        }
        key.push(if digits {
            (0, piece.parse().unwrap_or(u64::MAX), String::new())
        } else {
            (1, 0, piece.to_lowercase())
        });
    }
    key
}

/// VTOP lists a course's marks in a different order from one fetch to the
/// next; a fixed order keeps the data (and its hash) the same when nothing
/// changed. Rows are numbered again in that order.
fn in_fixed_order(mut marks: Vec<MarksRecordEach>) -> Vec<MarksRecordEach> {
    marks.sort_by_cached_key(|m| {
        (
            natural_key(&m.markstitle),
            natural_key(&m.maxmarks),
            natural_key(&m.weightage),
            m.scoredmark.clone(),
            m.status.clone(),
            m.remark.clone(),
        )
    });
    for (index, mark) in marks.iter_mut().enumerate() {
        mark.serial = (index + 1).to_string();
    }
    marks
}

fn mark_rows(container: Option<&ElementRef<'_>>) -> Vec<MarksRecordEach> {
    let Some(container) = container else {
        return Vec::new();
    };
    let rows = container
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
        .collect();
    in_fixed_order(rows)
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
