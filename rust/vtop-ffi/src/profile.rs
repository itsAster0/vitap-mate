//! The student profile and the mentor, ready to draw: header, hostel,
//! contacts, addresses, parents, every other field by section, and the
//! mentor with their messages. Cached per account (`pages/profile.json`,
//! `pages/mentor.json`; signing out removes `pages/`). Port of the Flutter
//! app's `lib/features/profile/logic/` (`profile_layout.dart`,
//! `profile_text.dart`) and its cards.

use tracing::{info, warn};
use vtop_core::VtopError;
use vtop_core::types::{MentorData, ProfileField, ProfileGroup, StudentProfile};

use crate::{Core, CoreError, PAGES, traced, traced_sync};

/// Not per semester: straight under `pages/`.
const PROFILE: &str = "profile.json";
const MENTOR: &str = "mentor.json";

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Profile {
    /// In title case.
    pub name: String,
    /// Up to two letters.
    pub initials: String,
    pub registration_number: String,
    pub program: String,
    pub school: String,
    /// VTOP's photo as base64 JPEG; empty without one.
    pub photo: String,
    pub hostel: Option<Hostel>,
    /// VIT email, mobile, parent WhatsApp, personal email.
    pub contacts: Vec<Contact>,
    pub addresses: Vec<Address>,
    pub parents: Vec<Parent>,
    /// Every field the cards above do not show, by VTOP's section.
    pub details: Vec<DetailSection>,
    pub mentor: Option<Mentor>,
    /// Unix seconds.
    pub updated_at: i64,
}

/// "MH-2 · Room 101" over "4 Bed AC · Non-Veg Mess".
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Hostel {
    pub title: String,
    pub line: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ContactKind {
    Phone,
    Email,
    WhatsApp,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Contact {
    pub label: String,
    pub value: String,
    pub kind: ContactKind,
}

/// One address as a line of text ("Current", "3rd Floor, ..., Guntur, Andhra Pradesh 522006, India").
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Address {
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Parent {
    /// "Father", "Mother", "Guardian".
    pub relation: String,
    pub name: Option<String>,
    pub initials: String,
    pub occupation: Option<String>,
    pub contacts: Vec<Contact>,
    /// The other fields with a value.
    pub more: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Field {
    /// In sentence case: "Date of birth".
    pub label: String,
    /// In title case, as shown.
    pub value: String,
    /// VTOP's value, for copying.
    pub raw: String,
    /// An Aadhaar number: shown masked until tapped.
    pub secret: bool,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct FieldGroup {
    /// VTOP's sub-heading; empty for a section's first group.
    pub title: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct DetailSection {
    /// "Personal details", "Education".
    pub title: String,
    pub groups: Vec<FieldGroup>,
    pub count: i32,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Mentor {
    /// None when no mentor is assigned.
    pub name: Option<String>,
    pub initials: String,
    /// Designation, department.
    pub lines: Vec<String>,
    pub cabin: Option<String>,
    pub contacts: Vec<Contact>,
    pub more: Vec<Field>,
    /// VTOP drops them after a week.
    pub messages: Vec<MentorMessage>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct MentorMessage {
    pub body: String,
    /// The other columns: "Date: 01-Nov-2026".
    pub meta: Vec<String>,
}

#[uniffi::export]
impl Core {
    /// The signed-in account's cached profile and mentor.
    pub fn cached_profile(&self, flow: String) -> Option<Profile> {
        traced_sync("cached_profile", &flow, || {
            Ok(self
                .read::<StudentProfile>(&format!("{PAGES}/{PROFILE}"))
                .map(|data| logged(Profile::new(data, self.read(&format!("{PAGES}/{MENTOR}"))))))
        })
        .ok()
        .flatten()
    }

    /// Fetches the profile, then the mentor (kept from the cache when that
    /// fails), and caches both.
    pub async fn fetch_profile(&self, flow: String) -> Result<Profile, CoreError> {
        traced("fetch_profile", &flow, async {
            let data = signed_in!(self, |client| client.profile())?;
            let _ = std::fs::create_dir_all(self.dir.join(PAGES));
            self.write(&format!("{PAGES}/{PROFILE}"), &data);
            let mentor = match signed_in!(self, |client| client.mentor()) {
                Ok(mentor) => {
                    self.write(&format!("{PAGES}/{MENTOR}"), &mentor);
                    Some(mentor)
                }
                Err(error) => {
                    warn!(error = error.kind(), "mentor not fetched, cached copy kept");
                    self.read(&format!("{PAGES}/{MENTOR}"))
                }
            };
            Ok(logged(Profile::new(data, mentor)))
        })
        .await
    }
}

/// Counts only: never a name, number or address.
fn logged(profile: Profile) -> Profile {
    info!(
        sections = profile.details.len(),
        contacts = profile.contacts.len(),
        parents = profile.parents.len(),
        hostel = profile.hostel.is_some(),
        photo = !profile.photo.is_empty(),
        mentor = profile.mentor.as_ref().is_some_and(|m| m.name.is_some()),
        messages = profile.mentor.as_ref().map_or(0, |m| m.messages.len()),
        "profile"
    );
    profile
}

impl Profile {
    fn new(data: StudentProfile, mentor: Option<MentorData>) -> Self {
        let mut hostel = Vec::new();
        let mut addresses = Vec::new();
        let mut parents = Vec::new();
        let mut personal = Vec::new();
        let mut details = Vec::new();
        for section in &data.sections {
            let title = section.title.to_uppercase();
            if title.contains("PROCTOR") {
                continue; // The mentor has its own tab.
            }
            if title.contains("HOSTEL") {
                hostel = section.groups.clone();
                continue;
            }
            let mut rest = Vec::new();
            for group in &section.groups {
                let group_title = group.title.to_uppercase();
                if title.contains("PERSONAL") && group_title.contains("ADDRESS") {
                    addresses.push(group.clone());
                } else if title.contains("FAMILY") && group_title.contains("DETAILS") {
                    parents.push(group.clone());
                } else {
                    rest.push(group.clone());
                }
            }
            if title.contains("PERSONAL") {
                personal = rest.clone();
            }
            let groups: Vec<FieldGroup> = rest
                .iter()
                .map(|group| FieldGroup {
                    title: group.title.trim().to_string(),
                    fields: fields(group.fields.iter().filter(|f| !shown_above(&f.label))),
                })
                .filter(|group| !group.fields.is_empty())
                .collect();
            let count = groups.iter().map(|g| g.fields.len()).sum::<usize>() as i32;
            if count > 0 {
                details.push(DetailSection {
                    title: details_title(&section.title),
                    groups,
                    count,
                });
            }
        }
        let mut contacts = Vec::new();
        if has_value(&data.vit_email) {
            contacts.push(contact("VIT email", &data.vit_email));
        }
        if let Some(mobile) = field_value(&personal, "MOBILE NUMBER") {
            contacts.push(contact("Mobile", &mobile));
        }
        let with_addresses: Vec<ProfileGroup> =
            personal.iter().chain(&addresses).cloned().collect();
        if let Some(whatsapp) = field_value(&with_addresses, "PARENT WHATSAPP NUMBER") {
            contacts.push(contact("Parent WhatsApp", &whatsapp));
        }
        for group in &addresses {
            if let Some(email) = field_value(std::slice::from_ref(group), "EMAIL")
                && !contacts.iter().any(|c| c.value == email)
            {
                contacts.push(contact("Personal email", &email));
            }
        }
        let name = nice_case(&data.name);
        Self {
            initials: initials(&name),
            name,
            registration_number: data.registration_number.trim().to_string(),
            program: data.program.trim().to_string(),
            school: data.school.trim().to_string(),
            photo: data.photo,
            hostel: Hostel::of(&hostel),
            contacts,
            addresses: addresses
                .iter()
                .filter_map(|group| {
                    let text = address(group);
                    (!text.is_empty()).then(|| Address {
                        title: nice_case(strip_suffix(&group.title, "ADDRESS")),
                        text,
                    })
                })
                .collect(),
            parents: parents.iter().map(Parent::of).collect(),
            details,
            mentor: mentor.map(Mentor::of),
            updated_at: data.update_time as i64,
        }
    }
}

impl Hostel {
    /// None without a room.
    fn of(groups: &[ProfileGroup]) -> Option<Self> {
        let room = field_value(groups, "ROOM NO.")?;
        // "MH-2 (MH-2) (MH-2 - Block )" → "MH-2".
        let block = field_value(groups, "BLOCK NAME")
            .map(|b| b.split('(').next().unwrap_or_default().trim().to_string());
        let bed = field_value(groups, "BED TYPE").map(|b| bed_type(&b));
        // "Non-Veg Mess Fee - " → "Non-Veg Mess".
        let mess = field_value(groups, "MESS INFORMATION").map(|m| {
            let trimmed = m.trim_end_matches([' ', '-']);
            let lower = trimmed.to_lowercase();
            lower
                .strip_suffix("fee")
                .map_or(trimmed, |_| &trimmed[..trimmed.len() - 3])
                .trim()
                .to_string()
        });
        let mess = mess.filter(|m| has_value(m));
        let line = [bed, mess].into_iter().flatten().collect::<Vec<_>>();
        Some(Self {
            title: block
                .into_iter()
                .chain([format!("Room {room}")])
                .collect::<Vec<_>>()
                .join(" · "),
            line: (!line.is_empty()).then(|| line.join(" · ")),
        })
    }
}

impl Parent {
    fn of(group: &ProfileGroup) -> Self {
        let groups = std::slice::from_ref(group);
        let relation = nice_case(strip_suffix(&group.title, "DETAILS"));
        let name = group
            .fields
            .iter()
            .find(|f| f.label.to_uppercase().ends_with("NAME") && has_value(&f.value))
            .map(|f| f.value.trim().to_string());
        let occupation = field_value(groups, "OCCUPATION");
        let mobile = field_value(groups, "MOBILE NUMBER");
        let email = field_value(groups, "EMAIL");
        let used = [&name, &occupation, &mobile, &email];
        let more = fields(
            group
                .fields
                .iter()
                .filter(|f| !used.iter().any(|u| u.as_deref() == Some(f.value.trim()))),
        );
        Self {
            initials: initials(&nice_case(name.as_deref().unwrap_or(&relation))),
            name: name.as_deref().map(nice_case),
            relation,
            occupation: occupation.as_deref().map(nice_case),
            contacts: [("Mobile", mobile), ("Email", email)]
                .into_iter()
                .filter_map(|(label, value)| Some(contact(label, &value?)))
                .collect(),
            more,
        }
    }
}

impl Mentor {
    fn of(data: MentorData) -> Self {
        let detail = |label: &str| {
            data.details
                .iter()
                .find(|f| f.label.to_lowercase().contains(label) && has_value(&f.value))
                .map(|f| f.value.trim().to_string())
        };
        let name = detail("name");
        const SHOWN: [&str; 6] = [
            "name",
            "designation",
            "email",
            "mobile",
            "cabin",
            "department",
        ];
        Self {
            initials: name.as_deref().map(initials).unwrap_or_default(),
            lines: [detail("designation"), detail("department")]
                .into_iter()
                .flatten()
                .collect(),
            cabin: detail("cabin"),
            contacts: [("Mobile", detail("mobile")), ("Email", detail("email"))]
                .into_iter()
                .filter_map(|(label, value)| Some(contact(label, &value?)))
                .collect(),
            more: fields(data.details.iter().filter(|f| {
                let label = f.label.to_lowercase();
                !SHOWN.iter().any(|s| label.contains(s))
            })),
            messages: data
                .messages
                .iter()
                .filter_map(|message| {
                    let fields: Vec<&ProfileField> = message
                        .fields
                        .iter()
                        .filter(|f| has_value(&f.value) && !is_serial(&f.label))
                        .collect();
                    // The column headed "message", else the longest cell.
                    let body = fields
                        .iter()
                        .find(|f| f.label.to_lowercase().contains("message"))
                        .or_else(|| fields.iter().max_by_key(|f| f.value.len()))?;
                    Some(MentorMessage {
                        body: body.value.trim().to_string(),
                        meta: fields
                            .iter()
                            .filter(|f| !std::ptr::eq(**f, *body))
                            .map(|f| {
                                if f.label.trim().is_empty() {
                                    f.value.trim().to_string()
                                } else {
                                    format!("{}: {}", f.label.trim(), f.value.trim())
                                }
                            })
                            .collect(),
                    })
                })
                .collect(),
            name,
            updated_at: data.update_time as i64,
        }
    }
}

/// The fields with a value, labelled and cased for display.
fn fields<'a>(source: impl Iterator<Item = &'a ProfileField>) -> Vec<Field> {
    source
        .filter(|f| has_value(&f.value))
        .map(|f| Field {
            label: nice_label(&f.label),
            value: nice_case(&f.value),
            raw: f.value.trim().to_string(),
            secret: is_secret(&f.label),
        })
        .collect()
}

fn contact(label: &str, value: &str) -> Contact {
    Contact {
        label: label.to_string(),
        value: value.trim().to_string(),
        kind: if value.contains('@') {
            ContactKind::Email
        } else if label.to_lowercase().contains("whatsapp") {
            ContactKind::WhatsApp
        } else {
            ContactKind::Phone
        },
    }
}

/// The name is in the header and the contacts have their own card.
fn shown_above(label: &str) -> bool {
    matches!(
        label.trim().to_uppercase().as_str(),
        "STUDENT NAME" | "MOBILE NUMBER" | "PARENT WHATSAPP NUMBER"
    )
}

/// The address lines in VTOP's order, the pincode after the state, India last.
fn address(group: &ProfileGroup) -> String {
    let mut parts = Vec::new();
    let mut pincode = None;
    for field in &group.fields {
        let label = field.label.to_uppercase();
        if !has_value(&field.value)
            || ["PHONE", "EMAIL", "WHATSAPP"]
                .iter()
                .any(|s| label.contains(s))
        {
            continue;
        }
        if label.contains("PIN") {
            pincode = Some(field.value.trim().to_string());
            continue;
        }
        parts.push(
            nice_case(&field.value)
                .split(',')
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .collect::<Vec<_>>()
                .join(", "),
        );
    }
    let country = (parts.len() > 1
        && parts
            .last()
            .is_some_and(|p| p.eq_ignore_ascii_case("india")))
    .then(|| parts.pop())
    .flatten();
    if let (Some(pin), Some(last)) = (pincode, parts.last_mut()) {
        last.push(' ');
        last.push_str(&pin);
    }
    parts.extend(country);
    parts.join(", ")
}

fn strip_suffix<'a>(title: &'a str, suffix: &str) -> &'a str {
    let trimmed = title.trim();
    if trimmed.to_uppercase().ends_with(suffix) {
        trimmed[..trimmed.len() - suffix.len()].trim()
    } else {
        trimmed
    }
}

/// VTOP shows "-", "NA" or nothing for fields that do not apply.
fn has_value(value: &str) -> bool {
    !matches!(
        value.trim().to_uppercase().as_str(),
        "" | "-" | "NA" | "N/A"
    )
}

/// The first shown value labelled `label` (any case) in `groups`.
fn field_value(groups: &[ProfileGroup], label: &str) -> Option<String> {
    groups
        .iter()
        .flat_map(|g| &g.fields)
        .find(|f| f.label.trim().to_uppercase() == label && has_value(&f.value))
        .map(|f| f.value.trim().to_string())
}

fn is_secret(label: &str) -> bool {
    let upper = label.to_uppercase();
    upper.contains("AADHAR") || upper.contains("AADHAAR")
}

fn is_serial(label: &str) -> bool {
    let letters: String = label
        .to_lowercase()
        .chars()
        .filter(char::is_ascii_lowercase)
        .collect();
    matches!(letters.as_str(), "slno" | "sno" | "no")
}

/// "PERSONAL INFORMATION" → "Personal details"; "EDUCATIONAL INFORMATION" → "Education".
fn details_title(title: &str) -> String {
    let trimmed = title.trim();
    let base = trimmed
        .to_uppercase()
        .strip_suffix("INFORMATION")
        .map_or(trimmed, |_| &trimmed[..trimmed.len() - "INFORMATION".len()]);
    let name = nice_label(base);
    if name == "Educational" {
        "Education".to_string()
    } else {
        format!("{name} details")
    }
}

/// "4- BED -AC" → "4 Bed AC".
fn bed_type(value: &str) -> String {
    nice_case(
        &value
            .split('-')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// VTOP's upper-case labels in sentence case, keeping the abbreviations:
/// "EMP.ID(IF EMPLOYED IN VIT)" → "Emp. ID (if employed in VIT)".
fn nice_label(label: &str) -> String {
    let words = label
        .trim()
        .to_lowercase()
        .replace("no.of.", "")
        .replace("emp.id(", "emp. id (")
        .split_whitespace()
        .map(|w| match w {
            "vit" | "vit)" | "id" => w.to_uppercase(),
            _ => w.to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ");
    let mut chars = words.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// Words kept lower case inside a value: "Board of Intermediate".
const MINOR: [&str; 9] = ["and", "of", "the", "for", "to", "in", "at", "on", "by"];
/// Short words that are words, not abbreviations.
const SHORT: [&str; 9] = ["YES", "NO", "MR", "MRS", "DR", "ST", "RD", "OPP", "BED"];

/// An all-caps (or all-lowercase) value in title case: "ANDHRA PRADESH" →
/// "Andhra Pradesh". Emails and mixed case stay; short abbreviations like
/// OC, MPC or B+ stay upper case.
fn nice_case(value: &str) -> String {
    let v = value.trim().replace(',', ", ").replace(",  ", ", ");
    if v.contains('@') || (v != v.to_uppercase() && v != v.to_lowercase()) {
        return v;
    }
    let all_caps = v == v.to_uppercase();
    v.split(' ')
        .enumerate()
        .map(|(i, w)| word(w, i == 0, all_caps))
        .collect::<Vec<_>>()
        .join(" ")
}

fn word(w: &str, first: bool, all_caps: bool) -> String {
    let lower = w.to_lowercase();
    let letters: String = w
        .to_uppercase()
        .chars()
        .filter(char::is_ascii_uppercase)
        .collect();
    if !first && MINOR.contains(&lower.as_str()) {
        return lower;
    }
    if all_caps
        && !letters.is_empty()
        && letters.len() <= 3
        && !SHORT.contains(&letters.as_str())
        && !MINOR.contains(&letters.to_lowercase().as_str())
        && !w.chars().any(|c| c.is_ascii_digit())
    {
        return w.to_uppercase();
    }
    match lower.find(|c: char| c.is_ascii_lowercase()) {
        Some(i) if !lower[..i].chars().any(|c| c.is_ascii_digit()) => {
            format!(
                "{}{}{}",
                &lower[..i],
                lower[i..=i].to_uppercase(),
                &lower[i + 1..]
            )
        }
        _ => lower,
    }
}

fn initials(name: &str) -> String {
    name.split_whitespace()
        .take(2)
        .filter_map(|p| p.chars().next())
        .flat_map(char::to_uppercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(label: &str, value: &str) -> ProfileField {
        ProfileField {
            label: label.into(),
            value: value.into(),
        }
    }

    fn group(title: &str, fields: Vec<ProfileField>) -> ProfileGroup {
        ProfileGroup {
            title: title.into(),
            fields,
        }
    }

    fn profile() -> StudentProfile {
        use vtop_core::types::ProfileSection;
        StudentProfile {
            name: "PRIYA SHARMA".into(),
            registration_number: "22BCE0001".into(),
            vit_email: "a.b@vitapstudent.ac.in".into(),
            program: "B.Tech".into(),
            school: "SCOPE".into(),
            photo: String::new(),
            sections: vec![
                ProfileSection {
                    title: "PERSONAL INFORMATION".into(),
                    groups: vec![
                        group(
                            "",
                            vec![
                                field("STUDENT NAME", "PRIYA SHARMA"),
                                field("DATE OF BIRTH", "01-JAN-2004"),
                                field("MOBILE NUMBER", "9876543210"),
                                field("AADHAR NUMBER", "123412341234"),
                                field("NATIVE LANGUAGE", "-"),
                            ],
                        ),
                        group(
                            "CURRENT ADDRESS",
                            vec![
                                field("STREET", "3RD FLOOR,SAMPLE APARTMENTS"),
                                field("CITY", "GUNTUR"),
                                field("STATE", "ANDHRA PRADESH"),
                                field("PINCODE", "522006"),
                                field("COUNTRY", "INDIA"),
                                field("EMAIL", "me@example.com"),
                            ],
                        ),
                    ],
                },
                ProfileSection {
                    title: "FAMILY INFORMATION".into(),
                    groups: vec![group(
                        "FATHER DETAILS",
                        vec![
                            field("FATHER NAME", "RAVI SHARMA"),
                            field("OCCUPATION", "BUSINESS"),
                            field("MOBILE NUMBER", "9000000000"),
                            field("ANNUAL INCOME", "500000"),
                        ],
                    )],
                },
                ProfileSection {
                    title: "PROCTOR INFORMATION".into(),
                    groups: vec![group("", vec![field("FACULTY NAME", "X")])],
                },
                ProfileSection {
                    title: "HOSTEL INFORMATION".into(),
                    groups: vec![group(
                        "",
                        vec![
                            field("BLOCK NAME", "MH-2 (MH-2) (MH-2 - Block )"),
                            field("ROOM NO.", "101"),
                            field("BED TYPE", "4- BED -AC"),
                            field("MESS INFORMATION", "Non-Veg Mess Fee - "),
                        ],
                    )],
                },
            ],
            update_time: 7,
        }
    }

    #[test]
    fn the_layout_picks_cards_and_keeps_the_rest() {
        let p = Profile::new(profile(), None);
        assert_eq!(
            (p.name.as_str(), p.initials.as_str()),
            ("Priya Sharma", "PS")
        );
        assert_eq!(
            p.hostel,
            Some(Hostel {
                title: "MH-2 · Room 101".into(),
                line: Some("4 Bed AC · Non-Veg Mess".into())
            })
        );
        let contacts: Vec<_> = p
            .contacts
            .iter()
            .map(|c| (c.label.as_str(), c.kind))
            .collect();
        assert_eq!(
            contacts,
            [
                ("VIT email", ContactKind::Email),
                ("Mobile", ContactKind::Phone),
                ("Personal email", ContactKind::Email)
            ]
        );
        assert_eq!(p.addresses[0].title, "Current");
        assert_eq!(
            p.addresses[0].text,
            "3rd Floor, Sample Apartments, Guntur, Andhra Pradesh 522006, India"
        );
        let father = &p.parents[0];
        assert_eq!(
            (father.relation.as_str(), father.name.as_deref()),
            ("Father", Some("Ravi Sharma"))
        );
        assert_eq!(father.more.len(), 1);
        // Personal details lose the name and mobile and the empty field;
        // the proctor and hostel sections are not details.
        let personal = &p.details[0];
        assert_eq!(
            (personal.title.as_str(), personal.count),
            ("Personal details", 2)
        );
        assert!(personal.groups[0].fields[1].secret);
        assert_eq!(p.details.len(), 1, "family keeps only its parents");
    }

    #[test]
    fn mentor_and_messages() {
        let data = MentorData {
            details: vec![
                field("Faculty Name", "Dr. A B"),
                field("Faculty Designation", "Assistant Professor"),
                field("Cabin", "AB1-201"),
                field("Faculty Email", "ab@vitap.ac.in"),
                field("Faculty Intercom", "1234"),
            ],
            messages: vec![vtop_core::types::MentorMessage {
                fields: vec![
                    field("Sl.No.", "1"),
                    field("Date", "01-Nov-2026"),
                    field("Message", "Meet me"),
                ],
            }],
            update_time: 3,
        };
        let m = Mentor::of(data);
        assert_eq!(
            (m.name.as_deref(), m.initials.as_str()),
            (Some("Dr. A B"), "DA")
        );
        assert_eq!(m.cabin.as_deref(), Some("AB1-201"));
        assert_eq!(m.more.len(), 1);
        assert_eq!(m.messages[0].body, "Meet me");
        assert_eq!(m.messages[0].meta, ["Date: 01-Nov-2026"]);
    }

    #[test]
    fn text() {
        assert_eq!(
            nice_case("BOARD OF INTERMEDIATE EDUCATION"),
            "Board of Intermediate Education"
        );
        assert_eq!(nice_case("OC"), "OC");
        assert_eq!(nice_case("Mixed Case"), "Mixed Case");
        assert_eq!(
            nice_label("EMP.ID(IF EMPLOYED IN VIT)"),
            "Emp. ID (if employed in VIT)"
        );
        assert_eq!(details_title("EDUCATIONAL INFORMATION"), "Education");
        assert!(!has_value(" n/a ") && has_value("0"));
        assert!(is_serial("Sl.No.") && !is_serial("Name"));
    }
}
