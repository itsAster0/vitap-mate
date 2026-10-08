//! Dart-facing mirrors of `vtop_core::types`. flutter_rust_bridge reads
//! these `_Name` declarations to generate the Dart classes; the real types
//! live in vtop-core, which has no FRB dependency. Keep the fields in sync
//! with vtop-core (the compiler does not check mirrors).
#![allow(dead_code)]

use flutter_rust_bridge::frb;
pub use vtop_core::SessionState;

/// A session in the portable form vtop-server takes: the cookie header plus
/// the CSRF token and registration number, so the server can skip
/// validating the cookie with VTOP.
#[frb(mirror(SessionState))]
pub struct _SessionState {
    pub cookies: String,
    pub csrf_token: Option<String>,
    pub registration_number: Option<String>,
    pub otp_issued_at: Option<u64>,
    pub logged_in_at: Option<u64>,
}
pub use vtop_core::client::CourseFileInfo;
pub use vtop_core::types::{
    AcademicCalendarData, AttendanceData, AttendanceRecord, BiometricData, BiometricRecord,
    CalendarEntry, ClassKind, CourseLecture, CourseMaterial, CoursePageClass, CoursePageClasses,
    CoursePageCourse, CoursePageCourses, CoursePageDetail, ExamScheduleData, ExamScheduleRecord,
    FullAttendanceData, FullAttendanceRecord, GeneralOutingData, GeneralOutingRecord,
    GradeCourseRecord, GradeDetailMark, GradeDetailsData, GradeHistoryAttempt, GradeHistoryCgpa,
    GradeHistoryData, GradeHistoryRecord, GradeHistoryStudentInfo, GradeRange, GradeViewData,
    MarksData, MarksRecord, MarksRecordEach, OutingApplyResult, OutingCancelResult, OutingOption,
    OutingStudent, PerExamScheduleRecord, PersistedCookie, PersistedHeader, PersistedVtopSession,
    SemesterData, SemesterInfo, TimetableCourse, TimetableData, TimetableSlot, WeekendOutingData,
    WeekendOutingRecord,
};

#[frb(mirror(ClassKind), non_opaque)]
pub enum _ClassKind {
    Theory,
    Lab,
}
#[frb(mirror(FullAttendanceRecord), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _FullAttendanceRecord {
    pub serial: String,
    pub date: String,
    pub slot: String,
    pub day_time: String,
    pub status: String,
    pub remark: String,
}

#[frb(mirror(FullAttendanceData), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _FullAttendanceData {
    pub records: Vec<FullAttendanceRecord>,
    pub semester_id: String,
    pub update_time: u64,
    pub course_id: String,
    pub course_type: String,
}

#[frb(mirror(AttendanceRecord), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _AttendanceRecord {
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

#[frb(mirror(AttendanceData), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _AttendanceData {
    pub records: Vec<AttendanceRecord>,
    pub semester_id: String,
    pub update_time: u64,
}

#[frb(mirror(BiometricRecord), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _BiometricRecord {
    pub serial: String,
    pub punch_date: String,
    pub punch_time: String,
    pub venue: String,
}

#[frb(mirror(BiometricData), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _BiometricData {
    pub records: Vec<BiometricRecord>,
    pub requested_date: String,
    pub update_time: u64,
}

#[frb(mirror(TimetableSlot), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _TimetableSlot {
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
    pub credits: String,
}

#[frb(mirror(TimetableCourse), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _TimetableCourse {
    pub course_code: String,
    pub name: String,
    pub course_type: String,
    pub credits: String,
}

#[frb(mirror(TimetableData), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _TimetableData {
    pub slots: Vec<TimetableSlot>,
    pub courses: Vec<TimetableCourse>,
    pub semester_id: String,
    pub update_time: u64,
}

#[frb(mirror(MarksRecord), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _MarksRecord {
    pub serial: String,
    pub coursecode: String,
    pub coursetitle: String,
    pub coursetype: String,
    pub faculity: String,
    pub slot: String,
    pub marks: Vec<MarksRecordEach>,
}
#[frb(mirror(MarksRecordEach), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
#[frb(json_serializable)]
pub struct _MarksRecordEach {
    pub serial: String,
    pub markstitle: String,
    pub maxmarks: String,
    pub weightage: String,
    pub status: String,
    pub scoredmark: String,
    pub weightagemark: String,
    pub remark: String,
}

#[frb(mirror(MarksData), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _MarksData {
    pub records: Vec<MarksRecord>,
    pub semester_id: String,
    pub update_time: u64,
}

#[frb(mirror(ExamScheduleRecord), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _ExamScheduleRecord {
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

#[frb(mirror(PerExamScheduleRecord), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _PerExamScheduleRecord {
    pub records: Vec<ExamScheduleRecord>,
    pub exam_type: String,
}
#[frb(mirror(ExamScheduleData), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _ExamScheduleData {
    pub exams: Vec<PerExamScheduleRecord>,
    pub semester_id: String,
    pub update_time: u64,
}

#[frb(mirror(CalendarEntry), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _CalendarEntry {
    pub date: String,
    pub kind: String,
    pub group: String,
    pub note: String,
}

#[frb(mirror(AcademicCalendarData), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _AcademicCalendarData {
    pub entries: Vec<CalendarEntry>,
    pub semester_id: String,
    pub update_time: u64,
}

#[frb(mirror(SemesterInfo), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _SemesterInfo {
    pub id: String,
    pub name: String,
}

#[frb(mirror(SemesterData), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _SemesterData {
    pub semesters: Vec<SemesterInfo>,
    pub update_time: u64,
}

#[frb(mirror(PersistedHeader), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _PersistedHeader {
    pub name: String,
    pub value: String,
}

#[frb(mirror(PersistedCookie), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _PersistedCookie {
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

#[frb(mirror(PersistedVtopSession), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _PersistedVtopSession {
    pub username: String,
    pub saved_at_epoch_ms: u64,
    pub cookies: Option<String>,
    pub csrf_token: Option<String>,
    pub registration_number: Option<String>,
    pub logged_in_at: Option<u64>,
}

#[frb(mirror(GradeCourseRecord), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _GradeCourseRecord {
    pub serial: String,
    pub course_code: String,
    pub course_title: String,
    pub course_type: String,
    pub grading_type: String,
    pub grand_total: String,
    pub grade: String,
    pub course_id: String,
}

#[frb(mirror(GradeViewData), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _GradeViewData {
    pub courses: Vec<GradeCourseRecord>,
    pub semesters: Vec<SemesterInfo>,
    pub semester_id: String,
    pub update_time: u64,
}

#[frb(mirror(GradeDetailMark), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _GradeDetailMark {
    pub serial: String,
    pub mark_title: String,
    pub max_mark: String,
    pub weightage: String,
    pub status: String,
    pub scored_mark: String,
    pub weightage_mark: String,
}

#[frb(mirror(GradeRange), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _GradeRange {
    pub grade: String,
    pub range: String,
}

#[frb(mirror(GradeDetailsData), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _GradeDetailsData {
    pub semester_id: String,
    pub course_id: String,
    pub class_number: String,
    pub class_course_type: String,
    pub grand_total: String,
    pub marks: Vec<GradeDetailMark>,
    pub grade_ranges: Vec<GradeRange>,
    pub update_time: u64,
}

#[frb(mirror(GradeHistoryStudentInfo), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _GradeHistoryStudentInfo {
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

#[frb(mirror(GradeHistoryAttempt), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _GradeHistoryAttempt {
    pub course_code: String,
    pub course_title: String,
    pub course_type: String,
    pub credits: String,
    pub grade: String,
    pub exam_month: String,
    pub result_declared: String,
}

#[frb(mirror(GradeHistoryRecord), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _GradeHistoryRecord {
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

#[frb(mirror(GradeHistoryCgpa), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _GradeHistoryCgpa {
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

#[frb(mirror(GradeHistoryData), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
pub struct _GradeHistoryData {
    pub student: GradeHistoryStudentInfo,
    pub records: Vec<GradeHistoryRecord>,
    pub cgpa: GradeHistoryCgpa,
    pub update_time: u64,
}

#[frb(mirror(OutingStudent), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta))]
pub struct _OutingStudent {
    pub registration_number: String,
    pub name: String,
    pub application_no: String,
    pub gender: String,
    pub hostel_block: String,
    pub room_number: String,
    pub parent_contact_number: String,
}

#[frb(mirror(OutingOption), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta))]
pub struct _OutingOption {
    pub value: String,
    pub label: String,
}

#[frb(mirror(GeneralOutingRecord), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta))]
pub struct _GeneralOutingRecord {
    pub serial: String,
    pub place: String,
    pub purpose: String,
    pub from_date: String,
    pub from_time: String,
    pub to_date: String,
    pub to_time: String,
    pub status: String,
    pub pass_id: String,
    pub cancel_id: String,
}

#[frb(mirror(GeneralOutingData), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta))]
pub struct _GeneralOutingData {
    pub student: Option<OutingStudent>,
    pub notice: String,
    pub records: Vec<GeneralOutingRecord>,
    pub out_hours: Vec<u8>,
    pub in_hours: Vec<u8>,
    pub place_max_length: u32,
    pub purpose_max_length: u32,
    pub max_days_ahead: u32,
    pub max_days_away: u32,
    pub update_time: u64,
}

#[frb(mirror(WeekendOutingRecord), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta))]
pub struct _WeekendOutingRecord {
    pub serial: String,
    pub hostel_block: String,
    pub room_number: String,
    pub place: String,
    pub purpose: String,
    pub time_slot: String,
    pub date: String,
    pub status: String,
    pub pass_id: String,
    pub cancel_id: String,
}

#[frb(mirror(WeekendOutingData), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta))]
pub struct _WeekendOutingData {
    pub student: Option<OutingStudent>,
    pub notice: String,
    pub records: Vec<WeekendOutingRecord>,
    pub places: Vec<OutingOption>,
    pub time_slots: Vec<OutingOption>,
    pub purpose_max_length: u32,
    pub max_days_ahead: u32,
    pub weekdays: Vec<u8>,
    pub update_time: u64,
}

#[frb(mirror(OutingApplyResult), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta))]
pub struct _OutingApplyResult {
    pub applied: bool,
    pub message: String,
}

#[frb(mirror(CoursePageCourse), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta))]
pub struct _CoursePageCourse {
    pub id: String,
    pub code: String,
    pub title: String,
    pub course_type: String,
}

#[frb(mirror(CoursePageCourses), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta))]
pub struct _CoursePageCourses {
    pub semester_id: String,
    pub courses: Vec<CoursePageCourse>,
    pub update_time: u64,
}

#[frb(mirror(CoursePageClass), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta))]
pub struct _CoursePageClass {
    pub class_id: String,
    pub erp_id: String,
    pub class_group: String,
    pub course_code: String,
    pub course_title: String,
    pub course_type: String,
    pub slot: String,
    pub faculty: String,
    pub faculty_school: String,
}

#[frb(mirror(CoursePageClasses), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta))]
pub struct _CoursePageClasses {
    pub semester_id: String,
    pub course_id: String,
    pub classes: Vec<CoursePageClass>,
    pub update_time: u64,
}

#[frb(mirror(CourseMaterial), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta))]
pub struct _CourseMaterial {
    pub label: String,
    pub path: String,
}

#[frb(mirror(CourseLecture), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta))]
pub struct _CourseLecture {
    pub serial: String,
    pub date: String,
    pub day: String,
    pub topic: String,
    pub materials: Vec<CourseMaterial>,
}

#[frb(mirror(CoursePageDetail), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta))]
pub struct _CoursePageDetail {
    pub semester_id: String,
    pub class: CoursePageClass,
    pub course_id: String,
    pub all_materials_path: String,
    pub general_materials_path: String,
    pub syllabus_path: String,
    pub has_course_plan: bool,
    pub lectures: Vec<CourseLecture>,
    pub update_time: u64,
}

#[frb(mirror(CourseFileInfo))]
pub struct _CourseFileInfo {
    pub content_disposition: String,
    pub content_type: String,
}

#[frb(mirror(OutingCancelResult), dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta))]
pub struct _OutingCancelResult {
    pub cancelled: bool,
    pub message: String,
}
