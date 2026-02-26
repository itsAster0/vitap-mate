use flutter_rust_bridge::frb;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[frb(dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
#[frb]
pub struct FullAttendanceRecord {
    pub serial: String,
    pub date: String,
    pub slot: String,
    pub day_time: String,
    pub status: String,
    pub remark: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[frb(dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
#[frb]
pub struct FullAttendanceData {
    pub records: Vec<FullAttendanceRecord>,
    pub semester_id: String,
    pub update_time: u64,
    pub course_id: String,
    pub course_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[frb(dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
#[frb]
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
#[frb(dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
#[frb]
pub struct AttendanceData {
    pub records: Vec<AttendanceRecord>,
    pub semester_id: String,
    pub update_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[frb(dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
#[frb]
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
    pub is_lab: bool,
    pub faculty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[frb(dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
#[frb]
pub struct TimetableData {
    pub slots: Vec<TimetableSlot>,
    pub semester_id: String,
    pub update_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[frb(dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
#[frb]
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
#[frb(dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
#[frb]
#[frb(json_serializable)]
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
#[frb(dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
#[frb]
pub struct MarksData {
    pub records: Vec<MarksRecord>,
    pub semester_id: String,
    pub update_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[frb(dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
#[frb]
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
#[frb(dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
#[frb]
pub struct PerExamScheduleRecord {
    pub records: Vec<ExamScheduleRecord>,
    pub exam_type: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[frb(dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
#[frb]
pub struct ExamScheduleData {
    pub exams: Vec<PerExamScheduleRecord>,
    pub semester_id: String,
    pub update_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[frb(dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
#[frb]
pub struct SemesterInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[frb(dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
#[frb]
pub struct SemesterData {
    pub semesters: Vec<SemesterInfo>,
    pub update_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[frb(dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
#[frb]
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
#[frb(dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
#[frb]
pub struct GradeViewData {
    pub courses: Vec<GradeCourseRecord>,
    pub semesters: Vec<SemesterInfo>,
    pub semester_id: String,
    pub update_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[frb(dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
#[frb]
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
#[frb(dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
#[frb]
pub struct GradeRange {
    pub grade: String,
    pub range: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[frb(dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
#[frb]
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
#[frb(dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
#[frb]
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
#[frb(dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
#[frb]
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
#[frb(dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
#[frb]
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
#[frb(dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
#[frb]
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
#[frb(dart_metadata=("freezed", "immutable" import "package:meta/meta.dart" as meta),json_serializable)]
#[frb]
pub struct GradeHistoryData {
    pub student: GradeHistoryStudentInfo,
    pub records: Vec<GradeHistoryRecord>,
    pub cgpa: GradeHistoryCgpa,
    pub update_time: u64,
}
