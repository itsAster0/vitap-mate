//! The student profile (`studentsRecord/StudentProfileAllView`).
//!
//! The page is a header card (photo, name, register number, VIT email,
//! program, school) and an accordion of label/value tables: personal,
//! educational, family, proctor and hostel information. The tables are read
//! generically, so their rows stay in VTOP's order and wording.

use std::sync::LazyLock;

use scraper::{ElementRef, Html, Selector};

use super::{selector, word_text, TD, TR};
use crate::now_unix;
use crate::types::{ProfileField, ProfileGroup, ProfileSection, StudentProfile};

static LABEL: LazyLock<Selector> = LazyLock::new(|| selector("label"));
static PARAGRAPH: LazyLock<Selector> = LazyLock::new(|| selector("p"));
static PHOTO: LazyLock<Selector> = LazyLock::new(|| selector("img[src^='data:']"));
static SECTION: LazyLock<Selector> = LazyLock::new(|| selector(".accordion-item"));
static SECTION_TITLE: LazyLock<Selector> = LazyLock::new(|| selector(".accordion-header"));

pub fn parse_profile(html: &str) -> StudentProfile {
    let document = Html::parse_document(html);
    let header = header_fields(&document);
    let header_value = |key: &str| {
        header
            .iter()
            .find(|(label, _)| label == key)
            .map(|(_, value)| value.clone())
            .unwrap_or_default()
    };
    let sections = document
        .select(&SECTION)
        .map(|item| ProfileSection {
            title: item
                .select(&SECTION_TITLE)
                .next()
                .map(|title| word_text(&title))
                .unwrap_or_default(),
            groups: groups(&item),
        })
        .filter(|section| !section.groups.is_empty())
        .collect::<Vec<_>>();

    let mut name = document
        .select(&PARAGRAPH)
        .map(|p| word_text(&p))
        .find(|text| !text.is_empty())
        .unwrap_or_default();
    if name.is_empty() {
        name = field(&sections, "STUDENT NAME");
    }
    let mut registration_number = header_value("REGISTER NUMBER");
    if registration_number.is_empty() {
        registration_number = field(&sections, "Register Number");
    }

    StudentProfile {
        name,
        registration_number,
        vit_email: header_value("VIT EMAIL"),
        program: header_value("PROGRAM & BRANCH"),
        school: header_value("SCHOOL NAME"),
        photo: photo(&document),
        sections,
        update_time: now_unix(),
    }
}

/// The header card's `LABEL:` / value label pairs, keys without the colon.
fn header_fields(document: &Html) -> Vec<(String, String)> {
    let labels = document
        .select(&LABEL)
        .map(|label| word_text(&label))
        .collect::<Vec<_>>();
    labels
        .windows(2)
        .filter_map(|pair| {
            let key = pair[0].strip_suffix(':')?.trim();
            (!pair[1].ends_with(':')).then(|| (key.to_owned(), pair[1].clone()))
        })
        .collect()
}

/// A section's rows: a lone (spanning) cell starts a new group, two or
/// more cells are a label and its value; anything after the value, like
/// the proctor's picture, is left out.
pub(crate) fn groups(item: &ElementRef<'_>) -> Vec<ProfileGroup> {
    let mut groups = vec![ProfileGroup {
        title: String::new(),
        fields: Vec::new(),
    }];
    for row in item.select(&TR) {
        let cells = row.select(&TD).collect::<Vec<_>>();
        match cells.as_slice() {
            [title] => {
                let title = word_text(title);
                if !title.is_empty() {
                    groups.push(ProfileGroup {
                        title,
                        fields: Vec::new(),
                    });
                }
            }
            [label, value, ..] => {
                let label = word_text(label);
                if label.is_empty() {
                    continue;
                }
                if let Some(group) = groups.last_mut() {
                    group.fields.push(ProfileField {
                        label,
                        value: word_text(value),
                    });
                }
            }
            [] => {}
        }
    }
    groups.retain(|group| !group.fields.is_empty());
    groups
}

/// The first value labelled `label` (any case) in any section.
fn field(sections: &[ProfileSection], label: &str) -> String {
    sections
        .iter()
        .flat_map(|section| &section.groups)
        .flat_map(|group| &group.fields)
        .find(|field| field.label.eq_ignore_ascii_case(label))
        .map(|field| field.value.clone())
        .unwrap_or_default()
}

/// The inline photo's base64, without the `data:...;base64,` prefix.
fn photo(document: &Html) -> String {
    document
        .select(&PHOTO)
        .filter_map(|img| img.value().attr("src"))
        .find_map(|src| src.split_once("base64,"))
        .map(|(_, data)| data.chars().filter(|c| !c.is_whitespace()).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_header_groups_and_photo() {
        let html = r#"<div><img src="data:null;base64,/9j/AB CD=" /><p> JANE DOE </p>
            <label>REGISTER NUMBER:</label> <label>24ABC0001</label>
            <label>PROGRAM &amp; BRANCH:</label> <label>BTECH - CSE</label></div>
            <div class="accordion-item"><h2 class="accordion-header"><button><strong>PERSONAL
              INFORMATION</strong></button></h2><table>
              <tr><td>BLOOD GROUP</td><td colspan="2">O+</td></tr>
              <tr><td colspan="3">CURRENT ADDRESS</td></tr>
              <tr><td>CITY</td><td>GUNTUR</td><td></td></tr></table></div>
            <div class="accordion-item"><h2 class="accordion-header">EMPTY</h2></div>"#;
        let profile = parse_profile(html);
        assert_eq!(profile.name, "JANE DOE");
        assert_eq!(profile.registration_number, "24ABC0001");
        assert_eq!(profile.program, "BTECH - CSE");
        assert_eq!(profile.photo, "/9j/ABCD=");
        assert_eq!(profile.sections.len(), 1);
        let section = &profile.sections[0];
        assert_eq!(section.title, "PERSONAL INFORMATION");
        assert_eq!(section.groups.len(), 2);
        assert_eq!(section.groups[0].fields[0].value, "O+");
        assert_eq!(section.groups[1].title, "CURRENT ADDRESS");
        assert_eq!(section.groups[1].fields[0].label, "CITY");
    }
}
