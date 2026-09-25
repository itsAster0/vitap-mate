//! Registered courses and the weekly timetable grid, plus the semester list
//! from the same page.

use std::collections::HashMap;
use std::sync::LazyLock;

use scraper::{ElementRef, Html, Selector};

use super::{compact_text, selector, TD, TR};
use crate::now_unix;
use crate::types::{
    ClassKind, SemesterData, SemesterInfo, TimetableCourse, TimetableData, TimetableSlot,
};

static TBODY: LazyLock<Selector> = LazyLock::new(|| selector("tbody"));
static SEMESTER_OPTION: LazyLock<Selector> =
    LazyLock::new(|| selector(r#"select[name="semesterSubId"] option"#));

/// What the registration table says about each course.
#[derive(Default)]
struct Registration {
    courses: Vec<TimetableCourse>,
    names: HashMap<String, String>,
    theory_faculty: HashMap<String, String>,
    lab_faculty: HashMap<String, String>,
    credits: HashMap<(String, bool), String>,
}

impl Registration {
    fn parse(table: ElementRef<'_>) -> Self {
        let mut registration = Self::default();
        for row in table.select(&TR) {
            let cells: Vec<_> = row.select(&TD).collect();
            if cells.len() > 8 {
                registration.add_row(&cells);
            }
        }
        registration
    }

    /// Reads a "CODE - Name (Type)" row with credits in cell 3 ("L T P J C")
    /// and faculty in cell 8.
    fn add_row(&mut self, cells: &[ElementRef<'_>]) {
        let course = compact_text(&cells[2]);
        let parts: Vec<_> = course
            .splitn(2, '-')
            .filter(|part| !part.is_empty())
            .collect();
        let [code, rest] = parts.as_slice() else {
            return;
        };
        let code = code.trim().to_string();
        let (name, course_type) = rest.split_once('(').unwrap_or(("", ""));
        let name = name.trim().to_string();
        let course_type = course_type.trim().trim_end_matches(')').trim().to_string();
        let is_lab = course_type.to_lowercase().contains("lab");
        let credits = cells[3]
            .text()
            .flat_map(str::split_whitespace)
            .last()
            .unwrap_or("")
            .to_string();

        self.credits.insert((code.clone(), is_lab), credits.clone());
        if !self
            .courses
            .iter()
            .any(|known| known.course_code == code && known.course_type == course_type)
        {
            self.courses.push(TimetableCourse {
                course_code: code.clone(),
                name: name.clone(),
                course_type,
                credits,
            });
        }
        self.names.entry(code.clone()).or_insert(name);
        let faculty = compact_text(&cells[8]);
        let by_code = if is_lab {
            &mut self.lab_faculty
        } else {
            &mut self.theory_faculty
        };
        by_code.entry(code).or_insert(faculty);
    }
}

/// Start and end times for each grid column, for theory and lab rows.
#[derive(Default)]
struct ColumnTimes {
    theory: Vec<(String, String)>,
    lab: Vec<(String, String)>,
}

impl ColumnTimes {
    fn for_kind(&self, kind: ClassKind) -> &[(String, String)] {
        match kind {
            ClassKind::Theory => &self.theory,
            ClassKind::Lab => &self.lab,
        }
    }
}

/// Reads one grid cell such as `A1-CSE2001-ETH-107-AB1-ALL`
/// (slot-code-type-room-block...). Free slots hold just the slot name.
fn grid_slot(
    text: &str,
    column: usize,
    day: &str,
    kind: ClassKind,
    registration: &Registration,
) -> Option<TimetableSlot> {
    if text.len() <= 5 || column == 0 {
        return None;
    }
    if text.split('-').filter(|part| !part.is_empty()).count() <= 2 {
        return None;
    }
    let is_lab = kind == ClassKind::Lab;
    let code = text.split('-').nth(1).unwrap_or("").trim().to_string();
    let mut parts = text.split('-');
    let mut next = || parts.next().unwrap_or("").trim().to_string();
    let slot = next();
    let course_code = next();
    let course_type = next();
    let room_no = next();
    let block = parts.take(2).collect::<Vec<_>>().join(" ");
    let faculty = if is_lab {
        &registration.lab_faculty
    } else {
        &registration.theory_faculty
    };
    Some(TimetableSlot {
        serial: column.to_string(),
        day: day.to_string(),
        slot,
        course_code,
        course_type,
        room_no,
        block,
        start_time: String::new(),
        end_time: String::new(),
        name: registration.names.get(&code).cloned().unwrap_or_default(),
        kind,
        faculty: faculty.get(&code).cloned().unwrap_or_default(),
        credits: registration
            .credits
            .get(&(code, is_lab))
            .cloned()
            .unwrap_or_default(),
    })
}

/// Walks the timetable grid. The first four rows are theory start, theory
/// end, lab start and lab end times; then each day has a theory row (which
/// starts with the day name) and a lab row.
fn parse_grid(grid: ElementRef<'_>, registration: &Registration) -> Vec<TimetableSlot> {
    let mut times = ColumnTimes::default();
    let mut slots = Vec::new();
    let mut day = String::new();
    let mut row_index = 0usize;

    for row in grid.select(&TR) {
        let mut cells: Vec<_> = row.select(&TD).collect();
        if cells.len() <= 6 {
            continue;
        }
        if row_index.is_multiple_of(2) {
            day = compact_text(&cells[0]);
            cells.remove(0);
        }
        for (column, cell) in cells.iter().enumerate() {
            let text = compact_text(cell);
            match row_index {
                0 => times.theory.push((text, String::new())),
                1 => {
                    if let Some(entry) = times.theory.get_mut(column) {
                        entry.1 = text;
                    }
                }
                2 => times.lab.push((text, String::new())),
                3 => {
                    if let Some(entry) = times.lab.get_mut(column) {
                        entry.1 = text;
                    }
                }
                _ => {
                    let kind = if !row_index.is_multiple_of(2) {
                        ClassKind::Lab
                    } else {
                        ClassKind::Theory
                    };
                    slots.extend(grid_slot(&text, column, &day, kind, registration));
                }
            }
        }
        row_index += 1;
    }

    for slot in &mut slots {
        let column = slot.serial.parse::<usize>().ok();
        if let Some((start, end)) = column.and_then(|column| times.for_kind(slot.kind).get(column))
        {
            slot.start_time = start.clone();
            slot.end_time = end.clone();
        }
    }
    slots
}

pub fn parse_timetable(html: &str, sem: &str) -> TimetableData {
    let document = Html::parse_document(html);
    let mut tables = document.select(&TBODY);
    let registration = tables.next().map(Registration::parse).unwrap_or_default();
    let slots = tables
        .next()
        .map(|grid| parse_grid(grid, &registration))
        .unwrap_or_default();

    TimetableData {
        slots,
        courses: registration.courses,
        semester_id: sem.to_string(),
        update_time: now_unix(),
    }
}

/// The semester picker on the timetable page. The first option is the
/// "choose" placeholder.
pub fn parse_semid_timetable(html: &str) -> SemesterData {
    let document = Html::parse_document(html);
    let semesters = document
        .select(&SEMESTER_OPTION)
        .skip(1)
        .filter_map(|option| {
            let id = option.value().attr("value")?;
            let name = option.text().next()?;
            Some(SemesterInfo {
                id: id.trim().to_string(),
                name: name.trim().replace("- AMR", ""),
            })
        })
        .collect();
    SemesterData {
        semesters,
        update_time: now_unix(),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_timetable;
    use crate::types::ClassKind;

    #[test]
    fn timetable_keeps_component_credit_values() {
        let registration_row = |course_type: &str, credit_line: &str| {
            format!(
                "<tr><td>1</td><td>General</td><td>CSE4007 - Digital Image Processing ({course_type})</td><td>{credit_line}</td><td>-</td><td>Regular</td><td>1</td><td>B2</td><td>Faculty</td><td>Active</td></tr>"
            )
        };
        let cells = |first: &str| {
            format!(
                "<tr><td>{first}</td><td></td><td></td><td></td><td></td><td></td><td></td><td></td><td></td></tr>"
            )
        };
        let html = format!(
            "<table><tbody>{}{}</tbody></table><table><tbody>{}{}{}{}<tr><td>MON</td><td></td><td>B2-CSE4007-ETH-315-CB-X-Y</td><td></td><td></td><td></td><td></td><td></td><td></td></tr><tr><td>MON</td><td>L25-CSE4007-ELA-101-CB-X-Y</td><td></td><td></td><td></td><td></td><td></td><td></td></tr></tbody></table>",
            registration_row("Embedded Theory", "3 0 0 0 3.0"),
            registration_row("Embedded Lab", "0 0 2 0 1.0"),
            cells("08:00"),
            cells("08:50"),
            cells("09:00"),
            cells("10:40"),
        );

        let timetable = parse_timetable(&html, "AP2026");
        let theory = timetable
            .slots
            .iter()
            .find(|slot| slot.kind == ClassKind::Theory)
            .expect("theory slot");
        let lab = timetable
            .slots
            .iter()
            .find(|slot| slot.kind == ClassKind::Lab)
            .expect("lab slot");

        assert_eq!(theory.credits, "3.0");
        assert_eq!(lab.credits, "1.0");
        assert_eq!(timetable.courses.len(), 2);
        assert_eq!(timetable.courses[0].credits, "3.0");
        assert_eq!(timetable.courses[1].credits, "1.0");
    }
}
