//! Course page: this semester's courses grouped by code, a course's sections
//! (the student's one found from the timetable) and a section's lecture plan
//! with its material, and the files saved to the app's files dir. Port of
//! the Flutter app's `lib/features/course_page/` (`course_page_logic.dart`,
//! `course_page_service.dart`).

use std::collections::HashSet;

use tracing::info;
use vtop_core::VtopError;
use vtop_core::inputs::{CourseFilePath, SemesterId};
use vtop_core::spec;
use vtop_core::types::{
    CoursePageClass, CoursePageClasses, CoursePageCourse, CoursePageCourses, CoursePageDetail,
    TimetableData,
};

use crate::pages::records;
use crate::{Core, CoreError, TIMETABLE, traced, traced_sync};

const COURSES: &str = spec::COURSE_PAGE_COURSES.file;
/// A course's sections: `course_page/classes/<course id>.json`.
const CLASSES: &str = spec::COURSE_PAGE_CLASSES.file;
/// A section's lecture plan: `course_page/plan/<class id>.json`.
const PLANS: &str = spec::COURSE_PAGE_DETAIL.file;

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct CoursePage {
    pub semester_id: String,
    /// Unix seconds.
    pub updated_at: i64,
    /// By code, in VTOP's order.
    pub groups: Vec<CourseGroup>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct CourseGroup {
    pub code: String,
    pub title: String,
    /// Theory, lab, project..., in VTOP's order.
    pub parts: Vec<CoursePart>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct CoursePart {
    /// What [`Core::fetch_course_view`] takes.
    pub id: String,
    pub code: String,
    pub title: String,
    /// "Theory", "Lab", "Project", "Soft skills", else VTOP's type.
    pub label: String,
    pub lab: bool,
}

/// One course part: its sections and the shown one's lecture plan.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct CourseView {
    /// Unix seconds; the older of the sections and the plan.
    pub updated_at: i64,
    pub course: CoursePart,
    pub sections: Vec<CourseSection>,
    /// The class id shown: the one asked for, else the student's, else the
    /// only one. None: the student picks.
    pub selected: Option<String>,
    /// The selected section's plan.
    pub plan: Option<CoursePlan>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct CourseSection {
    pub class_id: String,
    pub faculty: String,
    pub school: String,
    pub slot: String,
    /// The student's own, by the timetable.
    pub mine: bool,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct CoursePlan {
    pub class_id: String,
    /// What [`Core::download_course_file`] takes; empty when none.
    pub syllabus_path: String,
    pub general_materials_path: String,
    pub all_materials_path: String,
    /// [`Core::download_course_plan`] has a sheet.
    pub has_course_plan: bool,
    /// In VTOP's order (by date).
    pub lectures: Vec<Lecture>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Lecture {
    pub serial: String,
    /// Days since 1970-01-01.
    pub day: Option<i32>,
    /// "TUE".
    pub weekday: String,
    pub topic: String,
    pub materials: Vec<Material>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Material {
    /// "Material II" for VTOP's "Reference Material II".
    pub label: String,
    pub path: String,
}

/// A course page file or outing pass for Android's DownloadManager, which fetches it straight from VTOP: the
/// URL (its CSRF token in the query) and the `Cookie` header, both secrets that are never logged;
/// VTOP's name for it (else a name from the link) and its type ("application/pdf"), when VTOP sends one.
#[derive(Clone, PartialEq, uniffi::Record)]
pub struct FileLink {
    pub url: String,
    pub cookie: String,
    pub name: String,
    pub mime: Option<String>,
}

impl std::fmt::Debug for FileLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileLink")
            .field("name", &self.name)
            .field("mime", &self.mime)
            .finish_non_exhaustive()
    }
}

#[uniffi::export]
impl Core {
    /// The selected semester's cached course list.
    pub fn cached_course_page(&self, flow: String) -> Option<CoursePage> {
        traced_sync("cached_course_page", &flow, || {
            Ok(self
                .read_page::<CoursePageCourses>(COURSES)
                .map(|data| logged(CoursePage::from(data))))
        })
        .ok()
        .flatten()
    }

    /// Fetches the selected semester's course list and caches it.
    pub async fn fetch_course_page(&self, flow: String) -> Result<CoursePage, CoreError> {
        traced("fetch_course_page", &flow, async {
            let data = self.fetch(&spec::COURSE_PAGE_COURSES, None, &[]).await?;
            Ok(logged(data.into()))
        })
        .await
    }

    /// `course_id`'s saved sections and the shown one's plan ([`CourseView::selected`]);
    /// none until both are saved. When `match_timetable` is false, the student's section is
    /// not inferred from the timetable.
    pub fn cached_course_view(
        &self,
        flow: String,
        course_id: String,
        class_id: Option<String>,
        match_timetable: bool,
    ) -> Option<CourseView> {
        traced_sync("cached_course_view", &flow, || {
            let course_id = plain(&course_id)?;
            let Some(classes) = self.read_page::<CoursePageClasses>(&classes_file(course_id))
            else {
                return Ok(None);
            };
            let mut view = self.view(course_id, classes, class_id.as_deref(), match_timetable);
            if let Some(selected) = &view.selected {
                let Some(detail) = self.read_page::<CoursePageDetail>(&plan_file(selected)) else {
                    return Ok(None);
                };
                view.add_plan(detail);
            }
            Ok(Some(logged_view(view)))
        })
        .ok()
        .flatten()
    }

    /// Fetches `course_id`'s sections, then the plan of `class_id` (else the
    /// student's section when `match_timetable` is true, else the only one), and caches both.
    pub async fn fetch_course_view(
        &self,
        flow: String,
        course_id: String,
        class_id: Option<String>,
        match_timetable: bool,
    ) -> Result<CourseView, CoreError> {
        traced("fetch_course_view", &flow, async {
            let course_id = plain(&course_id)?;
            let classes = self
                .fetch(
                    &spec::COURSE_PAGE_CLASSES,
                    Some(&classes_file(course_id)),
                    &[("course_id", course_id)],
                )
                .await?;
            let mut view = self.view(
                course_id,
                classes.clone(),
                class_id.as_deref(),
                match_timetable,
            );
            if let Some(selected) = view.selected.clone() {
                let erp_id = classes
                    .classes
                    .iter()
                    .find(|class| class.class_id == selected)
                    .map(|class| class.erp_id.clone())
                    .unwrap_or_default();
                let detail = self
                    .fetch(
                        &spec::COURSE_PAGE_DETAIL,
                        Some(&plan_file(&selected)),
                        &[("erp_id", &erp_id), ("class_id", &selected)],
                    )
                    .await?;
                view.add_plan(detail);
            }
            Ok(logged_view(view))
        })
        .await
    }

    /// A course page file (a lecture's material, the syllabus, a material bundle; a `path` from
    /// [`CoursePlan`]) as a [`FileLink`], signing in first if needed.
    pub async fn course_file_link(
        &self,
        flow: String,
        path: String,
    ) -> Result<FileLink, CoreError> {
        traced("course_file_link", &flow, async {
            let path = CourseFilePath::parse(&path)?;
            let fallback = path
                .as_str()
                .rsplit('/')
                .next()
                .unwrap_or("file")
                .to_string();
            let link =
                async { signed_in!(self, |client| client.course_page_file_link(&path)) }.await?;
            Ok(file_link(link, &fallback))
        })
        .await
    }

    /// `class_id`'s course plan (an Excel sheet) as [`Self::course_file_link`].
    pub async fn course_plan_link(
        &self,
        flow: String,
        class_id: String,
    ) -> Result<FileLink, CoreError> {
        traced("course_plan_link", &flow, async {
            self.current()?;
            let semester = self
                .read_kept::<vtop_core::types::SemesterData>(crate::SEMESTERS)
                .and_then(|list| self.pick(&list))
                .ok_or(CoreError::SemesterNeeded)?;
            let semester = SemesterId::parse(&semester)?;
            let link =
                async { signed_in!(self, |client| client.course_plan_link(&semester, &class_id)) }
                    .await?;
            Ok(file_link(link, "course-plan"))
        })
        .await
    }
}

impl Core {
    fn view(
        &self,
        course_id: &str,
        classes: CoursePageClasses,
        asked: Option<&str>,
        match_timetable: bool,
    ) -> CourseView {
        let mine = if match_timetable {
            let timetable = self.read_page::<TimetableData>(TIMETABLE);
            my_class(&classes.classes, timetable.as_ref()).map(|c| c.class_id.clone())
        } else {
            None
        };
        let course = self
            .read_page::<CoursePageCourses>(COURSES)
            .and_then(|data| data.courses.into_iter().find(|c| c.id == course_id))
            .map(CoursePart::from)
            .or_else(|| {
                classes.classes.first().map(|class| CoursePart {
                    id: course_id.to_string(),
                    code: class.course_code.clone(),
                    title: class.course_title.clone(),
                    label: class.course_type.clone(),
                    lab: is_lab(&class.course_type),
                })
            })
            .unwrap_or_else(|| CoursePart {
                id: course_id.to_string(),
                code: String::new(),
                title: String::new(),
                label: String::new(),
                lab: false,
            });
        let has = |id: &str| classes.classes.iter().any(|class| class.class_id == id);
        let selected = asked
            .filter(|id| has(id))
            .map(str::to_string)
            .or_else(|| mine.clone())
            .or_else(|| match classes.classes.as_slice() {
                [only] => Some(only.class_id.clone()),
                _ => None,
            });
        CourseView {
            updated_at: classes.update_time as i64,
            course,
            sections: classes
                .classes
                .iter()
                .map(|class| CourseSection {
                    class_id: class.class_id.clone(),
                    faculty: class.faculty.trim().to_string(),
                    school: class.faculty_school.trim().to_string(),
                    slot: class.slot.trim().to_string(),
                    mine: mine.as_deref() == Some(class.class_id.as_str()),
                })
                .collect(),
            selected,
            plan: None,
        }
    }
}

/// `link` with VTOP's name, else `fallback`; its type without parameters.
pub(crate) fn file_link(link: vtop_core::client::FileLink, fallback: &str) -> FileLink {
    let mime = link
        .content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let mime = Some(mime).filter(|mime| !mime.is_empty());
    info!(?mime, "file link ready");
    FileLink {
        url: link.url,
        cookie: link.cookie,
        name: if link.file_name.trim().is_empty() {
            fallback.to_string()
        } else {
            link.file_name
        },
        mime,
    }
}

impl CourseView {
    fn add_plan(&mut self, detail: CoursePageDetail) {
        self.updated_at = self.updated_at.min(detail.update_time as i64);
        self.plan = Some(CoursePlan {
            class_id: detail.class.class_id,
            syllabus_path: detail.syllabus_path,
            general_materials_path: detail.general_materials_path,
            all_materials_path: detail.all_materials_path,
            has_course_plan: detail.has_course_plan,
            lectures: detail
                .lectures
                .into_iter()
                .map(|lecture| Lecture {
                    day: records::epoch_day(&lecture.date),
                    serial: lecture.serial,
                    weekday: lecture.day,
                    topic: lecture.topic.trim().to_string(),
                    materials: lecture
                        .materials
                        .into_iter()
                        .map(|material| Material {
                            label: material_label(&material.label),
                            path: material.path,
                        })
                        .collect(),
                })
                .collect(),
        });
    }
}

impl From<CoursePageCourses> for CoursePage {
    fn from(data: CoursePageCourses) -> Self {
        let mut groups: Vec<CourseGroup> = Vec::new();
        for course in data.courses {
            let part = CoursePart::from(course);
            match groups.iter_mut().find(|group| group.code == part.code) {
                Some(group) => group.parts.push(part),
                None => groups.push(CourseGroup {
                    code: part.code.clone(),
                    title: part.title.clone(),
                    parts: vec![part],
                }),
            }
        }
        Self {
            semester_id: data.semester_id,
            updated_at: data.update_time as i64,
            groups,
        }
    }
}

impl From<CoursePageCourse> for CoursePart {
    fn from(course: CoursePageCourse) -> Self {
        Self {
            label: type_label(&course.course_type),
            lab: is_lab(&course.course_type),
            id: course.id,
            code: course.code.trim().to_string(),
            title: course.title.trim().to_string(),
        }
    }
}

fn logged(page: CoursePage) -> CoursePage {
    info!(
        groups = page.groups.len(),
        parts = page.groups.iter().map(|g| g.parts.len()).sum::<usize>(),
        "course page"
    );
    page
}

fn logged_view(view: CourseView) -> CourseView {
    info!(
        sections = view.sections.len(),
        mine = view.sections.iter().any(|s| s.mine),
        selected = view.selected.is_some(),
        lectures = view.plan.as_ref().map(|p| p.lectures.len()),
        materials =
            view.plan
                .as_ref()
                .map(|p| p.lectures.iter().map(|l| l.materials.len()).sum::<usize>()),
        "course"
    );
    view
}

/// Ids name cache files and go back into form posts: letters and digits only.
fn plain(id: &str) -> Result<&str, CoreError> {
    let id = id.trim();
    if id.is_empty() || id.len() > 40 || !id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(CoreError::Other {
            reason: "not a course page id".to_string(),
        });
    }
    Ok(id)
}

fn classes_file(course_id: &str) -> String {
    format!("{CLASSES}/{course_id}.json")
}

/// Only for ids from VTOP's class list, which [`plain`] would pass; anything
/// else is flattened so it can't leave the cache.
fn plan_file(class_id: &str) -> String {
    let safe: String = class_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format!("{PLANS}/{safe}.json")
}

/// `ETH` → "Theory", `ELA` → "Lab"...
fn type_label(kind: &str) -> String {
    match kind.trim().to_ascii_uppercase().as_str() {
        "TH" | "ETH" => "Theory".to_string(),
        "LO" | "ELA" => "Lab".to_string(),
        "EPJ" | "PJT" => "Project".to_string(),
        "SS" => "Soft skills".to_string(),
        _ => kind.trim().to_string(),
    }
}

fn is_lab(kind: &str) -> bool {
    matches!(kind.trim().to_ascii_uppercase().as_str(), "LO" | "ELA")
        || kind.to_lowercase().contains("lab")
}

/// "Reference Material II" → "Material II".
fn material_label(label: &str) -> String {
    let label = label.trim();
    match label.get(..10) {
        Some(head) if head.eq_ignore_ascii_case("reference ") => {
            label.get(10..).unwrap_or_default().trim().to_string()
        }
        _ => label.to_string(),
    }
}

fn norm(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// "A1/TA1", "A2+TA2" → {a1, ta1}.
fn slot_tokens(value: &str) -> HashSet<String> {
    value
        .to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

/// The class the student attends, judged from the timetable: same course
/// code and faculty, and when the faculty teaches several sections, the one
/// sharing a slot. None when the timetable does not settle it.
fn my_class<'a>(
    classes: &'a [CoursePageClass],
    timetable: Option<&TimetableData>,
) -> Option<&'a CoursePageClass> {
    let (timetable, first) = (timetable?, classes.first()?);
    let mine: Vec<_> = timetable
        .slots
        .iter()
        .filter(|slot| slot.course_code.trim() == first.course_code.trim())
        .collect();
    if mine.is_empty() {
        return None;
    }
    let faculty: HashSet<String> = mine
        .iter()
        .map(|slot| norm(slot.faculty.split(" - ").next().unwrap_or_default()))
        .collect();
    let slots: HashSet<String> = mine
        .iter()
        .flat_map(|slot| slot_tokens(&slot.slot))
        .collect();
    let by_faculty: Vec<_> = classes
        .iter()
        .filter(|class| faculty.contains(&norm(&class.faculty)))
        .collect();
    if let [only] = by_faculty.as_slice() {
        return Some(only);
    }
    let pool = if by_faculty.is_empty() {
        classes.iter().collect()
    } else {
        by_faculty
    };
    let by_slot: Vec<_> = pool
        .into_iter()
        .filter(|class| !slot_tokens(&class.slot).is_disjoint(&slots))
        .collect();
    match by_slot.as_slice() {
        [only] => Some(only),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
