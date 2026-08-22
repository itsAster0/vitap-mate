use scraper::{Html, Selector};
use serde;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::super::types::*;

pub fn parse_timetable(html: String, sem: &str) -> TimetableData {
    #[derive(serde::Serialize, Deserialize)]
    struct Timeing {
        serial: String,
        start_time: String,
        end_time: String,
    }

    let mut classname_code: HashMap<String, String> = HashMap::new();
    let mut facultyname_code: HashMap<String, String> = HashMap::new();
    let mut facultyname_lab_code: HashMap<String, String> = HashMap::new();
    let mut credits_by_component: HashMap<(String, bool), String> = HashMap::new();
    let mut courses: Vec<TimetableCourse> = Vec::new();
    let document = Html::parse_document(&html);
    let rows_selector = Selector::parse("tr").unwrap();
    let mut timetables: Vec<TimetableSlot> = Vec::new();
    let mut timeings_temp_th: Vec<Timeing> = Vec::new();
    let mut timeings_temp_lab: Vec<Timeing> = Vec::new();
    let mut count_for_offset = 0;
    let tabel_selector = Selector::parse("tbody").unwrap();
    let mut table = document.select(&tabel_selector);
    let mut day = "".to_string();

    if let Some(document) = table.next() {
        for row in document.select(&rows_selector) {
            let cells: Vec<_> = row.select(&Selector::parse("td").unwrap()).collect();
            if cells.len() > 8 {
                let cname = cells[2]
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .replace("\t", "")
                    .replace("\n", "");
                let tep = cname
                    .splitn(2, "-")
                    .filter(|k| !k.is_empty())
                    .collect::<Vec<_>>();
                if tep.len() > 1 {
                    let code = tep[0].trim().to_string();
                    let val_str = tep[1].to_string();
                    let temp_val_for = val_str.split_once("(").unwrap_or(("", ""));
                    let name = temp_val_for.0.trim().to_string();
                    let course_type = temp_val_for
                        .1
                        .trim()
                        .trim_end_matches(')')
                        .trim()
                        .to_string();
                    let islab = course_type.to_lowercase().contains("lab");
                    let credits = cells[3]
                        .text()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .split_whitespace()
                        .last()
                        .unwrap_or("")
                        .to_string();

                    credits_by_component.insert((code.clone(), islab), credits);
                    if !courses.iter().any(|course| {
                        course.course_code == code && course.course_type == course_type
                    }) {
                        courses.push(TimetableCourse {
                            course_code: code.clone(),
                            name: name.clone(),
                            course_type,
                            credits: credits_by_component
                                .get(&(code.clone(), islab))
                                .cloned()
                                .unwrap_or_default(),
                        });
                    }

                    if !classname_code.contains_key(&code) {
                        classname_code.insert(code.clone(), name);
                    }
                    let faculty_name = cells[8]
                        .text()
                        .collect::<Vec<_>>()
                        .join("")
                        .trim()
                        .replace("\t", "")
                        .replace("\n", "");
                    if islab {
                        if !facultyname_lab_code.contains_key(&code) {
                            facultyname_lab_code.insert(code, faculty_name);
                        }
                    } else {
                        if !facultyname_code.contains_key(&code) {
                            facultyname_code.insert(code, faculty_name);
                        }
                    }
                }
            }
        }
    }

    if let Some(document) = table.next() {
        for row in document.select(&rows_selector) {
            let mut cells: Vec<_> = row.select(&Selector::parse("td").unwrap()).collect();
            if cells.len() > 6 {
                if count_for_offset % 2 == 0 {
                    day = cells[0]
                        .text()
                        .collect::<Vec<_>>()
                        .join("")
                        .trim()
                        .replace("\t", "")
                        .replace("\n", "");
                    cells.remove(0);
                }

                for (index, val) in cells.iter().enumerate() {
                    if count_for_offset < 4 {
                        if count_for_offset == 0 {
                            let timeing = Timeing {
                                serial: index.to_string(),
                                start_time: val
                                    .text()
                                    .collect::<Vec<_>>()
                                    .join("")
                                    .trim()
                                    .replace("\t", "")
                                    .replace("\n", ""),
                                end_time: "".to_string(),
                            };
                            timeings_temp_th.push(timeing);
                        } else if count_for_offset == 1 {
                            if let Some(timeing) = timeings_temp_th.get_mut(index) {
                                timeing.end_time = val
                                    .text()
                                    .collect::<Vec<_>>()
                                    .join("")
                                    .trim()
                                    .replace("\t", "")
                                    .replace("\n", "");
                            }
                        } else if count_for_offset == 2 {
                            let timeing = Timeing {
                                serial: index.to_string(),
                                start_time: val
                                    .text()
                                    .collect::<Vec<_>>()
                                    .join("")
                                    .trim()
                                    .replace("\t", "")
                                    .replace("\n", ""),
                                end_time: "".to_string(),
                            };
                            timeings_temp_lab.push(timeing);
                        } else if count_for_offset == 3 {
                            if let Some(timeing) = timeings_temp_lab.get_mut(index) {
                                timeing.end_time = val
                                    .text()
                                    .collect::<Vec<_>>()
                                    .join("")
                                    .trim()
                                    .replace("\t", "")
                                    .replace("\n", "");
                            }
                        }
                    } else if count_for_offset > 3 {
                        let class_name = val
                            .text()
                            .collect::<Vec<_>>()
                            .join("")
                            .trim()
                            .replace("\t", "")
                            .replace("\n", "");
                        if class_name.len() > 5 && index != 0 {
                            let cle = class_name
                                .split("-")
                                .filter(|k| !k.is_empty())
                                .collect::<Vec<_>>();
                            if cle.len() > 2 {
                                let is_lab = !count_for_offset % 2 == 0;
                                let mut cl = class_name.split("-");
                                let code = class_name
                                    .split("-")
                                    .nth(1)
                                    .unwrap_or("")
                                    .trim()
                                    .to_string();
                                let class = TimetableSlot {
                                    serial: index.to_string(),
                                    day: day.clone(),
                                    slot: cl.next().unwrap_or("").trim().to_string(),
                                    course_code: cl.next().unwrap_or("").trim().to_string(),
                                    course_type: cl.next().unwrap_or("").trim().to_string(),
                                    room_no: cl.next().unwrap_or("").trim().to_string(),
                                    block: cl.take(2).collect::<Vec<_>>().join(" "),
                                    start_time: "".to_string(),
                                    end_time: "".to_string(),
                                    name: classname_code
                                        .get(&code)
                                        .unwrap_or(&"".to_string())
                                        .to_string(),
                                    is_lab: is_lab,
                                    faculty: if is_lab {
                                        facultyname_lab_code
                                            .get(&code)
                                            .unwrap_or(&"".to_string())
                                            .to_string()
                                    } else {
                                        facultyname_code
                                            .get(&code)
                                            .unwrap_or(&"".to_string())
                                            .to_string()
                                    },
                                    credits: credits_by_component
                                        .get(&(code, is_lab))
                                        .cloned()
                                        .unwrap_or_default(),
                                };
                                timetables.push(class);
                            }
                        }
                    }
                }
                count_for_offset += 1;
            }
        }
    } else {
        return TimetableData {
            slots: timetables,
            courses,
            semester_id: sem.to_string(),
            update_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::new(1, 0))
                .as_secs(),
        };
    }
    for timetable in &mut timetables {
        if let Some(times) = timeings_temp_th
            .iter()
            .find(|t| t.serial == timetable.serial)
        {
            if !timetable.is_lab {
                timetable.start_time = times.start_time.clone();
                timetable.end_time = times.end_time.clone();
            }
        }
    }
    for timetable in &mut timetables {
        if let Some(times) = timeings_temp_lab
            .iter()
            .find(|t| t.serial == timetable.serial)
        {
            if timetable.is_lab {
                timetable.start_time = times.start_time.clone();
                timetable.end_time = times.end_time.clone();
            }
        }
    }

    TimetableData {
        slots: timetables,
        courses,
        semester_id: sem.to_string(),
        update_time: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::new(1, 0))
            .as_secs(),
    }
}
pub fn parse_semid_timetable(html: String) -> SemesterData {
    let mut sem_names_ids = vec![];
    let document = Html::parse_document(&html);
    let selector = Selector::parse(r#"select[name="semesterSubId"] option"#).unwrap();
    for element in document.select(&selector).skip(1) {
        if let Some(value) = element.value().attr("value") {
            if let Some(name) = element.text().next() {
                sem_names_ids.push(SemesterInfo {
                    id: value.trim().to_string(),
                    name: name.trim().replace("- AMR", "").to_string(),
                });
            }
        }
    }
    SemesterData {
        semesters: sem_names_ids,
        update_time: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::new(1, 0))
            .as_secs(),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_timetable;

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

        let timetable = parse_timetable(html, "AP2026");
        let theory = timetable
            .slots
            .iter()
            .find(|slot| !slot.is_lab)
            .expect("theory slot");
        let lab = timetable
            .slots
            .iter()
            .find(|slot| slot.is_lab)
            .expect("lab slot");

        assert_eq!(theory.credits, "3.0");
        assert_eq!(lab.credits, "1.0");
        assert_eq!(timetable.courses.len(), 2);
        assert_eq!(timetable.courses[0].credits, "3.0");
        assert_eq!(timetable.courses[1].credits, "1.0");
    }
}
