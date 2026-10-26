//! The mentor (proctor) pages: `proctor/viewProctorDetails`, a table of
//! label/value rows, and `proctor/viewMessagesSendByProctor`, the messages
//! of the last week.

use std::sync::LazyLock;

use scraper::{ElementRef, Html, Selector};

use super::profile::groups;
use super::{selector, word_text, TD, TH, TR};
use crate::now_unix;
use crate::types::{MentorData, MentorMessage, ProfileField};

static TABLE: LazyLock<Selector> = LazyLock::new(|| selector("table"));

pub fn parse_mentor(details_html: &str, messages_html: &str) -> MentorData {
    let details = Html::parse_document(details_html);
    MentorData {
        details: groups(&details.root_element())
            .into_iter()
            .flat_map(|group| group.fields)
            .collect(),
        messages: parse_messages(messages_html),
        update_time: now_unix(),
    }
}

/// Each table's first row names the columns; every later row is a
/// message, kept as heading/cell pairs since VTOP's columns are not fixed.
/// With no messages the page has no table.
pub fn parse_messages(html: &str) -> Vec<MentorMessage> {
    let document = Html::parse_document(html);
    document
        .select(&TABLE)
        .flat_map(|table| {
            let mut rows = table.select(&TR);
            let headings = rows.next().map(|row| cells(&row)).unwrap_or_default();
            rows.filter_map(move |row| {
                let values = cells(&row);
                if values.iter().all(String::is_empty) {
                    return None;
                }
                let fields = values
                    .into_iter()
                    .enumerate()
                    .map(|(index, value)| ProfileField {
                        label: headings.get(index).cloned().unwrap_or_default(),
                        value,
                    })
                    .collect();
                Some(MentorMessage { fields })
            })
            .collect::<Vec<_>>()
        })
        .collect()
}

fn cells(row: &ElementRef<'_>) -> Vec<String> {
    let headings = row.select(&TH).map(|cell| word_text(&cell));
    let cells = row.select(&TD).map(|cell| word_text(&cell));
    headings.chain(cells).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_details_and_messages() {
        let details = r#"<table><tr><td> Faculty ID</td><td>10001</td><td rowspan="9"><img src="a.png"></td></tr>
            <tr><td>Faculty Email</td><td>a@vitap.ac.in</td></tr></table>"#;
        let messages = r#"<table><tr><th>Sl.No</th><th>Message</th><th>Sent On</th></tr>
            <tr><td>1</td><td>Meet me  at noon</td><td>27-Oct-2026</td></tr></table>"#;
        let data = parse_mentor(details, messages);
        assert_eq!(data.details.len(), 2);
        assert_eq!(data.details[0].label, "Faculty ID");
        assert_eq!(data.details[1].value, "a@vitap.ac.in");
        assert_eq!(data.messages.len(), 1);
        assert_eq!(data.messages[0].fields[1].label, "Message");
        assert_eq!(data.messages[0].fields[1].value, "Meet me at noon");
    }

    #[test]
    fn no_messages_is_empty() {
        let html = "<h3>No Messages Sent by Mentee/Faculty Adviser</h3>";
        assert!(parse_messages(html).is_empty());
    }
}
