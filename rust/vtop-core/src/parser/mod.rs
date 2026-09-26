//! HTML parsers for VTOP pages. Each takes the raw page and returns the
//! typed data from [`crate::types`]; none of them touch the network.
//!
//! Parsers never panic on unexpected markup: rows that do not have the
//! expected shape are skipped and missing cells read as empty strings.

pub mod attendance;
pub mod biometric;
pub mod calendar;
pub mod exam_schedule;
pub mod grade_history;
pub mod grades;
pub mod marks;
pub mod page;
pub mod timetable;

use std::sync::LazyLock;

use scraper::{ElementRef, Selector};

/// Compiles a selector literal. Only ever called with constant strings (all
/// exercised by the parser tests), so a failure is a programming error.
pub(crate) fn selector(css: &'static str) -> Selector {
    #[allow(clippy::expect_used)]
    Selector::parse(css).expect("constant CSS selector must be valid")
}

pub(crate) static TR: LazyLock<Selector> = LazyLock::new(|| selector("tr"));
pub(crate) static TD: LazyLock<Selector> = LazyLock::new(|| selector("td"));
pub(crate) static TH: LazyLock<Selector> = LazyLock::new(|| selector("th"));

/// The element's text with the ends trimmed and every tab and newline
/// removed (inner spaces are kept as they are).
pub(crate) fn compact_text(element: &ElementRef<'_>) -> String {
    let joined: String = element.text().collect();
    joined
        .trim()
        .chars()
        .filter(|character| *character != '\t' && *character != '\n')
        .collect()
}

/// [`compact_text`] for an optional cell; a missing cell is empty.
pub(crate) fn compact_cell(cell: Option<&ElementRef<'_>>) -> String {
    cell.map(compact_text).unwrap_or_default()
}

/// The element's text with all whitespace runs collapsed to one space.
pub(crate) fn word_text(element: &ElementRef<'_>) -> String {
    element
        .text()
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>()
        .join(" ")
}

/// [`word_text`] for an optional cell; a missing cell is empty.
pub(crate) fn word_cell(cell: Option<&ElementRef<'_>>) -> String {
    cell.map(word_text).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use scraper::Html;

    use super::*;

    fn first_td(html: &str) -> String {
        let document = Html::parse_fragment(html);
        let cell = document.select(&TD).next().expect("td");
        format!("{}|{}", compact_text(&cell), word_text(&cell))
    }

    #[test]
    fn text_helpers_match_the_original_cleanup() {
        assert_eq!(
            first_td("<table><tr><td>\t Data\tStructures \n</td></tr></table>"),
            "DataStructures|Data Structures"
        );
        assert_eq!(
            first_td("<table><tr><td><b>A</b>  <i>B</i></td></tr></table>"),
            "A  B|A B"
        );
    }
}
