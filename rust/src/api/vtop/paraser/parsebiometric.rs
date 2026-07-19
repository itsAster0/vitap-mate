use scraper::{Html, Selector};

use super::super::types::{BiometricData, BiometricRecord};
use super::super::vtop_client::now_unix;

fn cell_text(cell: scraper::ElementRef<'_>) -> String {
    cell.text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn parse_biometric(html: String, requested_date: String) -> BiometricData {
    let document = Html::parse_document(&html);
    let row_selector = Selector::parse("tr").expect("valid row selector");
    let cell_selector = Selector::parse("td").expect("valid cell selector");

    let records = document
        .select(&row_selector)
        .filter_map(|row| {
            let cells = row.select(&cell_selector).collect::<Vec<_>>();
            if cells.len() != 4 {
                return None;
            }

            let serial = cell_text(cells[0]);
            if serial.eq_ignore_ascii_case("sl.no") || serial.parse::<u32>().is_err() {
                return None;
            }

            Some(BiometricRecord {
                serial,
                punch_date: cell_text(cells[1]),
                punch_time: cell_text(cells[2]),
                venue: cell_text(cells[3]),
            })
        })
        .collect();

    BiometricData {
        records,
        requested_date,
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
        let data = parse_biometric(html.into(), "18/07/2026".into());
        assert_eq!(data.records.len(), 1);
        assert_eq!(data.records[0].punch_time, "19:38");
        assert_eq!(data.records[0].venue, "MH2-IN-6-(349)");
    }
}
