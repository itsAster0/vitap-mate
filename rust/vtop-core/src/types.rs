use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClassKind {
    Theory,
    Lab,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullAttendanceRecord {
    pub serial: String,
    pub date: String,
    pub slot: String,
    pub day_time: String,
    pub status: String,
    pub remark: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullAttendanceData {
    pub records: Vec<FullAttendanceRecord>,
    pub semester_id: String,
    pub update_time: u64,
    pub course_id: String,
    pub course_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttendanceRecord {
    pub serial: String,
    pub category: String,
    pub course_name: String,
    pub course_code: String,
    pub course_type: String,
    pub faculty_detail: String,
    pub classes_attended: String,
    pub total_classes: String,
    pub attendance_percentage: String,
    pub attendence_fat_cat: String,
    pub debar_status: String,
    pub course_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttendanceData {
    pub records: Vec<AttendanceRecord>,
    pub semester_id: String,
    pub update_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiometricRecord {
    pub serial: String,
    pub punch_date: String,
    pub punch_time: String,
    pub venue: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiometricData {
    pub records: Vec<BiometricRecord>,
    pub requested_date: String,
    pub update_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimetableSlot {
    pub serial: String,
    pub day: String,
    pub slot: String,
    pub course_code: String,
    pub course_type: String,
    pub room_no: String,
    pub block: String,
    pub start_time: String,
    pub end_time: String,
    pub name: String,
    pub kind: ClassKind,
    pub faculty: String,
    #[serde(default)]
    pub credits: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimetableCourse {
    pub course_code: String,
    pub name: String,
    pub course_type: String,
    pub credits: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimetableData {
    pub slots: Vec<TimetableSlot>,
    #[serde(default)]
    pub courses: Vec<TimetableCourse>,
    pub semester_id: String,
    pub update_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarksRecord {
    pub serial: String,
    pub coursecode: String,
    pub coursetitle: String,
    pub coursetype: String,
    pub faculity: String,
    pub slot: String,
    pub marks: Vec<MarksRecordEach>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarksRecordEach {
    pub serial: String,
    pub markstitle: String,
    pub maxmarks: String,
    pub weightage: String,
    pub status: String,
    pub scoredmark: String,
    pub weightagemark: String,
    pub remark: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarksData {
    pub records: Vec<MarksRecord>,
    pub semester_id: String,
    pub update_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExamScheduleRecord {
    pub serial: String,
    pub slot: String,
    pub course_name: String,
    pub course_code: String,
    pub course_type: String,
    pub course_id: String,
    pub exam_date: String,
    pub exam_session: String,
    pub reporting_time: String,
    pub exam_time: String,
    pub venue: String,
    pub seat_location: String,
    pub seat_no: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerExamScheduleRecord {
    pub records: Vec<ExamScheduleRecord>,
    pub exam_type: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExamScheduleData {
    pub exams: Vec<PerExamScheduleRecord>,
    pub semester_id: String,
    pub update_time: u64,
}

/// One event on one day of the academic calendar, e.g. "Holiday" for
/// "General (Semester)" with the note "Deepavali". A day can carry several.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEntry {
    /// `YYYY-MM-DD`.
    pub date: String,
    /// "Instructional Day", "Holiday", "No Instructional Day", "CAT - I", ...
    pub kind: String,
    /// The class group the event applies to, e.g. "General (Semester)".
    pub group: String,
    /// The bracketed note without brackets: "WorkingDay", "LAB FAT", ...
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcademicCalendarData {
    /// Every dated entry of every month of the semester, in date order.
    pub entries: Vec<CalendarEntry>,
    pub semester_id: String,
    pub update_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemesterInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemesterData {
    pub semesters: Vec<SemesterInfo>,
    pub update_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires_at_epoch_ms: Option<u64>,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: Option<String>,
    pub host_only: bool,
    pub persistent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedVtopSession {
    pub username: String,
    pub saved_at_epoch_ms: u64,
    pub cookies: Option<String>,
    /// Saved with the cookies so a restore can skip validating them.
    #[serde(default)]
    pub csrf_token: Option<String>,
    #[serde(default)]
    pub registration_number: Option<String>,
    /// Unix seconds of the last real login (not of the save).
    #[serde(default)]
    pub logged_in_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeCourseRecord {
    pub serial: String,
    pub course_code: String,
    pub course_title: String,
    pub course_type: String,
    pub grading_type: String,
    pub grand_total: String,
    pub grade: String,
    pub course_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeViewData {
    pub courses: Vec<GradeCourseRecord>,
    pub semesters: Vec<SemesterInfo>,
    pub semester_id: String,
    pub update_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeDetailMark {
    pub serial: String,
    pub mark_title: String,
    pub max_mark: String,
    pub weightage: String,
    pub status: String,
    pub scored_mark: String,
    pub weightage_mark: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeRange {
    pub grade: String,
    pub range: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeDetailsData {
    pub semester_id: String,
    pub course_id: String,
    pub class_number: String,
    pub class_course_type: String,
    pub grand_total: String,
    pub marks: Vec<GradeDetailMark>,
    pub grade_ranges: Vec<GradeRange>,
    pub update_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeHistoryStudentInfo {
    pub reg_no: String,
    pub name: String,
    pub programme_branch: String,
    pub programme_mode: String,
    pub study_system: String,
    pub gender: String,
    pub year_joined: String,
    pub edu_status: String,
    pub school: String,
    pub campus: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeHistoryAttempt {
    pub course_code: String,
    pub course_title: String,
    pub course_type: String,
    pub credits: String,
    pub grade: String,
    pub exam_month: String,
    pub result_declared: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeHistoryRecord {
    pub serial: String,
    pub course_code: String,
    pub course_title: String,
    pub course_type: String,
    pub credits: String,
    pub grade: String,
    pub exam_month: String,
    pub result_declared: String,
    pub course_distribution: String,
    pub attempts: Vec<GradeHistoryAttempt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeHistoryCgpa {
    pub credits_registered: String,
    pub credits_earned: String,
    pub cgpa: String,
    pub s_grades: String,
    pub a_grades: String,
    pub b_grades: String,
    pub c_grades: String,
    pub d_grades: String,
    pub e_grades: String,
    pub f_grades: String,
    pub n_grades: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeHistoryData {
    pub student: GradeHistoryStudentInfo,
    pub records: Vec<GradeHistoryRecord>,
    pub cgpa: GradeHistoryCgpa,
    pub update_time: u64,
}

/// The student details VTOP pre-fills on an outing form and expects back
/// when it is submitted. Only present while VTOP is taking applications.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutingStudent {
    pub registration_number: String,
    pub name: String,
    pub application_no: String,
    pub gender: String,
    pub hostel_block: String,
    pub room_number: String,
    pub parent_contact_number: String,
}

/// A `<select>` option: what VTOP expects back and what it shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutingOption {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralOutingRecord {
    pub serial: String,
    pub place: String,
    pub purpose: String,
    /// `YYYY-MM-DD`.
    pub from_date: String,
    /// `hh:mm AM`, as VTOP shows it.
    pub from_time: String,
    pub to_date: String,
    /// Empty on some older requests.
    pub to_time: String,
    pub status: String,
    /// The leave pass id, set once the request is accepted.
    pub pass_id: String,
    /// The id VTOP's delete button carries, set only while VTOP lets the
    /// request be cancelled (before the mentor approves it).
    #[serde(default)]
    pub cancel_id: String,
}

/// The general outing page: the form's limits and the student's requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralOutingData {
    /// `None` when VTOP is not taking applications.
    pub student: Option<OutingStudent>,
    /// VTOP's own message for why the form is shut, if it gave one.
    pub notice: String,
    pub records: Vec<GeneralOutingRecord>,
    /// The hours (24 h) offered for leaving and for coming back. The last
    /// hour of each only allows minute 00.
    pub out_hours: Vec<u8>,
    pub in_hours: Vec<u8>,
    pub place_max_length: u32,
    pub purpose_max_length: u32,
    /// How many days ahead the leaving date may be.
    pub max_days_ahead: u32,
    /// How many days after leaving the return date may be.
    pub max_days_away: u32,
    pub update_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeekendOutingRecord {
    pub serial: String,
    pub hostel_block: String,
    pub room_number: String,
    pub place: String,
    pub purpose: String,
    pub time_slot: String,
    /// `YYYY-MM-DD`.
    pub date: String,
    pub status: String,
    /// The outing form id, set once the request is accepted.
    pub pass_id: String,
    /// The id VTOP's delete button carries, set only while VTOP lets the
    /// request be cancelled.
    #[serde(default)]
    pub cancel_id: String,
}

/// The weekend outing page: the form's choices and the student's requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeekendOutingData {
    /// `None` when VTOP is not taking applications.
    pub student: Option<OutingStudent>,
    pub notice: String,
    pub records: Vec<WeekendOutingRecord>,
    pub places: Vec<OutingOption>,
    pub time_slots: Vec<OutingOption>,
    pub purpose_max_length: u32,
    /// How many days ahead the outing date may be.
    pub max_days_ahead: u32,
    /// Days an outing may fall on, `0` = Sunday … `6` = Saturday.
    pub weekdays: Vec<u8>,
    pub update_time: u64,
}

/// What came of cancelling an outing request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutingCancelResult {
    /// The request is gone from the list.
    pub cancelled: bool,
    pub message: String,
}

/// What came of an outing application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutingApplyResult {
    /// The request now shows in the list.
    pub applied: bool,
    /// VTOP's message, or ours when it gave none.
    pub message: String,
}

/// A course the student is registered for, as the course page lists it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoursePageCourse {
    /// What the course page posts to list the course's classes.
    pub id: String,
    pub code: String,
    pub title: String,
    /// Short type: `ETH`, `ELA`, `TH`, `EPJ`...
    pub course_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoursePageCourses {
    pub semester_id: String,
    pub courses: Vec<CoursePageCourse>,
    pub update_time: u64,
}

/// One class (section) of a course: its slot and faculty.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoursePageClass {
    pub class_id: String,
    /// The faculty's employee id, which opens the class's lecture plan.
    pub erp_id: String,
    pub class_group: String,
    pub course_code: String,
    pub course_title: String,
    /// Long type, e.g. "Embedded Theory".
    pub course_type: String,
    pub slot: String,
    pub faculty: String,
    pub faculty_school: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoursePageClasses {
    pub semester_id: String,
    pub course_id: String,
    pub classes: Vec<CoursePageClass>,
    pub update_time: u64,
}

/// A file link on the course page; `path` is what to download.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseMaterial {
    pub label: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseLecture {
    pub serial: String,
    /// `YYYY-MM-DD`.
    pub date: String,
    /// `TUE`.
    pub day: String,
    pub topic: String,
    pub materials: Vec<CourseMaterial>,
}

/// A class's lecture plan and its downloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoursePageDetail {
    pub semester_id: String,
    pub class: CoursePageClass,
    /// Course id used by the syllabus, e.g. `AM_CSE4007_00100`.
    pub course_id: String,
    pub all_materials_path: String,
    pub general_materials_path: String,
    pub syllabus_path: String,
    pub has_course_plan: bool,
    pub lectures: Vec<CourseLecture>,
    pub update_time: u64,
}
