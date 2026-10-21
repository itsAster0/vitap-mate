// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'types.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

_AcademicCalendarData _$AcademicCalendarDataFromJson(
  Map<String, dynamic> json,
) => _AcademicCalendarData(
  entries: (json['entries'] as List<dynamic>)
      .map((e) => CalendarEntry.fromJson(e as Map<String, dynamic>))
      .toList(),
  semesterId: json['semesterId'] as String,
  updateTime: BigInt.parse(json['updateTime'] as String),
);

Map<String, dynamic> _$AcademicCalendarDataToJson(
  _AcademicCalendarData instance,
) => <String, dynamic>{
  'entries': instance.entries,
  'semesterId': instance.semesterId,
  'updateTime': instance.updateTime.toString(),
};

_AttendanceData _$AttendanceDataFromJson(Map<String, dynamic> json) =>
    _AttendanceData(
      records: (json['records'] as List<dynamic>)
          .map((e) => AttendanceRecord.fromJson(e as Map<String, dynamic>))
          .toList(),
      semesterId: json['semesterId'] as String,
      updateTime: BigInt.parse(json['updateTime'] as String),
    );

Map<String, dynamic> _$AttendanceDataToJson(_AttendanceData instance) =>
    <String, dynamic>{
      'records': instance.records,
      'semesterId': instance.semesterId,
      'updateTime': instance.updateTime.toString(),
    };

_AttendanceRecord _$AttendanceRecordFromJson(Map<String, dynamic> json) =>
    _AttendanceRecord(
      serial: json['serial'] as String,
      category: json['category'] as String,
      courseName: json['courseName'] as String,
      courseCode: json['courseCode'] as String,
      courseType: json['courseType'] as String,
      facultyDetail: json['facultyDetail'] as String,
      classesAttended: json['classesAttended'] as String,
      totalClasses: json['totalClasses'] as String,
      attendancePercentage: json['attendancePercentage'] as String,
      attendenceFatCat: json['attendenceFatCat'] as String,
      debarStatus: json['debarStatus'] as String,
      courseId: json['courseId'] as String,
    );

Map<String, dynamic> _$AttendanceRecordToJson(_AttendanceRecord instance) =>
    <String, dynamic>{
      'serial': instance.serial,
      'category': instance.category,
      'courseName': instance.courseName,
      'courseCode': instance.courseCode,
      'courseType': instance.courseType,
      'facultyDetail': instance.facultyDetail,
      'classesAttended': instance.classesAttended,
      'totalClasses': instance.totalClasses,
      'attendancePercentage': instance.attendancePercentage,
      'attendenceFatCat': instance.attendenceFatCat,
      'debarStatus': instance.debarStatus,
      'courseId': instance.courseId,
    };

_BiometricData _$BiometricDataFromJson(Map<String, dynamic> json) =>
    _BiometricData(
      records: (json['records'] as List<dynamic>)
          .map((e) => BiometricRecord.fromJson(e as Map<String, dynamic>))
          .toList(),
      requestedDate: json['requestedDate'] as String,
      updateTime: BigInt.parse(json['updateTime'] as String),
    );

Map<String, dynamic> _$BiometricDataToJson(_BiometricData instance) =>
    <String, dynamic>{
      'records': instance.records,
      'requestedDate': instance.requestedDate,
      'updateTime': instance.updateTime.toString(),
    };

_BiometricRecord _$BiometricRecordFromJson(Map<String, dynamic> json) =>
    _BiometricRecord(
      serial: json['serial'] as String,
      punchDate: json['punchDate'] as String,
      punchTime: json['punchTime'] as String,
      venue: json['venue'] as String,
    );

Map<String, dynamic> _$BiometricRecordToJson(_BiometricRecord instance) =>
    <String, dynamic>{
      'serial': instance.serial,
      'punchDate': instance.punchDate,
      'punchTime': instance.punchTime,
      'venue': instance.venue,
    };

_CalendarEntry _$CalendarEntryFromJson(Map<String, dynamic> json) =>
    _CalendarEntry(
      date: json['date'] as String,
      kind: json['kind'] as String,
      group: json['group'] as String,
      note: json['note'] as String,
    );

Map<String, dynamic> _$CalendarEntryToJson(_CalendarEntry instance) =>
    <String, dynamic>{
      'date': instance.date,
      'kind': instance.kind,
      'group': instance.group,
      'note': instance.note,
    };

_CourseFileInfo _$CourseFileInfoFromJson(Map<String, dynamic> json) =>
    _CourseFileInfo(
      contentDisposition: json['contentDisposition'] as String,
      contentType: json['contentType'] as String,
    );

Map<String, dynamic> _$CourseFileInfoToJson(_CourseFileInfo instance) =>
    <String, dynamic>{
      'contentDisposition': instance.contentDisposition,
      'contentType': instance.contentType,
    };

_CourseLecture _$CourseLectureFromJson(Map<String, dynamic> json) =>
    _CourseLecture(
      serial: json['serial'] as String,
      date: json['date'] as String,
      day: json['day'] as String,
      topic: json['topic'] as String,
      materials: (json['materials'] as List<dynamic>)
          .map((e) => CourseMaterial.fromJson(e as Map<String, dynamic>))
          .toList(),
    );

Map<String, dynamic> _$CourseLectureToJson(_CourseLecture instance) =>
    <String, dynamic>{
      'serial': instance.serial,
      'date': instance.date,
      'day': instance.day,
      'topic': instance.topic,
      'materials': instance.materials,
    };

_CourseMaterial _$CourseMaterialFromJson(Map<String, dynamic> json) =>
    _CourseMaterial(
      label: json['label'] as String,
      path: json['path'] as String,
    );

Map<String, dynamic> _$CourseMaterialToJson(_CourseMaterial instance) =>
    <String, dynamic>{'label': instance.label, 'path': instance.path};

_CoursePageClass _$CoursePageClassFromJson(Map<String, dynamic> json) =>
    _CoursePageClass(
      classId: json['classId'] as String,
      erpId: json['erpId'] as String,
      classGroup: json['classGroup'] as String,
      courseCode: json['courseCode'] as String,
      courseTitle: json['courseTitle'] as String,
      courseType: json['courseType'] as String,
      slot: json['slot'] as String,
      faculty: json['faculty'] as String,
      facultySchool: json['facultySchool'] as String,
    );

Map<String, dynamic> _$CoursePageClassToJson(_CoursePageClass instance) =>
    <String, dynamic>{
      'classId': instance.classId,
      'erpId': instance.erpId,
      'classGroup': instance.classGroup,
      'courseCode': instance.courseCode,
      'courseTitle': instance.courseTitle,
      'courseType': instance.courseType,
      'slot': instance.slot,
      'faculty': instance.faculty,
      'facultySchool': instance.facultySchool,
    };

_CoursePageClasses _$CoursePageClassesFromJson(Map<String, dynamic> json) =>
    _CoursePageClasses(
      semesterId: json['semesterId'] as String,
      courseId: json['courseId'] as String,
      classes: (json['classes'] as List<dynamic>)
          .map((e) => CoursePageClass.fromJson(e as Map<String, dynamic>))
          .toList(),
      updateTime: BigInt.parse(json['updateTime'] as String),
    );

Map<String, dynamic> _$CoursePageClassesToJson(_CoursePageClasses instance) =>
    <String, dynamic>{
      'semesterId': instance.semesterId,
      'courseId': instance.courseId,
      'classes': instance.classes,
      'updateTime': instance.updateTime.toString(),
    };

_CoursePageCourse _$CoursePageCourseFromJson(Map<String, dynamic> json) =>
    _CoursePageCourse(
      id: json['id'] as String,
      code: json['code'] as String,
      title: json['title'] as String,
      courseType: json['courseType'] as String,
    );

Map<String, dynamic> _$CoursePageCourseToJson(_CoursePageCourse instance) =>
    <String, dynamic>{
      'id': instance.id,
      'code': instance.code,
      'title': instance.title,
      'courseType': instance.courseType,
    };

_CoursePageCourses _$CoursePageCoursesFromJson(Map<String, dynamic> json) =>
    _CoursePageCourses(
      semesterId: json['semesterId'] as String,
      courses: (json['courses'] as List<dynamic>)
          .map((e) => CoursePageCourse.fromJson(e as Map<String, dynamic>))
          .toList(),
      updateTime: BigInt.parse(json['updateTime'] as String),
    );

Map<String, dynamic> _$CoursePageCoursesToJson(_CoursePageCourses instance) =>
    <String, dynamic>{
      'semesterId': instance.semesterId,
      'courses': instance.courses,
      'updateTime': instance.updateTime.toString(),
    };

_CoursePageDetail _$CoursePageDetailFromJson(Map<String, dynamic> json) =>
    _CoursePageDetail(
      semesterId: json['semesterId'] as String,
      class_: CoursePageClass.fromJson(json['class_'] as Map<String, dynamic>),
      courseId: json['courseId'] as String,
      allMaterialsPath: json['allMaterialsPath'] as String,
      generalMaterialsPath: json['generalMaterialsPath'] as String,
      syllabusPath: json['syllabusPath'] as String,
      hasCoursePlan: json['hasCoursePlan'] as bool,
      lectures: (json['lectures'] as List<dynamic>)
          .map((e) => CourseLecture.fromJson(e as Map<String, dynamic>))
          .toList(),
      updateTime: BigInt.parse(json['updateTime'] as String),
    );

Map<String, dynamic> _$CoursePageDetailToJson(_CoursePageDetail instance) =>
    <String, dynamic>{
      'semesterId': instance.semesterId,
      'class_': instance.class_,
      'courseId': instance.courseId,
      'allMaterialsPath': instance.allMaterialsPath,
      'generalMaterialsPath': instance.generalMaterialsPath,
      'syllabusPath': instance.syllabusPath,
      'hasCoursePlan': instance.hasCoursePlan,
      'lectures': instance.lectures,
      'updateTime': instance.updateTime.toString(),
    };

_ExamScheduleData _$ExamScheduleDataFromJson(Map<String, dynamic> json) =>
    _ExamScheduleData(
      exams: (json['exams'] as List<dynamic>)
          .map((e) => PerExamScheduleRecord.fromJson(e as Map<String, dynamic>))
          .toList(),
      semesterId: json['semesterId'] as String,
      updateTime: BigInt.parse(json['updateTime'] as String),
    );

Map<String, dynamic> _$ExamScheduleDataToJson(_ExamScheduleData instance) =>
    <String, dynamic>{
      'exams': instance.exams,
      'semesterId': instance.semesterId,
      'updateTime': instance.updateTime.toString(),
    };

_ExamScheduleRecord _$ExamScheduleRecordFromJson(Map<String, dynamic> json) =>
    _ExamScheduleRecord(
      serial: json['serial'] as String,
      slot: json['slot'] as String,
      courseName: json['courseName'] as String,
      courseCode: json['courseCode'] as String,
      courseType: json['courseType'] as String,
      courseId: json['courseId'] as String,
      examDate: json['examDate'] as String,
      examSession: json['examSession'] as String,
      reportingTime: json['reportingTime'] as String,
      examTime: json['examTime'] as String,
      venue: json['venue'] as String,
      seatLocation: json['seatLocation'] as String,
      seatNo: json['seatNo'] as String,
    );

Map<String, dynamic> _$ExamScheduleRecordToJson(_ExamScheduleRecord instance) =>
    <String, dynamic>{
      'serial': instance.serial,
      'slot': instance.slot,
      'courseName': instance.courseName,
      'courseCode': instance.courseCode,
      'courseType': instance.courseType,
      'courseId': instance.courseId,
      'examDate': instance.examDate,
      'examSession': instance.examSession,
      'reportingTime': instance.reportingTime,
      'examTime': instance.examTime,
      'venue': instance.venue,
      'seatLocation': instance.seatLocation,
      'seatNo': instance.seatNo,
    };

_FullAttendanceData _$FullAttendanceDataFromJson(Map<String, dynamic> json) =>
    _FullAttendanceData(
      records: (json['records'] as List<dynamic>)
          .map((e) => FullAttendanceRecord.fromJson(e as Map<String, dynamic>))
          .toList(),
      semesterId: json['semesterId'] as String,
      updateTime: BigInt.parse(json['updateTime'] as String),
      courseId: json['courseId'] as String,
      courseType: json['courseType'] as String,
    );

Map<String, dynamic> _$FullAttendanceDataToJson(_FullAttendanceData instance) =>
    <String, dynamic>{
      'records': instance.records,
      'semesterId': instance.semesterId,
      'updateTime': instance.updateTime.toString(),
      'courseId': instance.courseId,
      'courseType': instance.courseType,
    };

_FullAttendanceRecord _$FullAttendanceRecordFromJson(
  Map<String, dynamic> json,
) => _FullAttendanceRecord(
  serial: json['serial'] as String,
  date: json['date'] as String,
  slot: json['slot'] as String,
  dayTime: json['dayTime'] as String,
  status: json['status'] as String,
  remark: json['remark'] as String,
);

Map<String, dynamic> _$FullAttendanceRecordToJson(
  _FullAttendanceRecord instance,
) => <String, dynamic>{
  'serial': instance.serial,
  'date': instance.date,
  'slot': instance.slot,
  'dayTime': instance.dayTime,
  'status': instance.status,
  'remark': instance.remark,
};

_GeneralOutingRecord _$GeneralOutingRecordFromJson(Map<String, dynamic> json) =>
    _GeneralOutingRecord(
      serial: json['serial'] as String,
      place: json['place'] as String,
      purpose: json['purpose'] as String,
      fromDate: json['fromDate'] as String,
      fromTime: json['fromTime'] as String,
      toDate: json['toDate'] as String,
      toTime: json['toTime'] as String,
      status: json['status'] as String,
      passId: json['passId'] as String,
      cancelId: json['cancelId'] as String,
    );

Map<String, dynamic> _$GeneralOutingRecordToJson(
  _GeneralOutingRecord instance,
) => <String, dynamic>{
  'serial': instance.serial,
  'place': instance.place,
  'purpose': instance.purpose,
  'fromDate': instance.fromDate,
  'fromTime': instance.fromTime,
  'toDate': instance.toDate,
  'toTime': instance.toTime,
  'status': instance.status,
  'passId': instance.passId,
  'cancelId': instance.cancelId,
};

_GradeCourseRecord _$GradeCourseRecordFromJson(Map<String, dynamic> json) =>
    _GradeCourseRecord(
      serial: json['serial'] as String,
      courseCode: json['courseCode'] as String,
      courseTitle: json['courseTitle'] as String,
      courseType: json['courseType'] as String,
      gradingType: json['gradingType'] as String,
      grandTotal: json['grandTotal'] as String,
      grade: json['grade'] as String,
      courseId: json['courseId'] as String,
    );

Map<String, dynamic> _$GradeCourseRecordToJson(_GradeCourseRecord instance) =>
    <String, dynamic>{
      'serial': instance.serial,
      'courseCode': instance.courseCode,
      'courseTitle': instance.courseTitle,
      'courseType': instance.courseType,
      'gradingType': instance.gradingType,
      'grandTotal': instance.grandTotal,
      'grade': instance.grade,
      'courseId': instance.courseId,
    };

_GradeDetailMark _$GradeDetailMarkFromJson(Map<String, dynamic> json) =>
    _GradeDetailMark(
      serial: json['serial'] as String,
      markTitle: json['markTitle'] as String,
      maxMark: json['maxMark'] as String,
      weightage: json['weightage'] as String,
      status: json['status'] as String,
      scoredMark: json['scoredMark'] as String,
      weightageMark: json['weightageMark'] as String,
    );

Map<String, dynamic> _$GradeDetailMarkToJson(_GradeDetailMark instance) =>
    <String, dynamic>{
      'serial': instance.serial,
      'markTitle': instance.markTitle,
      'maxMark': instance.maxMark,
      'weightage': instance.weightage,
      'status': instance.status,
      'scoredMark': instance.scoredMark,
      'weightageMark': instance.weightageMark,
    };

_GradeDetailsData _$GradeDetailsDataFromJson(Map<String, dynamic> json) =>
    _GradeDetailsData(
      semesterId: json['semesterId'] as String,
      courseId: json['courseId'] as String,
      classNumber: json['classNumber'] as String,
      classCourseType: json['classCourseType'] as String,
      grandTotal: json['grandTotal'] as String,
      marks: (json['marks'] as List<dynamic>)
          .map((e) => GradeDetailMark.fromJson(e as Map<String, dynamic>))
          .toList(),
      gradeRanges: (json['gradeRanges'] as List<dynamic>)
          .map((e) => GradeRange.fromJson(e as Map<String, dynamic>))
          .toList(),
      updateTime: BigInt.parse(json['updateTime'] as String),
    );

Map<String, dynamic> _$GradeDetailsDataToJson(_GradeDetailsData instance) =>
    <String, dynamic>{
      'semesterId': instance.semesterId,
      'courseId': instance.courseId,
      'classNumber': instance.classNumber,
      'classCourseType': instance.classCourseType,
      'grandTotal': instance.grandTotal,
      'marks': instance.marks,
      'gradeRanges': instance.gradeRanges,
      'updateTime': instance.updateTime.toString(),
    };

_GradeHistoryAttempt _$GradeHistoryAttemptFromJson(Map<String, dynamic> json) =>
    _GradeHistoryAttempt(
      courseCode: json['courseCode'] as String,
      courseTitle: json['courseTitle'] as String,
      courseType: json['courseType'] as String,
      credits: json['credits'] as String,
      grade: json['grade'] as String,
      examMonth: json['examMonth'] as String,
      resultDeclared: json['resultDeclared'] as String,
    );

Map<String, dynamic> _$GradeHistoryAttemptToJson(
  _GradeHistoryAttempt instance,
) => <String, dynamic>{
  'courseCode': instance.courseCode,
  'courseTitle': instance.courseTitle,
  'courseType': instance.courseType,
  'credits': instance.credits,
  'grade': instance.grade,
  'examMonth': instance.examMonth,
  'resultDeclared': instance.resultDeclared,
};

_GradeHistoryCgpa _$GradeHistoryCgpaFromJson(Map<String, dynamic> json) =>
    _GradeHistoryCgpa(
      creditsRegistered: json['creditsRegistered'] as String,
      creditsEarned: json['creditsEarned'] as String,
      cgpa: json['cgpa'] as String,
      sGrades: json['sGrades'] as String,
      aGrades: json['aGrades'] as String,
      bGrades: json['bGrades'] as String,
      cGrades: json['cGrades'] as String,
      dGrades: json['dGrades'] as String,
      eGrades: json['eGrades'] as String,
      fGrades: json['fGrades'] as String,
      nGrades: json['nGrades'] as String,
    );

Map<String, dynamic> _$GradeHistoryCgpaToJson(_GradeHistoryCgpa instance) =>
    <String, dynamic>{
      'creditsRegistered': instance.creditsRegistered,
      'creditsEarned': instance.creditsEarned,
      'cgpa': instance.cgpa,
      'sGrades': instance.sGrades,
      'aGrades': instance.aGrades,
      'bGrades': instance.bGrades,
      'cGrades': instance.cGrades,
      'dGrades': instance.dGrades,
      'eGrades': instance.eGrades,
      'fGrades': instance.fGrades,
      'nGrades': instance.nGrades,
    };

_GradeHistoryData _$GradeHistoryDataFromJson(Map<String, dynamic> json) =>
    _GradeHistoryData(
      student: GradeHistoryStudentInfo.fromJson(
        json['student'] as Map<String, dynamic>,
      ),
      records: (json['records'] as List<dynamic>)
          .map((e) => GradeHistoryRecord.fromJson(e as Map<String, dynamic>))
          .toList(),
      cgpa: GradeHistoryCgpa.fromJson(json['cgpa'] as Map<String, dynamic>),
      updateTime: BigInt.parse(json['updateTime'] as String),
    );

Map<String, dynamic> _$GradeHistoryDataToJson(_GradeHistoryData instance) =>
    <String, dynamic>{
      'student': instance.student,
      'records': instance.records,
      'cgpa': instance.cgpa,
      'updateTime': instance.updateTime.toString(),
    };

_GradeHistoryRecord _$GradeHistoryRecordFromJson(Map<String, dynamic> json) =>
    _GradeHistoryRecord(
      serial: json['serial'] as String,
      courseCode: json['courseCode'] as String,
      courseTitle: json['courseTitle'] as String,
      courseType: json['courseType'] as String,
      credits: json['credits'] as String,
      grade: json['grade'] as String,
      examMonth: json['examMonth'] as String,
      resultDeclared: json['resultDeclared'] as String,
      courseDistribution: json['courseDistribution'] as String,
      attempts: (json['attempts'] as List<dynamic>)
          .map((e) => GradeHistoryAttempt.fromJson(e as Map<String, dynamic>))
          .toList(),
    );

Map<String, dynamic> _$GradeHistoryRecordToJson(_GradeHistoryRecord instance) =>
    <String, dynamic>{
      'serial': instance.serial,
      'courseCode': instance.courseCode,
      'courseTitle': instance.courseTitle,
      'courseType': instance.courseType,
      'credits': instance.credits,
      'grade': instance.grade,
      'examMonth': instance.examMonth,
      'resultDeclared': instance.resultDeclared,
      'courseDistribution': instance.courseDistribution,
      'attempts': instance.attempts,
    };

_GradeHistoryStudentInfo _$GradeHistoryStudentInfoFromJson(
  Map<String, dynamic> json,
) => _GradeHistoryStudentInfo(
  regNo: json['regNo'] as String,
  name: json['name'] as String,
  programmeBranch: json['programmeBranch'] as String,
  programmeMode: json['programmeMode'] as String,
  studySystem: json['studySystem'] as String,
  gender: json['gender'] as String,
  yearJoined: json['yearJoined'] as String,
  eduStatus: json['eduStatus'] as String,
  school: json['school'] as String,
  campus: json['campus'] as String,
);

Map<String, dynamic> _$GradeHistoryStudentInfoToJson(
  _GradeHistoryStudentInfo instance,
) => <String, dynamic>{
  'regNo': instance.regNo,
  'name': instance.name,
  'programmeBranch': instance.programmeBranch,
  'programmeMode': instance.programmeMode,
  'studySystem': instance.studySystem,
  'gender': instance.gender,
  'yearJoined': instance.yearJoined,
  'eduStatus': instance.eduStatus,
  'school': instance.school,
  'campus': instance.campus,
};

_GradeRange _$GradeRangeFromJson(Map<String, dynamic> json) =>
    _GradeRange(grade: json['grade'] as String, range: json['range'] as String);

Map<String, dynamic> _$GradeRangeToJson(_GradeRange instance) =>
    <String, dynamic>{'grade': instance.grade, 'range': instance.range};

_GradeViewData _$GradeViewDataFromJson(Map<String, dynamic> json) =>
    _GradeViewData(
      courses: (json['courses'] as List<dynamic>)
          .map((e) => GradeCourseRecord.fromJson(e as Map<String, dynamic>))
          .toList(),
      semesters: (json['semesters'] as List<dynamic>)
          .map((e) => SemesterInfo.fromJson(e as Map<String, dynamic>))
          .toList(),
      semesterId: json['semesterId'] as String,
      updateTime: BigInt.parse(json['updateTime'] as String),
    );

Map<String, dynamic> _$GradeViewDataToJson(_GradeViewData instance) =>
    <String, dynamic>{
      'courses': instance.courses,
      'semesters': instance.semesters,
      'semesterId': instance.semesterId,
      'updateTime': instance.updateTime.toString(),
    };

_MarksData _$MarksDataFromJson(Map<String, dynamic> json) => _MarksData(
  records: (json['records'] as List<dynamic>)
      .map((e) => MarksRecord.fromJson(e as Map<String, dynamic>))
      .toList(),
  semesterId: json['semesterId'] as String,
  updateTime: BigInt.parse(json['updateTime'] as String),
);

Map<String, dynamic> _$MarksDataToJson(_MarksData instance) =>
    <String, dynamic>{
      'records': instance.records,
      'semesterId': instance.semesterId,
      'updateTime': instance.updateTime.toString(),
    };

_MarksRecord _$MarksRecordFromJson(Map<String, dynamic> json) => _MarksRecord(
  serial: json['serial'] as String,
  coursecode: json['coursecode'] as String,
  coursetitle: json['coursetitle'] as String,
  coursetype: json['coursetype'] as String,
  faculity: json['faculity'] as String,
  slot: json['slot'] as String,
  marks: (json['marks'] as List<dynamic>)
      .map((e) => MarksRecordEach.fromJson(e as Map<String, dynamic>))
      .toList(),
);

Map<String, dynamic> _$MarksRecordToJson(_MarksRecord instance) =>
    <String, dynamic>{
      'serial': instance.serial,
      'coursecode': instance.coursecode,
      'coursetitle': instance.coursetitle,
      'coursetype': instance.coursetype,
      'faculity': instance.faculity,
      'slot': instance.slot,
      'marks': instance.marks,
    };

_MarksRecordEach _$MarksRecordEachFromJson(Map<String, dynamic> json) =>
    _MarksRecordEach(
      serial: json['serial'] as String,
      markstitle: json['markstitle'] as String,
      maxmarks: json['maxmarks'] as String,
      weightage: json['weightage'] as String,
      status: json['status'] as String,
      scoredmark: json['scoredmark'] as String,
      weightagemark: json['weightagemark'] as String,
      remark: json['remark'] as String,
    );

Map<String, dynamic> _$MarksRecordEachToJson(_MarksRecordEach instance) =>
    <String, dynamic>{
      'serial': instance.serial,
      'markstitle': instance.markstitle,
      'maxmarks': instance.maxmarks,
      'weightage': instance.weightage,
      'status': instance.status,
      'scoredmark': instance.scoredmark,
      'weightagemark': instance.weightagemark,
      'remark': instance.remark,
    };

_OutingApplyResult _$OutingApplyResultFromJson(Map<String, dynamic> json) =>
    _OutingApplyResult(
      applied: json['applied'] as bool,
      message: json['message'] as String,
    );

Map<String, dynamic> _$OutingApplyResultToJson(_OutingApplyResult instance) =>
    <String, dynamic>{'applied': instance.applied, 'message': instance.message};

_OutingCancelResult _$OutingCancelResultFromJson(Map<String, dynamic> json) =>
    _OutingCancelResult(
      cancelled: json['cancelled'] as bool,
      message: json['message'] as String,
    );

Map<String, dynamic> _$OutingCancelResultToJson(_OutingCancelResult instance) =>
    <String, dynamic>{
      'cancelled': instance.cancelled,
      'message': instance.message,
    };

_OutingOption _$OutingOptionFromJson(Map<String, dynamic> json) =>
    _OutingOption(
      value: json['value'] as String,
      label: json['label'] as String,
    );

Map<String, dynamic> _$OutingOptionToJson(_OutingOption instance) =>
    <String, dynamic>{'value': instance.value, 'label': instance.label};

_OutingStudent _$OutingStudentFromJson(Map<String, dynamic> json) =>
    _OutingStudent(
      registrationNumber: json['registrationNumber'] as String,
      name: json['name'] as String,
      applicationNo: json['applicationNo'] as String,
      gender: json['gender'] as String,
      hostelBlock: json['hostelBlock'] as String,
      roomNumber: json['roomNumber'] as String,
      parentContactNumber: json['parentContactNumber'] as String,
    );

Map<String, dynamic> _$OutingStudentToJson(_OutingStudent instance) =>
    <String, dynamic>{
      'registrationNumber': instance.registrationNumber,
      'name': instance.name,
      'applicationNo': instance.applicationNo,
      'gender': instance.gender,
      'hostelBlock': instance.hostelBlock,
      'roomNumber': instance.roomNumber,
      'parentContactNumber': instance.parentContactNumber,
    };

_PerExamScheduleRecord _$PerExamScheduleRecordFromJson(
  Map<String, dynamic> json,
) => _PerExamScheduleRecord(
  records: (json['records'] as List<dynamic>)
      .map((e) => ExamScheduleRecord.fromJson(e as Map<String, dynamic>))
      .toList(),
  examType: json['examType'] as String,
);

Map<String, dynamic> _$PerExamScheduleRecordToJson(
  _PerExamScheduleRecord instance,
) => <String, dynamic>{
  'records': instance.records,
  'examType': instance.examType,
};

_PersistedVtopSession _$PersistedVtopSessionFromJson(
  Map<String, dynamic> json,
) => _PersistedVtopSession(
  username: json['username'] as String,
  savedAtEpochMs: BigInt.parse(json['savedAtEpochMs'] as String),
  cookies: json['cookies'] as String?,
  csrfToken: json['csrfToken'] as String?,
  registrationNumber: json['registrationNumber'] as String?,
  loggedInAt: json['loggedInAt'] == null
      ? null
      : BigInt.parse(json['loggedInAt'] as String),
);

Map<String, dynamic> _$PersistedVtopSessionToJson(
  _PersistedVtopSession instance,
) => <String, dynamic>{
  'username': instance.username,
  'savedAtEpochMs': instance.savedAtEpochMs.toString(),
  'cookies': instance.cookies,
  'csrfToken': instance.csrfToken,
  'registrationNumber': instance.registrationNumber,
  'loggedInAt': instance.loggedInAt?.toString(),
};

_SemesterData _$SemesterDataFromJson(Map<String, dynamic> json) =>
    _SemesterData(
      semesters: (json['semesters'] as List<dynamic>)
          .map((e) => SemesterInfo.fromJson(e as Map<String, dynamic>))
          .toList(),
      updateTime: BigInt.parse(json['updateTime'] as String),
    );

Map<String, dynamic> _$SemesterDataToJson(_SemesterData instance) =>
    <String, dynamic>{
      'semesters': instance.semesters,
      'updateTime': instance.updateTime.toString(),
    };

_SemesterInfo _$SemesterInfoFromJson(Map<String, dynamic> json) =>
    _SemesterInfo(id: json['id'] as String, name: json['name'] as String);

Map<String, dynamic> _$SemesterInfoToJson(_SemesterInfo instance) =>
    <String, dynamic>{'id': instance.id, 'name': instance.name};

_TimetableCourse _$TimetableCourseFromJson(Map<String, dynamic> json) =>
    _TimetableCourse(
      courseCode: json['courseCode'] as String,
      name: json['name'] as String,
      courseType: json['courseType'] as String,
      credits: json['credits'] as String,
    );

Map<String, dynamic> _$TimetableCourseToJson(_TimetableCourse instance) =>
    <String, dynamic>{
      'courseCode': instance.courseCode,
      'name': instance.name,
      'courseType': instance.courseType,
      'credits': instance.credits,
    };

_TimetableData _$TimetableDataFromJson(Map<String, dynamic> json) =>
    _TimetableData(
      slots: (json['slots'] as List<dynamic>)
          .map((e) => TimetableSlot.fromJson(e as Map<String, dynamic>))
          .toList(),
      courses: (json['courses'] as List<dynamic>)
          .map((e) => TimetableCourse.fromJson(e as Map<String, dynamic>))
          .toList(),
      semesterId: json['semesterId'] as String,
      updateTime: BigInt.parse(json['updateTime'] as String),
    );

Map<String, dynamic> _$TimetableDataToJson(_TimetableData instance) =>
    <String, dynamic>{
      'slots': instance.slots,
      'courses': instance.courses,
      'semesterId': instance.semesterId,
      'updateTime': instance.updateTime.toString(),
    };

_TimetableSlot _$TimetableSlotFromJson(Map<String, dynamic> json) =>
    _TimetableSlot(
      serial: json['serial'] as String,
      day: json['day'] as String,
      slot: json['slot'] as String,
      courseCode: json['courseCode'] as String,
      courseType: json['courseType'] as String,
      roomNo: json['roomNo'] as String,
      block: json['block'] as String,
      startTime: json['startTime'] as String,
      endTime: json['endTime'] as String,
      name: json['name'] as String,
      kind: $enumDecode(_$ClassKindEnumMap, json['kind']),
      faculty: json['faculty'] as String,
      credits: json['credits'] as String,
    );

Map<String, dynamic> _$TimetableSlotToJson(_TimetableSlot instance) =>
    <String, dynamic>{
      'serial': instance.serial,
      'day': instance.day,
      'slot': instance.slot,
      'courseCode': instance.courseCode,
      'courseType': instance.courseType,
      'roomNo': instance.roomNo,
      'block': instance.block,
      'startTime': instance.startTime,
      'endTime': instance.endTime,
      'name': instance.name,
      'kind': _$ClassKindEnumMap[instance.kind]!,
      'faculty': instance.faculty,
      'credits': instance.credits,
    };

const _$ClassKindEnumMap = {ClassKind.theory: 'theory', ClassKind.lab: 'lab'};

_WeekendOutingRecord _$WeekendOutingRecordFromJson(Map<String, dynamic> json) =>
    _WeekendOutingRecord(
      serial: json['serial'] as String,
      hostelBlock: json['hostelBlock'] as String,
      roomNumber: json['roomNumber'] as String,
      place: json['place'] as String,
      purpose: json['purpose'] as String,
      timeSlot: json['timeSlot'] as String,
      date: json['date'] as String,
      status: json['status'] as String,
      passId: json['passId'] as String,
      cancelId: json['cancelId'] as String,
    );

Map<String, dynamic> _$WeekendOutingRecordToJson(
  _WeekendOutingRecord instance,
) => <String, dynamic>{
  'serial': instance.serial,
  'hostelBlock': instance.hostelBlock,
  'roomNumber': instance.roomNumber,
  'place': instance.place,
  'purpose': instance.purpose,
  'timeSlot': instance.timeSlot,
  'date': instance.date,
  'status': instance.status,
  'passId': instance.passId,
  'cancelId': instance.cancelId,
};
