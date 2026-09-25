//! Biometric punch history.

use scraper::Html;

use super::{word_text, TD, TR};
use crate::now_unix;
use crate::types::{BiometricData, BiometricRecord};

pub fn parse_biometric(html: &str, requested_date: &str) -> BiometricData {
    let document = Html::parse_document(html);
    let records = document
        .select(&TR)
        .filter_map(|row| {
            let cells = row.select(&TD).collect::<Vec<_>>();
            let [serial, punch_date, punch_time, venue] = cells.as_slice() else {
                return None;
            };
            let serial = word_text(serial);
            if serial.eq_ignore_ascii_case("sl.no") || serial.parse::<u32>().is_err() {
                return None;
            }
            Some(BiometricRecord {
                serial,
                punch_date: word_text(punch_date),
                punch_time: word_text(punch_time),
                venue: word_text(venue),
            })
        })
        .collect();

    BiometricData {
        records,
        requested_date: requested_date.to_string(),
        update_time: now_unix(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_vtop_biometric_rows_and_skips_header() {
        let html = r#"<table><tr><td><b>Sl.No</b></td><td>Punch Date</td><td>Punch Time</td><td>Venue</td></tr>
            <tr><td>1</td><td>18/07/2026</td><td>19:38</td><td>MH2-IN-6-(349)</td></tr></table>"#;
        let data = parse_biometric(html, "18/07/2026");
        assert_eq!(data.records.len(), 1);
        assert_eq!(data.records[0].punch_time, "19:38");
        assert_eq!(data.records[0].venue, "MH2-IN-6-(349)");
    }
}
