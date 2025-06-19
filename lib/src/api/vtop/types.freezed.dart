// dart format width=80
// coverage:ignore-file
// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'types.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$AttendanceData {

 List<AttendanceRecord> get records; String get semesterId; BigInt get updateTime;
/// Create a copy of AttendanceData
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$AttendanceDataCopyWith<AttendanceData> get copyWith => _$AttendanceDataCopyWithImpl<AttendanceData>(this as AttendanceData, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AttendanceData&&const DeepCollectionEquality().equals(other.records, records)&&(identical(other.semesterId, semesterId) || other.semesterId == semesterId)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(records),semesterId,updateTime);

@override
String toString() {
  return 'AttendanceData(records: $records, semesterId: $semesterId, updateTime: $updateTime)';
}


}

/// @nodoc
abstract mixin class $AttendanceDataCopyWith<$Res>  {
  factory $AttendanceDataCopyWith(AttendanceData value, $Res Function(AttendanceData) _then) = _$AttendanceDataCopyWithImpl;
@useResult
$Res call({
 List<AttendanceRecord> records, String semesterId, BigInt updateTime
});




}
/// @nodoc
class _$AttendanceDataCopyWithImpl<$Res>
    implements $AttendanceDataCopyWith<$Res> {
  _$AttendanceDataCopyWithImpl(this._self, this._then);

  final AttendanceData _self;
  final $Res Function(AttendanceData) _then;

/// Create a copy of AttendanceData
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? records = null,Object? semesterId = null,Object? updateTime = null,}) {
  return _then(_self.copyWith(
records: null == records ? _self.records : records // ignore: cast_nullable_to_non_nullable
as List<AttendanceRecord>,semesterId: null == semesterId ? _self.semesterId : semesterId // ignore: cast_nullable_to_non_nullable
as String,updateTime: null == updateTime ? _self.updateTime : updateTime // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}

}


/// @nodoc


class _AttendanceData implements AttendanceData {
  const _AttendanceData({required final  List<AttendanceRecord> records, required this.semesterId, required this.updateTime}): _records = records;
  

 final  List<AttendanceRecord> _records;
@override List<AttendanceRecord> get records {
  if (_records is EqualUnmodifiableListView) return _records;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_records);
}

@override final  String semesterId;
@override final  BigInt updateTime;

/// Create a copy of AttendanceData
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$AttendanceDataCopyWith<_AttendanceData> get copyWith => __$AttendanceDataCopyWithImpl<_AttendanceData>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _AttendanceData&&const DeepCollectionEquality().equals(other._records, _records)&&(identical(other.semesterId, semesterId) || other.semesterId == semesterId)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(_records),semesterId,updateTime);

@override
String toString() {
  return 'AttendanceData(records: $records, semesterId: $semesterId, updateTime: $updateTime)';
}


}

/// @nodoc
abstract mixin class _$AttendanceDataCopyWith<$Res> implements $AttendanceDataCopyWith<$Res> {
  factory _$AttendanceDataCopyWith(_AttendanceData value, $Res Function(_AttendanceData) _then) = __$AttendanceDataCopyWithImpl;
@override @useResult
$Res call({
 List<AttendanceRecord> records, String semesterId, BigInt updateTime
});




}
/// @nodoc
class __$AttendanceDataCopyWithImpl<$Res>
    implements _$AttendanceDataCopyWith<$Res> {
  __$AttendanceDataCopyWithImpl(this._self, this._then);

  final _AttendanceData _self;
  final $Res Function(_AttendanceData) _then;

/// Create a copy of AttendanceData
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? records = null,Object? semesterId = null,Object? updateTime = null,}) {
  return _then(_AttendanceData(
records: null == records ? _self._records : records // ignore: cast_nullable_to_non_nullable
as List<AttendanceRecord>,semesterId: null == semesterId ? _self.semesterId : semesterId // ignore: cast_nullable_to_non_nullable
as String,updateTime: null == updateTime ? _self.updateTime : updateTime // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc
mixin _$AttendanceRecord {

 String get serial; String get category; String get courseName; String get courseCode; String get courseType; String get facultyDetail; String get classesAttended; String get totalClasses; String get attendancePercentage; String get attendenceFatCat; String get debarStatus; String get courseId;
/// Create a copy of AttendanceRecord
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$AttendanceRecordCopyWith<AttendanceRecord> get copyWith => _$AttendanceRecordCopyWithImpl<AttendanceRecord>(this as AttendanceRecord, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AttendanceRecord&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.category, category) || other.category == category)&&(identical(other.courseName, courseName) || other.courseName == courseName)&&(identical(other.courseCode, courseCode) || other.courseCode == courseCode)&&(identical(other.courseType, courseType) || other.courseType == courseType)&&(identical(other.facultyDetail, facultyDetail) || other.facultyDetail == facultyDetail)&&(identical(other.classesAttended, classesAttended) || other.classesAttended == classesAttended)&&(identical(other.totalClasses, totalClasses) || other.totalClasses == totalClasses)&&(identical(other.attendancePercentage, attendancePercentage) || other.attendancePercentage == attendancePercentage)&&(identical(other.attendenceFatCat, attendenceFatCat) || other.attendenceFatCat == attendenceFatCat)&&(identical(other.debarStatus, debarStatus) || other.debarStatus == debarStatus)&&(identical(other.courseId, courseId) || other.courseId == courseId));
}


@override
int get hashCode => Object.hash(runtimeType,serial,category,courseName,courseCode,courseType,facultyDetail,classesAttended,totalClasses,attendancePercentage,attendenceFatCat,debarStatus,courseId);

@override
String toString() {
  return 'AttendanceRecord(serial: $serial, category: $category, courseName: $courseName, courseCode: $courseCode, courseType: $courseType, facultyDetail: $facultyDetail, classesAttended: $classesAttended, totalClasses: $totalClasses, attendancePercentage: $attendancePercentage, attendenceFatCat: $attendenceFatCat, debarStatus: $debarStatus, courseId: $courseId)';
}


}

/// @nodoc
abstract mixin class $AttendanceRecordCopyWith<$Res>  {
  factory $AttendanceRecordCopyWith(AttendanceRecord value, $Res Function(AttendanceRecord) _then) = _$AttendanceRecordCopyWithImpl;
@useResult
$Res call({
 String serial, String category, String courseName, String courseCode, String courseType, String facultyDetail, String classesAttended, String totalClasses, String attendancePercentage, String attendenceFatCat, String debarStatus, String courseId
});




}
/// @nodoc
class _$AttendanceRecordCopyWithImpl<$Res>
    implements $AttendanceRecordCopyWith<$Res> {
  _$AttendanceRecordCopyWithImpl(this._self, this._then);

  final AttendanceRecord _self;
  final $Res Function(AttendanceRecord) _then;

/// Create a copy of AttendanceRecord
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? serial = null,Object? category = null,Object? courseName = null,Object? courseCode = null,Object? courseType = null,Object? facultyDetail = null,Object? classesAttended = null,Object? totalClasses = null,Object? attendancePercentage = null,Object? attendenceFatCat = null,Object? debarStatus = null,Object? courseId = null,}) {
  return _then(_self.copyWith(
serial: null == serial ? _self.serial : serial // ignore: cast_nullable_to_non_nullable
as String,category: null == category ? _self.category : category // ignore: cast_nullable_to_non_nullable
as String,courseName: null == courseName ? _self.courseName : courseName // ignore: cast_nullable_to_non_nullable
as String,courseCode: null == courseCode ? _self.courseCode : courseCode // ignore: cast_nullable_to_non_nullable
as String,courseType: null == courseType ? _self.courseType : courseType // ignore: cast_nullable_to_non_nullable
as String,facultyDetail: null == facultyDetail ? _self.facultyDetail : facultyDetail // ignore: cast_nullable_to_non_nullable
as String,classesAttended: null == classesAttended ? _self.classesAttended : classesAttended // ignore: cast_nullable_to_non_nullable
as String,totalClasses: null == totalClasses ? _self.totalClasses : totalClasses // ignore: cast_nullable_to_non_nullable
as String,attendancePercentage: null == attendancePercentage ? _self.attendancePercentage : attendancePercentage // ignore: cast_nullable_to_non_nullable
as String,attendenceFatCat: null == attendenceFatCat ? _self.attendenceFatCat : attendenceFatCat // ignore: cast_nullable_to_non_nullable
as String,debarStatus: null == debarStatus ? _self.debarStatus : debarStatus // ignore: cast_nullable_to_non_nullable
as String,courseId: null == courseId ? _self.courseId : courseId // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// @nodoc


class _AttendanceRecord implements AttendanceRecord {
  const _AttendanceRecord({required this.serial, required this.category, required this.courseName, required this.courseCode, required this.courseType, required this.facultyDetail, required this.classesAttended, required this.totalClasses, required this.attendancePercentage, required this.attendenceFatCat, required this.debarStatus, required this.courseId});
  

@override final  String serial;
@override final  String category;
@override final  String courseName;
@override final  String courseCode;
@override final  String courseType;
@override final  String facultyDetail;
@override final  String classesAttended;
@override final  String totalClasses;
@override final  String attendancePercentage;
@override final  String attendenceFatCat;
@override final  String debarStatus;
@override final  String courseId;

/// Create a copy of AttendanceRecord
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$AttendanceRecordCopyWith<_AttendanceRecord> get copyWith => __$AttendanceRecordCopyWithImpl<_AttendanceRecord>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _AttendanceRecord&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.category, category) || other.category == category)&&(identical(other.courseName, courseName) || other.courseName == courseName)&&(identical(other.courseCode, courseCode) || other.courseCode == courseCode)&&(identical(other.courseType, courseType) || other.courseType == courseType)&&(identical(other.facultyDetail, facultyDetail) || other.facultyDetail == facultyDetail)&&(identical(other.classesAttended, classesAttended) || other.classesAttended == classesAttended)&&(identical(other.totalClasses, totalClasses) || other.totalClasses == totalClasses)&&(identical(other.attendancePercentage, attendancePercentage) || other.attendancePercentage == attendancePercentage)&&(identical(other.attendenceFatCat, attendenceFatCat) || other.attendenceFatCat == attendenceFatCat)&&(identical(other.debarStatus, debarStatus) || other.debarStatus == debarStatus)&&(identical(other.courseId, courseId) || other.courseId == courseId));
}


@override
int get hashCode => Object.hash(runtimeType,serial,category,courseName,courseCode,courseType,facultyDetail,classesAttended,totalClasses,attendancePercentage,attendenceFatCat,debarStatus,courseId);

@override
String toString() {
  return 'AttendanceRecord(serial: $serial, category: $category, courseName: $courseName, courseCode: $courseCode, courseType: $courseType, facultyDetail: $facultyDetail, classesAttended: $classesAttended, totalClasses: $totalClasses, attendancePercentage: $attendancePercentage, attendenceFatCat: $attendenceFatCat, debarStatus: $debarStatus, courseId: $courseId)';
}


}

/// @nodoc
abstract mixin class _$AttendanceRecordCopyWith<$Res> implements $AttendanceRecordCopyWith<$Res> {
  factory _$AttendanceRecordCopyWith(_AttendanceRecord value, $Res Function(_AttendanceRecord) _then) = __$AttendanceRecordCopyWithImpl;
@override @useResult
$Res call({
 String serial, String category, String courseName, String courseCode, String courseType, String facultyDetail, String classesAttended, String totalClasses, String attendancePercentage, String attendenceFatCat, String debarStatus, String courseId
});




}
/// @nodoc
class __$AttendanceRecordCopyWithImpl<$Res>
    implements _$AttendanceRecordCopyWith<$Res> {
  __$AttendanceRecordCopyWithImpl(this._self, this._then);

  final _AttendanceRecord _self;
  final $Res Function(_AttendanceRecord) _then;

/// Create a copy of AttendanceRecord
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? serial = null,Object? category = null,Object? courseName = null,Object? courseCode = null,Object? courseType = null,Object? facultyDetail = null,Object? classesAttended = null,Object? totalClasses = null,Object? attendancePercentage = null,Object? attendenceFatCat = null,Object? debarStatus = null,Object? courseId = null,}) {
  return _then(_AttendanceRecord(
serial: null == serial ? _self.serial : serial // ignore: cast_nullable_to_non_nullable
as String,category: null == category ? _self.category : category // ignore: cast_nullable_to_non_nullable
as String,courseName: null == courseName ? _self.courseName : courseName // ignore: cast_nullable_to_non_nullable
as String,courseCode: null == courseCode ? _self.courseCode : courseCode // ignore: cast_nullable_to_non_nullable
as String,courseType: null == courseType ? _self.courseType : courseType // ignore: cast_nullable_to_non_nullable
as String,facultyDetail: null == facultyDetail ? _self.facultyDetail : facultyDetail // ignore: cast_nullable_to_non_nullable
as String,classesAttended: null == classesAttended ? _self.classesAttended : classesAttended // ignore: cast_nullable_to_non_nullable
as String,totalClasses: null == totalClasses ? _self.totalClasses : totalClasses // ignore: cast_nullable_to_non_nullable
as String,attendancePercentage: null == attendancePercentage ? _self.attendancePercentage : attendancePercentage // ignore: cast_nullable_to_non_nullable
as String,attendenceFatCat: null == attendenceFatCat ? _self.attendenceFatCat : attendenceFatCat // ignore: cast_nullable_to_non_nullable
as String,debarStatus: null == debarStatus ? _self.debarStatus : debarStatus // ignore: cast_nullable_to_non_nullable
as String,courseId: null == courseId ? _self.courseId : courseId // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc
mixin _$ExamScheduleData {

 List<PerExamScheduleRecord> get exams; String get semesterId; BigInt get updateTime;
/// Create a copy of ExamScheduleData
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ExamScheduleDataCopyWith<ExamScheduleData> get copyWith => _$ExamScheduleDataCopyWithImpl<ExamScheduleData>(this as ExamScheduleData, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ExamScheduleData&&const DeepCollectionEquality().equals(other.exams, exams)&&(identical(other.semesterId, semesterId) || other.semesterId == semesterId)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(exams),semesterId,updateTime);

@override
String toString() {
  return 'ExamScheduleData(exams: $exams, semesterId: $semesterId, updateTime: $updateTime)';
}


}

/// @nodoc
abstract mixin class $ExamScheduleDataCopyWith<$Res>  {
  factory $ExamScheduleDataCopyWith(ExamScheduleData value, $Res Function(ExamScheduleData) _then) = _$ExamScheduleDataCopyWithImpl;
@useResult
$Res call({
 List<PerExamScheduleRecord> exams, String semesterId, BigInt updateTime
});




}
/// @nodoc
class _$ExamScheduleDataCopyWithImpl<$Res>
    implements $ExamScheduleDataCopyWith<$Res> {
  _$ExamScheduleDataCopyWithImpl(this._self, this._then);

  final ExamScheduleData _self;
  final $Res Function(ExamScheduleData) _then;

/// Create a copy of ExamScheduleData
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? exams = null,Object? semesterId = null,Object? updateTime = null,}) {
  return _then(_self.copyWith(
exams: null == exams ? _self.exams : exams // ignore: cast_nullable_to_non_nullable
as List<PerExamScheduleRecord>,semesterId: null == semesterId ? _self.semesterId : semesterId // ignore: cast_nullable_to_non_nullable
as String,updateTime: null == updateTime ? _self.updateTime : updateTime // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}

}


/// @nodoc


class _ExamScheduleData implements ExamScheduleData {
  const _ExamScheduleData({required final  List<PerExamScheduleRecord> exams, required this.semesterId, required this.updateTime}): _exams = exams;
  

 final  List<PerExamScheduleRecord> _exams;
@override List<PerExamScheduleRecord> get exams {
  if (_exams is EqualUnmodifiableListView) return _exams;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_exams);
}

@override final  String semesterId;
@override final  BigInt updateTime;

/// Create a copy of ExamScheduleData
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$ExamScheduleDataCopyWith<_ExamScheduleData> get copyWith => __$ExamScheduleDataCopyWithImpl<_ExamScheduleData>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _ExamScheduleData&&const DeepCollectionEquality().equals(other._exams, _exams)&&(identical(other.semesterId, semesterId) || other.semesterId == semesterId)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(_exams),semesterId,updateTime);

@override
String toString() {
  return 'ExamScheduleData(exams: $exams, semesterId: $semesterId, updateTime: $updateTime)';
}


}

/// @nodoc
abstract mixin class _$ExamScheduleDataCopyWith<$Res> implements $ExamScheduleDataCopyWith<$Res> {
  factory _$ExamScheduleDataCopyWith(_ExamScheduleData value, $Res Function(_ExamScheduleData) _then) = __$ExamScheduleDataCopyWithImpl;
@override @useResult
$Res call({
 List<PerExamScheduleRecord> exams, String semesterId, BigInt updateTime
});




}
/// @nodoc
class __$ExamScheduleDataCopyWithImpl<$Res>
    implements _$ExamScheduleDataCopyWith<$Res> {
  __$ExamScheduleDataCopyWithImpl(this._self, this._then);

  final _ExamScheduleData _self;
  final $Res Function(_ExamScheduleData) _then;

/// Create a copy of ExamScheduleData
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? exams = null,Object? semesterId = null,Object? updateTime = null,}) {
  return _then(_ExamScheduleData(
exams: null == exams ? _self._exams : exams // ignore: cast_nullable_to_non_nullable
as List<PerExamScheduleRecord>,semesterId: null == semesterId ? _self.semesterId : semesterId // ignore: cast_nullable_to_non_nullable
as String,updateTime: null == updateTime ? _self.updateTime : updateTime // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc
mixin _$ExamScheduleRecord {

 String get serial; String get slot; String get courseName; String get courseCode; String get courseType; String get courseId; String get examDate; String get examSession; String get reportingTime; String get examTime; String get venue; String get seatLocation; String get seatNo;
/// Create a copy of ExamScheduleRecord
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ExamScheduleRecordCopyWith<ExamScheduleRecord> get copyWith => _$ExamScheduleRecordCopyWithImpl<ExamScheduleRecord>(this as ExamScheduleRecord, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ExamScheduleRecord&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.slot, slot) || other.slot == slot)&&(identical(other.courseName, courseName) || other.courseName == courseName)&&(identical(other.courseCode, courseCode) || other.courseCode == courseCode)&&(identical(other.courseType, courseType) || other.courseType == courseType)&&(identical(other.courseId, courseId) || other.courseId == courseId)&&(identical(other.examDate, examDate) || other.examDate == examDate)&&(identical(other.examSession, examSession) || other.examSession == examSession)&&(identical(other.reportingTime, reportingTime) || other.reportingTime == reportingTime)&&(identical(other.examTime, examTime) || other.examTime == examTime)&&(identical(other.venue, venue) || other.venue == venue)&&(identical(other.seatLocation, seatLocation) || other.seatLocation == seatLocation)&&(identical(other.seatNo, seatNo) || other.seatNo == seatNo));
}


@override
int get hashCode => Object.hash(runtimeType,serial,slot,courseName,courseCode,courseType,courseId,examDate,examSession,reportingTime,examTime,venue,seatLocation,seatNo);

@override
String toString() {
  return 'ExamScheduleRecord(serial: $serial, slot: $slot, courseName: $courseName, courseCode: $courseCode, courseType: $courseType, courseId: $courseId, examDate: $examDate, examSession: $examSession, reportingTime: $reportingTime, examTime: $examTime, venue: $venue, seatLocation: $seatLocation, seatNo: $seatNo)';
}


}

/// @nodoc
abstract mixin class $ExamScheduleRecordCopyWith<$Res>  {
  factory $ExamScheduleRecordCopyWith(ExamScheduleRecord value, $Res Function(ExamScheduleRecord) _then) = _$ExamScheduleRecordCopyWithImpl;
@useResult
$Res call({
 String serial, String slot, String courseName, String courseCode, String courseType, String courseId, String examDate, String examSession, String reportingTime, String examTime, String venue, String seatLocation, String seatNo
});




}
/// @nodoc
class _$ExamScheduleRecordCopyWithImpl<$Res>
    implements $ExamScheduleRecordCopyWith<$Res> {
  _$ExamScheduleRecordCopyWithImpl(this._self, this._then);

  final ExamScheduleRecord _self;
  final $Res Function(ExamScheduleRecord) _then;

/// Create a copy of ExamScheduleRecord
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? serial = null,Object? slot = null,Object? courseName = null,Object? courseCode = null,Object? courseType = null,Object? courseId = null,Object? examDate = null,Object? examSession = null,Object? reportingTime = null,Object? examTime = null,Object? venue = null,Object? seatLocation = null,Object? seatNo = null,}) {
  return _then(_self.copyWith(
serial: null == serial ? _self.serial : serial // ignore: cast_nullable_to_non_nullable
as String,slot: null == slot ? _self.slot : slot // ignore: cast_nullable_to_non_nullable
as String,courseName: null == courseName ? _self.courseName : courseName // ignore: cast_nullable_to_non_nullable
as String,courseCode: null == courseCode ? _self.courseCode : courseCode // ignore: cast_nullable_to_non_nullable
as String,courseType: null == courseType ? _self.courseType : courseType // ignore: cast_nullable_to_non_nullable
as String,courseId: null == courseId ? _self.courseId : courseId // ignore: cast_nullable_to_non_nullable
as String,examDate: null == examDate ? _self.examDate : examDate // ignore: cast_nullable_to_non_nullable
as String,examSession: null == examSession ? _self.examSession : examSession // ignore: cast_nullable_to_non_nullable
as String,reportingTime: null == reportingTime ? _self.reportingTime : reportingTime // ignore: cast_nullable_to_non_nullable
as String,examTime: null == examTime ? _self.examTime : examTime // ignore: cast_nullable_to_non_nullable
as String,venue: null == venue ? _self.venue : venue // ignore: cast_nullable_to_non_nullable
as String,seatLocation: null == seatLocation ? _self.seatLocation : seatLocation // ignore: cast_nullable_to_non_nullable
as String,seatNo: null == seatNo ? _self.seatNo : seatNo // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// @nodoc


class _ExamScheduleRecord implements ExamScheduleRecord {
  const _ExamScheduleRecord({required this.serial, required this.slot, required this.courseName, required this.courseCode, required this.courseType, required this.courseId, required this.examDate, required this.examSession, required this.reportingTime, required this.examTime, required this.venue, required this.seatLocation, required this.seatNo});
  

@override final  String serial;
@override final  String slot;
@override final  String courseName;
@override final  String courseCode;
@override final  String courseType;
@override final  String courseId;
@override final  String examDate;
@override final  String examSession;
@override final  String reportingTime;
@override final  String examTime;
@override final  String venue;
@override final  String seatLocation;
@override final  String seatNo;

/// Create a copy of ExamScheduleRecord
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$ExamScheduleRecordCopyWith<_ExamScheduleRecord> get copyWith => __$ExamScheduleRecordCopyWithImpl<_ExamScheduleRecord>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _ExamScheduleRecord&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.slot, slot) || other.slot == slot)&&(identical(other.courseName, courseName) || other.courseName == courseName)&&(identical(other.courseCode, courseCode) || other.courseCode == courseCode)&&(identical(other.courseType, courseType) || other.courseType == courseType)&&(identical(other.courseId, courseId) || other.courseId == courseId)&&(identical(other.examDate, examDate) || other.examDate == examDate)&&(identical(other.examSession, examSession) || other.examSession == examSession)&&(identical(other.reportingTime, reportingTime) || other.reportingTime == reportingTime)&&(identical(other.examTime, examTime) || other.examTime == examTime)&&(identical(other.venue, venue) || other.venue == venue)&&(identical(other.seatLocation, seatLocation) || other.seatLocation == seatLocation)&&(identical(other.seatNo, seatNo) || other.seatNo == seatNo));
}


@override
int get hashCode => Object.hash(runtimeType,serial,slot,courseName,courseCode,courseType,courseId,examDate,examSession,reportingTime,examTime,venue,seatLocation,seatNo);

@override
String toString() {
  return 'ExamScheduleRecord(serial: $serial, slot: $slot, courseName: $courseName, courseCode: $courseCode, courseType: $courseType, courseId: $courseId, examDate: $examDate, examSession: $examSession, reportingTime: $reportingTime, examTime: $examTime, venue: $venue, seatLocation: $seatLocation, seatNo: $seatNo)';
}


}

/// @nodoc
abstract mixin class _$ExamScheduleRecordCopyWith<$Res> implements $ExamScheduleRecordCopyWith<$Res> {
  factory _$ExamScheduleRecordCopyWith(_ExamScheduleRecord value, $Res Function(_ExamScheduleRecord) _then) = __$ExamScheduleRecordCopyWithImpl;
@override @useResult
$Res call({
 String serial, String slot, String courseName, String courseCode, String courseType, String courseId, String examDate, String examSession, String reportingTime, String examTime, String venue, String seatLocation, String seatNo
});




}
/// @nodoc
class __$ExamScheduleRecordCopyWithImpl<$Res>
    implements _$ExamScheduleRecordCopyWith<$Res> {
  __$ExamScheduleRecordCopyWithImpl(this._self, this._then);

  final _ExamScheduleRecord _self;
  final $Res Function(_ExamScheduleRecord) _then;

/// Create a copy of ExamScheduleRecord
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? serial = null,Object? slot = null,Object? courseName = null,Object? courseCode = null,Object? courseType = null,Object? courseId = null,Object? examDate = null,Object? examSession = null,Object? reportingTime = null,Object? examTime = null,Object? venue = null,Object? seatLocation = null,Object? seatNo = null,}) {
  return _then(_ExamScheduleRecord(
serial: null == serial ? _self.serial : serial // ignore: cast_nullable_to_non_nullable
as String,slot: null == slot ? _self.slot : slot // ignore: cast_nullable_to_non_nullable
as String,courseName: null == courseName ? _self.courseName : courseName // ignore: cast_nullable_to_non_nullable
as String,courseCode: null == courseCode ? _self.courseCode : courseCode // ignore: cast_nullable_to_non_nullable
as String,courseType: null == courseType ? _self.courseType : courseType // ignore: cast_nullable_to_non_nullable
as String,courseId: null == courseId ? _self.courseId : courseId // ignore: cast_nullable_to_non_nullable
as String,examDate: null == examDate ? _self.examDate : examDate // ignore: cast_nullable_to_non_nullable
as String,examSession: null == examSession ? _self.examSession : examSession // ignore: cast_nullable_to_non_nullable
as String,reportingTime: null == reportingTime ? _self.reportingTime : reportingTime // ignore: cast_nullable_to_non_nullable
as String,examTime: null == examTime ? _self.examTime : examTime // ignore: cast_nullable_to_non_nullable
as String,venue: null == venue ? _self.venue : venue // ignore: cast_nullable_to_non_nullable
as String,seatLocation: null == seatLocation ? _self.seatLocation : seatLocation // ignore: cast_nullable_to_non_nullable
as String,seatNo: null == seatNo ? _self.seatNo : seatNo // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc
mixin _$FullAttendanceData {

 List<FullAttendanceRecord> get records; String get semesterId; BigInt get updateTime; String get courseId; String get courseType;
/// Create a copy of FullAttendanceData
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$FullAttendanceDataCopyWith<FullAttendanceData> get copyWith => _$FullAttendanceDataCopyWithImpl<FullAttendanceData>(this as FullAttendanceData, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is FullAttendanceData&&const DeepCollectionEquality().equals(other.records, records)&&(identical(other.semesterId, semesterId) || other.semesterId == semesterId)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime)&&(identical(other.courseId, courseId) || other.courseId == courseId)&&(identical(other.courseType, courseType) || other.courseType == courseType));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(records),semesterId,updateTime,courseId,courseType);

@override
String toString() {
  return 'FullAttendanceData(records: $records, semesterId: $semesterId, updateTime: $updateTime, courseId: $courseId, courseType: $courseType)';
}


}

/// @nodoc
abstract mixin class $FullAttendanceDataCopyWith<$Res>  {
  factory $FullAttendanceDataCopyWith(FullAttendanceData value, $Res Function(FullAttendanceData) _then) = _$FullAttendanceDataCopyWithImpl;
@useResult
$Res call({
 List<FullAttendanceRecord> records, String semesterId, BigInt updateTime, String courseId, String courseType
});




}
/// @nodoc
class _$FullAttendanceDataCopyWithImpl<$Res>
    implements $FullAttendanceDataCopyWith<$Res> {
  _$FullAttendanceDataCopyWithImpl(this._self, this._then);

  final FullAttendanceData _self;
  final $Res Function(FullAttendanceData) _then;

/// Create a copy of FullAttendanceData
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? records = null,Object? semesterId = null,Object? updateTime = null,Object? courseId = null,Object? courseType = null,}) {
  return _then(_self.copyWith(
records: null == records ? _self.records : records // ignore: cast_nullable_to_non_nullable
as List<FullAttendanceRecord>,semesterId: null == semesterId ? _self.semesterId : semesterId // ignore: cast_nullable_to_non_nullable
as String,updateTime: null == updateTime ? _self.updateTime : updateTime // ignore: cast_nullable_to_non_nullable
as BigInt,courseId: null == courseId ? _self.courseId : courseId // ignore: cast_nullable_to_non_nullable
as String,courseType: null == courseType ? _self.courseType : courseType // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// @nodoc


class _FullAttendanceData implements FullAttendanceData {
  const _FullAttendanceData({required final  List<FullAttendanceRecord> records, required this.semesterId, required this.updateTime, required this.courseId, required this.courseType}): _records = records;
  

 final  List<FullAttendanceRecord> _records;
@override List<FullAttendanceRecord> get records {
  if (_records is EqualUnmodifiableListView) return _records;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_records);
}

@override final  String semesterId;
@override final  BigInt updateTime;
@override final  String courseId;
@override final  String courseType;

/// Create a copy of FullAttendanceData
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$FullAttendanceDataCopyWith<_FullAttendanceData> get copyWith => __$FullAttendanceDataCopyWithImpl<_FullAttendanceData>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _FullAttendanceData&&const DeepCollectionEquality().equals(other._records, _records)&&(identical(other.semesterId, semesterId) || other.semesterId == semesterId)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime)&&(identical(other.courseId, courseId) || other.courseId == courseId)&&(identical(other.courseType, courseType) || other.courseType == courseType));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(_records),semesterId,updateTime,courseId,courseType);

@override
String toString() {
  return 'FullAttendanceData(records: $records, semesterId: $semesterId, updateTime: $updateTime, courseId: $courseId, courseType: $courseType)';
}


}

/// @nodoc
abstract mixin class _$FullAttendanceDataCopyWith<$Res> implements $FullAttendanceDataCopyWith<$Res> {
  factory _$FullAttendanceDataCopyWith(_FullAttendanceData value, $Res Function(_FullAttendanceData) _then) = __$FullAttendanceDataCopyWithImpl;
@override @useResult
$Res call({
 List<FullAttendanceRecord> records, String semesterId, BigInt updateTime, String courseId, String courseType
});




}
/// @nodoc
class __$FullAttendanceDataCopyWithImpl<$Res>
    implements _$FullAttendanceDataCopyWith<$Res> {
  __$FullAttendanceDataCopyWithImpl(this._self, this._then);

  final _FullAttendanceData _self;
  final $Res Function(_FullAttendanceData) _then;

/// Create a copy of FullAttendanceData
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? records = null,Object? semesterId = null,Object? updateTime = null,Object? courseId = null,Object? courseType = null,}) {
  return _then(_FullAttendanceData(
records: null == records ? _self._records : records // ignore: cast_nullable_to_non_nullable
as List<FullAttendanceRecord>,semesterId: null == semesterId ? _self.semesterId : semesterId // ignore: cast_nullable_to_non_nullable
as String,updateTime: null == updateTime ? _self.updateTime : updateTime // ignore: cast_nullable_to_non_nullable
as BigInt,courseId: null == courseId ? _self.courseId : courseId // ignore: cast_nullable_to_non_nullable
as String,courseType: null == courseType ? _self.courseType : courseType // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc
mixin _$FullAttendanceRecord {

 String get serial; String get date; String get slot; String get dayTime; String get status; String get remark;
/// Create a copy of FullAttendanceRecord
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$FullAttendanceRecordCopyWith<FullAttendanceRecord> get copyWith => _$FullAttendanceRecordCopyWithImpl<FullAttendanceRecord>(this as FullAttendanceRecord, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is FullAttendanceRecord&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.date, date) || other.date == date)&&(identical(other.slot, slot) || other.slot == slot)&&(identical(other.dayTime, dayTime) || other.dayTime == dayTime)&&(identical(other.status, status) || other.status == status)&&(identical(other.remark, remark) || other.remark == remark));
}


@override
int get hashCode => Object.hash(runtimeType,serial,date,slot,dayTime,status,remark);

@override
String toString() {
  return 'FullAttendanceRecord(serial: $serial, date: $date, slot: $slot, dayTime: $dayTime, status: $status, remark: $remark)';
}


}

/// @nodoc
abstract mixin class $FullAttendanceRecordCopyWith<$Res>  {
  factory $FullAttendanceRecordCopyWith(FullAttendanceRecord value, $Res Function(FullAttendanceRecord) _then) = _$FullAttendanceRecordCopyWithImpl;
@useResult
$Res call({
 String serial, String date, String slot, String dayTime, String status, String remark
});




}
/// @nodoc
class _$FullAttendanceRecordCopyWithImpl<$Res>
    implements $FullAttendanceRecordCopyWith<$Res> {
  _$FullAttendanceRecordCopyWithImpl(this._self, this._then);

  final FullAttendanceRecord _self;
  final $Res Function(FullAttendanceRecord) _then;

/// Create a copy of FullAttendanceRecord
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? serial = null,Object? date = null,Object? slot = null,Object? dayTime = null,Object? status = null,Object? remark = null,}) {
  return _then(_self.copyWith(
serial: null == serial ? _self.serial : serial // ignore: cast_nullable_to_non_nullable
as String,date: null == date ? _self.date : date // ignore: cast_nullable_to_non_nullable
as String,slot: null == slot ? _self.slot : slot // ignore: cast_nullable_to_non_nullable
as String,dayTime: null == dayTime ? _self.dayTime : dayTime // ignore: cast_nullable_to_non_nullable
as String,status: null == status ? _self.status : status // ignore: cast_nullable_to_non_nullable
as String,remark: null == remark ? _self.remark : remark // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// @nodoc


class _FullAttendanceRecord implements FullAttendanceRecord {
  const _FullAttendanceRecord({required this.serial, required this.date, required this.slot, required this.dayTime, required this.status, required this.remark});
  

@override final  String serial;
@override final  String date;
@override final  String slot;
@override final  String dayTime;
@override final  String status;
@override final  String remark;

/// Create a copy of FullAttendanceRecord
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$FullAttendanceRecordCopyWith<_FullAttendanceRecord> get copyWith => __$FullAttendanceRecordCopyWithImpl<_FullAttendanceRecord>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _FullAttendanceRecord&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.date, date) || other.date == date)&&(identical(other.slot, slot) || other.slot == slot)&&(identical(other.dayTime, dayTime) || other.dayTime == dayTime)&&(identical(other.status, status) || other.status == status)&&(identical(other.remark, remark) || other.remark == remark));
}


@override
int get hashCode => Object.hash(runtimeType,serial,date,slot,dayTime,status,remark);

@override
String toString() {
  return 'FullAttendanceRecord(serial: $serial, date: $date, slot: $slot, dayTime: $dayTime, status: $status, remark: $remark)';
}


}

/// @nodoc
abstract mixin class _$FullAttendanceRecordCopyWith<$Res> implements $FullAttendanceRecordCopyWith<$Res> {
  factory _$FullAttendanceRecordCopyWith(_FullAttendanceRecord value, $Res Function(_FullAttendanceRecord) _then) = __$FullAttendanceRecordCopyWithImpl;
@override @useResult
$Res call({
 String serial, String date, String slot, String dayTime, String status, String remark
});




}
/// @nodoc
class __$FullAttendanceRecordCopyWithImpl<$Res>
    implements _$FullAttendanceRecordCopyWith<$Res> {
  __$FullAttendanceRecordCopyWithImpl(this._self, this._then);

  final _FullAttendanceRecord _self;
  final $Res Function(_FullAttendanceRecord) _then;

/// Create a copy of FullAttendanceRecord
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? serial = null,Object? date = null,Object? slot = null,Object? dayTime = null,Object? status = null,Object? remark = null,}) {
  return _then(_FullAttendanceRecord(
serial: null == serial ? _self.serial : serial // ignore: cast_nullable_to_non_nullable
as String,date: null == date ? _self.date : date // ignore: cast_nullable_to_non_nullable
as String,slot: null == slot ? _self.slot : slot // ignore: cast_nullable_to_non_nullable
as String,dayTime: null == dayTime ? _self.dayTime : dayTime // ignore: cast_nullable_to_non_nullable
as String,status: null == status ? _self.status : status // ignore: cast_nullable_to_non_nullable
as String,remark: null == remark ? _self.remark : remark // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc
mixin _$MarksData {

 List<MarksRecord> get records; String get semesterId; BigInt get updateTime;
/// Create a copy of MarksData
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$MarksDataCopyWith<MarksData> get copyWith => _$MarksDataCopyWithImpl<MarksData>(this as MarksData, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is MarksData&&const DeepCollectionEquality().equals(other.records, records)&&(identical(other.semesterId, semesterId) || other.semesterId == semesterId)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(records),semesterId,updateTime);

@override
String toString() {
  return 'MarksData(records: $records, semesterId: $semesterId, updateTime: $updateTime)';
}


}

/// @nodoc
abstract mixin class $MarksDataCopyWith<$Res>  {
  factory $MarksDataCopyWith(MarksData value, $Res Function(MarksData) _then) = _$MarksDataCopyWithImpl;
@useResult
$Res call({
 List<MarksRecord> records, String semesterId, BigInt updateTime
});




}
/// @nodoc
class _$MarksDataCopyWithImpl<$Res>
    implements $MarksDataCopyWith<$Res> {
  _$MarksDataCopyWithImpl(this._self, this._then);

  final MarksData _self;
  final $Res Function(MarksData) _then;

/// Create a copy of MarksData
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? records = null,Object? semesterId = null,Object? updateTime = null,}) {
  return _then(_self.copyWith(
records: null == records ? _self.records : records // ignore: cast_nullable_to_non_nullable
as List<MarksRecord>,semesterId: null == semesterId ? _self.semesterId : semesterId // ignore: cast_nullable_to_non_nullable
as String,updateTime: null == updateTime ? _self.updateTime : updateTime // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}

}


/// @nodoc


class _MarksData implements MarksData {
  const _MarksData({required final  List<MarksRecord> records, required this.semesterId, required this.updateTime}): _records = records;
  

 final  List<MarksRecord> _records;
@override List<MarksRecord> get records {
  if (_records is EqualUnmodifiableListView) return _records;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_records);
}

@override final  String semesterId;
@override final  BigInt updateTime;

/// Create a copy of MarksData
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$MarksDataCopyWith<_MarksData> get copyWith => __$MarksDataCopyWithImpl<_MarksData>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _MarksData&&const DeepCollectionEquality().equals(other._records, _records)&&(identical(other.semesterId, semesterId) || other.semesterId == semesterId)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(_records),semesterId,updateTime);

@override
String toString() {
  return 'MarksData(records: $records, semesterId: $semesterId, updateTime: $updateTime)';
}


}

/// @nodoc
abstract mixin class _$MarksDataCopyWith<$Res> implements $MarksDataCopyWith<$Res> {
  factory _$MarksDataCopyWith(_MarksData value, $Res Function(_MarksData) _then) = __$MarksDataCopyWithImpl;
@override @useResult
$Res call({
 List<MarksRecord> records, String semesterId, BigInt updateTime
});




}
/// @nodoc
class __$MarksDataCopyWithImpl<$Res>
    implements _$MarksDataCopyWith<$Res> {
  __$MarksDataCopyWithImpl(this._self, this._then);

  final _MarksData _self;
  final $Res Function(_MarksData) _then;

/// Create a copy of MarksData
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? records = null,Object? semesterId = null,Object? updateTime = null,}) {
  return _then(_MarksData(
records: null == records ? _self._records : records // ignore: cast_nullable_to_non_nullable
as List<MarksRecord>,semesterId: null == semesterId ? _self.semesterId : semesterId // ignore: cast_nullable_to_non_nullable
as String,updateTime: null == updateTime ? _self.updateTime : updateTime // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc
mixin _$MarksRecord {

 String get serial; String get coursecode; String get coursetitle; String get coursetype; String get faculity; String get slot; List<MarksRecordEach> get marks;
/// Create a copy of MarksRecord
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$MarksRecordCopyWith<MarksRecord> get copyWith => _$MarksRecordCopyWithImpl<MarksRecord>(this as MarksRecord, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is MarksRecord&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.coursecode, coursecode) || other.coursecode == coursecode)&&(identical(other.coursetitle, coursetitle) || other.coursetitle == coursetitle)&&(identical(other.coursetype, coursetype) || other.coursetype == coursetype)&&(identical(other.faculity, faculity) || other.faculity == faculity)&&(identical(other.slot, slot) || other.slot == slot)&&const DeepCollectionEquality().equals(other.marks, marks));
}


@override
int get hashCode => Object.hash(runtimeType,serial,coursecode,coursetitle,coursetype,faculity,slot,const DeepCollectionEquality().hash(marks));

@override
String toString() {
  return 'MarksRecord(serial: $serial, coursecode: $coursecode, coursetitle: $coursetitle, coursetype: $coursetype, faculity: $faculity, slot: $slot, marks: $marks)';
}


}

/// @nodoc
abstract mixin class $MarksRecordCopyWith<$Res>  {
  factory $MarksRecordCopyWith(MarksRecord value, $Res Function(MarksRecord) _then) = _$MarksRecordCopyWithImpl;
@useResult
$Res call({
 String serial, String coursecode, String coursetitle, String coursetype, String faculity, String slot, List<MarksRecordEach> marks
});




}
/// @nodoc
class _$MarksRecordCopyWithImpl<$Res>
    implements $MarksRecordCopyWith<$Res> {
  _$MarksRecordCopyWithImpl(this._self, this._then);

  final MarksRecord _self;
  final $Res Function(MarksRecord) _then;

/// Create a copy of MarksRecord
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? serial = null,Object? coursecode = null,Object? coursetitle = null,Object? coursetype = null,Object? faculity = null,Object? slot = null,Object? marks = null,}) {
  return _then(_self.copyWith(
serial: null == serial ? _self.serial : serial // ignore: cast_nullable_to_non_nullable
as String,coursecode: null == coursecode ? _self.coursecode : coursecode // ignore: cast_nullable_to_non_nullable
as String,coursetitle: null == coursetitle ? _self.coursetitle : coursetitle // ignore: cast_nullable_to_non_nullable
as String,coursetype: null == coursetype ? _self.coursetype : coursetype // ignore: cast_nullable_to_non_nullable
as String,faculity: null == faculity ? _self.faculity : faculity // ignore: cast_nullable_to_non_nullable
as String,slot: null == slot ? _self.slot : slot // ignore: cast_nullable_to_non_nullable
as String,marks: null == marks ? _self.marks : marks // ignore: cast_nullable_to_non_nullable
as List<MarksRecordEach>,
  ));
}

}


/// @nodoc


class _MarksRecord implements MarksRecord {
  const _MarksRecord({required this.serial, required this.coursecode, required this.coursetitle, required this.coursetype, required this.faculity, required this.slot, required final  List<MarksRecordEach> marks}): _marks = marks;
  

@override final  String serial;
@override final  String coursecode;
@override final  String coursetitle;
@override final  String coursetype;
@override final  String faculity;
@override final  String slot;
 final  List<MarksRecordEach> _marks;
@override List<MarksRecordEach> get marks {
  if (_marks is EqualUnmodifiableListView) return _marks;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_marks);
}


/// Create a copy of MarksRecord
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$MarksRecordCopyWith<_MarksRecord> get copyWith => __$MarksRecordCopyWithImpl<_MarksRecord>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _MarksRecord&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.coursecode, coursecode) || other.coursecode == coursecode)&&(identical(other.coursetitle, coursetitle) || other.coursetitle == coursetitle)&&(identical(other.coursetype, coursetype) || other.coursetype == coursetype)&&(identical(other.faculity, faculity) || other.faculity == faculity)&&(identical(other.slot, slot) || other.slot == slot)&&const DeepCollectionEquality().equals(other._marks, _marks));
}


@override
int get hashCode => Object.hash(runtimeType,serial,coursecode,coursetitle,coursetype,faculity,slot,const DeepCollectionEquality().hash(_marks));

@override
String toString() {
  return 'MarksRecord(serial: $serial, coursecode: $coursecode, coursetitle: $coursetitle, coursetype: $coursetype, faculity: $faculity, slot: $slot, marks: $marks)';
}


}

/// @nodoc
abstract mixin class _$MarksRecordCopyWith<$Res> implements $MarksRecordCopyWith<$Res> {
  factory _$MarksRecordCopyWith(_MarksRecord value, $Res Function(_MarksRecord) _then) = __$MarksRecordCopyWithImpl;
@override @useResult
$Res call({
 String serial, String coursecode, String coursetitle, String coursetype, String faculity, String slot, List<MarksRecordEach> marks
});




}
/// @nodoc
class __$MarksRecordCopyWithImpl<$Res>
    implements _$MarksRecordCopyWith<$Res> {
  __$MarksRecordCopyWithImpl(this._self, this._then);

  final _MarksRecord _self;
  final $Res Function(_MarksRecord) _then;

/// Create a copy of MarksRecord
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? serial = null,Object? coursecode = null,Object? coursetitle = null,Object? coursetype = null,Object? faculity = null,Object? slot = null,Object? marks = null,}) {
  return _then(_MarksRecord(
serial: null == serial ? _self.serial : serial // ignore: cast_nullable_to_non_nullable
as String,coursecode: null == coursecode ? _self.coursecode : coursecode // ignore: cast_nullable_to_non_nullable
as String,coursetitle: null == coursetitle ? _self.coursetitle : coursetitle // ignore: cast_nullable_to_non_nullable
as String,coursetype: null == coursetype ? _self.coursetype : coursetype // ignore: cast_nullable_to_non_nullable
as String,faculity: null == faculity ? _self.faculity : faculity // ignore: cast_nullable_to_non_nullable
as String,slot: null == slot ? _self.slot : slot // ignore: cast_nullable_to_non_nullable
as String,marks: null == marks ? _self._marks : marks // ignore: cast_nullable_to_non_nullable
as List<MarksRecordEach>,
  ));
}


}


/// @nodoc
mixin _$MarksRecordEach {

 String get serial; String get markstitle; String get maxmarks; String get weightage; String get status; String get scoredmark; String get weightagemark; String get remark;
/// Create a copy of MarksRecordEach
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$MarksRecordEachCopyWith<MarksRecordEach> get copyWith => _$MarksRecordEachCopyWithImpl<MarksRecordEach>(this as MarksRecordEach, _$identity);

  /// Serializes this MarksRecordEach to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is MarksRecordEach&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.markstitle, markstitle) || other.markstitle == markstitle)&&(identical(other.maxmarks, maxmarks) || other.maxmarks == maxmarks)&&(identical(other.weightage, weightage) || other.weightage == weightage)&&(identical(other.status, status) || other.status == status)&&(identical(other.scoredmark, scoredmark) || other.scoredmark == scoredmark)&&(identical(other.weightagemark, weightagemark) || other.weightagemark == weightagemark)&&(identical(other.remark, remark) || other.remark == remark));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,serial,markstitle,maxmarks,weightage,status,scoredmark,weightagemark,remark);

@override
String toString() {
  return 'MarksRecordEach(serial: $serial, markstitle: $markstitle, maxmarks: $maxmarks, weightage: $weightage, status: $status, scoredmark: $scoredmark, weightagemark: $weightagemark, remark: $remark)';
}


}

/// @nodoc
abstract mixin class $MarksRecordEachCopyWith<$Res>  {
  factory $MarksRecordEachCopyWith(MarksRecordEach value, $Res Function(MarksRecordEach) _then) = _$MarksRecordEachCopyWithImpl;
@useResult
$Res call({
 String serial, String markstitle, String maxmarks, String weightage, String status, String scoredmark, String weightagemark, String remark
});




}
/// @nodoc
class _$MarksRecordEachCopyWithImpl<$Res>
    implements $MarksRecordEachCopyWith<$Res> {
  _$MarksRecordEachCopyWithImpl(this._self, this._then);

  final MarksRecordEach _self;
  final $Res Function(MarksRecordEach) _then;

/// Create a copy of MarksRecordEach
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? serial = null,Object? markstitle = null,Object? maxmarks = null,Object? weightage = null,Object? status = null,Object? scoredmark = null,Object? weightagemark = null,Object? remark = null,}) {
  return _then(_self.copyWith(
serial: null == serial ? _self.serial : serial // ignore: cast_nullable_to_non_nullable
as String,markstitle: null == markstitle ? _self.markstitle : markstitle // ignore: cast_nullable_to_non_nullable
as String,maxmarks: null == maxmarks ? _self.maxmarks : maxmarks // ignore: cast_nullable_to_non_nullable
as String,weightage: null == weightage ? _self.weightage : weightage // ignore: cast_nullable_to_non_nullable
as String,status: null == status ? _self.status : status // ignore: cast_nullable_to_non_nullable
as String,scoredmark: null == scoredmark ? _self.scoredmark : scoredmark // ignore: cast_nullable_to_non_nullable
as String,weightagemark: null == weightagemark ? _self.weightagemark : weightagemark // ignore: cast_nullable_to_non_nullable
as String,remark: null == remark ? _self.remark : remark // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// @nodoc
@JsonSerializable()

class _MarksRecordEach implements MarksRecordEach {
  const _MarksRecordEach({required this.serial, required this.markstitle, required this.maxmarks, required this.weightage, required this.status, required this.scoredmark, required this.weightagemark, required this.remark});
  factory _MarksRecordEach.fromJson(Map<String, dynamic> json) => _$MarksRecordEachFromJson(json);

@override final  String serial;
@override final  String markstitle;
@override final  String maxmarks;
@override final  String weightage;
@override final  String status;
@override final  String scoredmark;
@override final  String weightagemark;
@override final  String remark;

/// Create a copy of MarksRecordEach
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$MarksRecordEachCopyWith<_MarksRecordEach> get copyWith => __$MarksRecordEachCopyWithImpl<_MarksRecordEach>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$MarksRecordEachToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _MarksRecordEach&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.markstitle, markstitle) || other.markstitle == markstitle)&&(identical(other.maxmarks, maxmarks) || other.maxmarks == maxmarks)&&(identical(other.weightage, weightage) || other.weightage == weightage)&&(identical(other.status, status) || other.status == status)&&(identical(other.scoredmark, scoredmark) || other.scoredmark == scoredmark)&&(identical(other.weightagemark, weightagemark) || other.weightagemark == weightagemark)&&(identical(other.remark, remark) || other.remark == remark));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,serial,markstitle,maxmarks,weightage,status,scoredmark,weightagemark,remark);

@override
String toString() {
  return 'MarksRecordEach(serial: $serial, markstitle: $markstitle, maxmarks: $maxmarks, weightage: $weightage, status: $status, scoredmark: $scoredmark, weightagemark: $weightagemark, remark: $remark)';
}


}

/// @nodoc
abstract mixin class _$MarksRecordEachCopyWith<$Res> implements $MarksRecordEachCopyWith<$Res> {
  factory _$MarksRecordEachCopyWith(_MarksRecordEach value, $Res Function(_MarksRecordEach) _then) = __$MarksRecordEachCopyWithImpl;
@override @useResult
$Res call({
 String serial, String markstitle, String maxmarks, String weightage, String status, String scoredmark, String weightagemark, String remark
});




}
/// @nodoc
class __$MarksRecordEachCopyWithImpl<$Res>
    implements _$MarksRecordEachCopyWith<$Res> {
  __$MarksRecordEachCopyWithImpl(this._self, this._then);

  final _MarksRecordEach _self;
  final $Res Function(_MarksRecordEach) _then;

/// Create a copy of MarksRecordEach
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? serial = null,Object? markstitle = null,Object? maxmarks = null,Object? weightage = null,Object? status = null,Object? scoredmark = null,Object? weightagemark = null,Object? remark = null,}) {
  return _then(_MarksRecordEach(
serial: null == serial ? _self.serial : serial // ignore: cast_nullable_to_non_nullable
as String,markstitle: null == markstitle ? _self.markstitle : markstitle // ignore: cast_nullable_to_non_nullable
as String,maxmarks: null == maxmarks ? _self.maxmarks : maxmarks // ignore: cast_nullable_to_non_nullable
as String,weightage: null == weightage ? _self.weightage : weightage // ignore: cast_nullable_to_non_nullable
as String,status: null == status ? _self.status : status // ignore: cast_nullable_to_non_nullable
as String,scoredmark: null == scoredmark ? _self.scoredmark : scoredmark // ignore: cast_nullable_to_non_nullable
as String,weightagemark: null == weightagemark ? _self.weightagemark : weightagemark // ignore: cast_nullable_to_non_nullable
as String,remark: null == remark ? _self.remark : remark // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc
mixin _$PerExamScheduleRecord {

 List<ExamScheduleRecord> get records; String get examType;
/// Create a copy of PerExamScheduleRecord
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PerExamScheduleRecordCopyWith<PerExamScheduleRecord> get copyWith => _$PerExamScheduleRecordCopyWithImpl<PerExamScheduleRecord>(this as PerExamScheduleRecord, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PerExamScheduleRecord&&const DeepCollectionEquality().equals(other.records, records)&&(identical(other.examType, examType) || other.examType == examType));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(records),examType);

@override
String toString() {
  return 'PerExamScheduleRecord(records: $records, examType: $examType)';
}


}

/// @nodoc
abstract mixin class $PerExamScheduleRecordCopyWith<$Res>  {
  factory $PerExamScheduleRecordCopyWith(PerExamScheduleRecord value, $Res Function(PerExamScheduleRecord) _then) = _$PerExamScheduleRecordCopyWithImpl;
@useResult
$Res call({
 List<ExamScheduleRecord> records, String examType
});




}
/// @nodoc
class _$PerExamScheduleRecordCopyWithImpl<$Res>
    implements $PerExamScheduleRecordCopyWith<$Res> {
  _$PerExamScheduleRecordCopyWithImpl(this._self, this._then);

  final PerExamScheduleRecord _self;
  final $Res Function(PerExamScheduleRecord) _then;

/// Create a copy of PerExamScheduleRecord
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? records = null,Object? examType = null,}) {
  return _then(_self.copyWith(
records: null == records ? _self.records : records // ignore: cast_nullable_to_non_nullable
as List<ExamScheduleRecord>,examType: null == examType ? _self.examType : examType // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// @nodoc


class _PerExamScheduleRecord implements PerExamScheduleRecord {
  const _PerExamScheduleRecord({required final  List<ExamScheduleRecord> records, required this.examType}): _records = records;
  

 final  List<ExamScheduleRecord> _records;
@override List<ExamScheduleRecord> get records {
  if (_records is EqualUnmodifiableListView) return _records;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_records);
}

@override final  String examType;

/// Create a copy of PerExamScheduleRecord
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$PerExamScheduleRecordCopyWith<_PerExamScheduleRecord> get copyWith => __$PerExamScheduleRecordCopyWithImpl<_PerExamScheduleRecord>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _PerExamScheduleRecord&&const DeepCollectionEquality().equals(other._records, _records)&&(identical(other.examType, examType) || other.examType == examType));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(_records),examType);

@override
String toString() {
  return 'PerExamScheduleRecord(records: $records, examType: $examType)';
}


}

/// @nodoc
abstract mixin class _$PerExamScheduleRecordCopyWith<$Res> implements $PerExamScheduleRecordCopyWith<$Res> {
  factory _$PerExamScheduleRecordCopyWith(_PerExamScheduleRecord value, $Res Function(_PerExamScheduleRecord) _then) = __$PerExamScheduleRecordCopyWithImpl;
@override @useResult
$Res call({
 List<ExamScheduleRecord> records, String examType
});




}
/// @nodoc
class __$PerExamScheduleRecordCopyWithImpl<$Res>
    implements _$PerExamScheduleRecordCopyWith<$Res> {
  __$PerExamScheduleRecordCopyWithImpl(this._self, this._then);

  final _PerExamScheduleRecord _self;
  final $Res Function(_PerExamScheduleRecord) _then;

/// Create a copy of PerExamScheduleRecord
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? records = null,Object? examType = null,}) {
  return _then(_PerExamScheduleRecord(
records: null == records ? _self._records : records // ignore: cast_nullable_to_non_nullable
as List<ExamScheduleRecord>,examType: null == examType ? _self.examType : examType // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc
mixin _$SemesterData {

 List<SemesterInfo> get semesters; BigInt get updateTime;
/// Create a copy of SemesterData
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SemesterDataCopyWith<SemesterData> get copyWith => _$SemesterDataCopyWithImpl<SemesterData>(this as SemesterData, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SemesterData&&const DeepCollectionEquality().equals(other.semesters, semesters)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(semesters),updateTime);

@override
String toString() {
  return 'SemesterData(semesters: $semesters, updateTime: $updateTime)';
}


}

/// @nodoc
abstract mixin class $SemesterDataCopyWith<$Res>  {
  factory $SemesterDataCopyWith(SemesterData value, $Res Function(SemesterData) _then) = _$SemesterDataCopyWithImpl;
@useResult
$Res call({
 List<SemesterInfo> semesters, BigInt updateTime
});




}
/// @nodoc
class _$SemesterDataCopyWithImpl<$Res>
    implements $SemesterDataCopyWith<$Res> {
  _$SemesterDataCopyWithImpl(this._self, this._then);

  final SemesterData _self;
  final $Res Function(SemesterData) _then;

/// Create a copy of SemesterData
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? semesters = null,Object? updateTime = null,}) {
  return _then(_self.copyWith(
semesters: null == semesters ? _self.semesters : semesters // ignore: cast_nullable_to_non_nullable
as List<SemesterInfo>,updateTime: null == updateTime ? _self.updateTime : updateTime // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}

}


/// @nodoc


class _SemesterData implements SemesterData {
  const _SemesterData({required final  List<SemesterInfo> semesters, required this.updateTime}): _semesters = semesters;
  

 final  List<SemesterInfo> _semesters;
@override List<SemesterInfo> get semesters {
  if (_semesters is EqualUnmodifiableListView) return _semesters;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_semesters);
}

@override final  BigInt updateTime;

/// Create a copy of SemesterData
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$SemesterDataCopyWith<_SemesterData> get copyWith => __$SemesterDataCopyWithImpl<_SemesterData>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _SemesterData&&const DeepCollectionEquality().equals(other._semesters, _semesters)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(_semesters),updateTime);

@override
String toString() {
  return 'SemesterData(semesters: $semesters, updateTime: $updateTime)';
}


}

/// @nodoc
abstract mixin class _$SemesterDataCopyWith<$Res> implements $SemesterDataCopyWith<$Res> {
  factory _$SemesterDataCopyWith(_SemesterData value, $Res Function(_SemesterData) _then) = __$SemesterDataCopyWithImpl;
@override @useResult
$Res call({
 List<SemesterInfo> semesters, BigInt updateTime
});




}
/// @nodoc
class __$SemesterDataCopyWithImpl<$Res>
    implements _$SemesterDataCopyWith<$Res> {
  __$SemesterDataCopyWithImpl(this._self, this._then);

  final _SemesterData _self;
  final $Res Function(_SemesterData) _then;

/// Create a copy of SemesterData
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? semesters = null,Object? updateTime = null,}) {
  return _then(_SemesterData(
semesters: null == semesters ? _self._semesters : semesters // ignore: cast_nullable_to_non_nullable
as List<SemesterInfo>,updateTime: null == updateTime ? _self.updateTime : updateTime // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc
mixin _$SemesterInfo {

 String get id; String get name;
/// Create a copy of SemesterInfo
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SemesterInfoCopyWith<SemesterInfo> get copyWith => _$SemesterInfoCopyWithImpl<SemesterInfo>(this as SemesterInfo, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SemesterInfo&&(identical(other.id, id) || other.id == id)&&(identical(other.name, name) || other.name == name));
}


@override
int get hashCode => Object.hash(runtimeType,id,name);

@override
String toString() {
  return 'SemesterInfo(id: $id, name: $name)';
}


}

/// @nodoc
abstract mixin class $SemesterInfoCopyWith<$Res>  {
  factory $SemesterInfoCopyWith(SemesterInfo value, $Res Function(SemesterInfo) _then) = _$SemesterInfoCopyWithImpl;
@useResult
$Res call({
 String id, String name
});




}
/// @nodoc
class _$SemesterInfoCopyWithImpl<$Res>
    implements $SemesterInfoCopyWith<$Res> {
  _$SemesterInfoCopyWithImpl(this._self, this._then);

  final SemesterInfo _self;
  final $Res Function(SemesterInfo) _then;

/// Create a copy of SemesterInfo
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? id = null,Object? name = null,}) {
  return _then(_self.copyWith(
id: null == id ? _self.id : id // ignore: cast_nullable_to_non_nullable
as String,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// @nodoc


class _SemesterInfo implements SemesterInfo {
  const _SemesterInfo({required this.id, required this.name});
  

@override final  String id;
@override final  String name;

/// Create a copy of SemesterInfo
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$SemesterInfoCopyWith<_SemesterInfo> get copyWith => __$SemesterInfoCopyWithImpl<_SemesterInfo>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _SemesterInfo&&(identical(other.id, id) || other.id == id)&&(identical(other.name, name) || other.name == name));
}


@override
int get hashCode => Object.hash(runtimeType,id,name);

@override
String toString() {
  return 'SemesterInfo(id: $id, name: $name)';
}


}

/// @nodoc
abstract mixin class _$SemesterInfoCopyWith<$Res> implements $SemesterInfoCopyWith<$Res> {
  factory _$SemesterInfoCopyWith(_SemesterInfo value, $Res Function(_SemesterInfo) _then) = __$SemesterInfoCopyWithImpl;
@override @useResult
$Res call({
 String id, String name
});




}
/// @nodoc
class __$SemesterInfoCopyWithImpl<$Res>
    implements _$SemesterInfoCopyWith<$Res> {
  __$SemesterInfoCopyWithImpl(this._self, this._then);

  final _SemesterInfo _self;
  final $Res Function(_SemesterInfo) _then;

/// Create a copy of SemesterInfo
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? id = null,Object? name = null,}) {
  return _then(_SemesterInfo(
id: null == id ? _self.id : id // ignore: cast_nullable_to_non_nullable
as String,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc
mixin _$TimetableData {

 List<TimetableSlot> get slots; String get semesterId; BigInt get updateTime;
/// Create a copy of TimetableData
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$TimetableDataCopyWith<TimetableData> get copyWith => _$TimetableDataCopyWithImpl<TimetableData>(this as TimetableData, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TimetableData&&const DeepCollectionEquality().equals(other.slots, slots)&&(identical(other.semesterId, semesterId) || other.semesterId == semesterId)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(slots),semesterId,updateTime);

@override
String toString() {
  return 'TimetableData(slots: $slots, semesterId: $semesterId, updateTime: $updateTime)';
}


}

/// @nodoc
abstract mixin class $TimetableDataCopyWith<$Res>  {
  factory $TimetableDataCopyWith(TimetableData value, $Res Function(TimetableData) _then) = _$TimetableDataCopyWithImpl;
@useResult
$Res call({
 List<TimetableSlot> slots, String semesterId, BigInt updateTime
});




}
/// @nodoc
class _$TimetableDataCopyWithImpl<$Res>
    implements $TimetableDataCopyWith<$Res> {
  _$TimetableDataCopyWithImpl(this._self, this._then);

  final TimetableData _self;
  final $Res Function(TimetableData) _then;

/// Create a copy of TimetableData
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? slots = null,Object? semesterId = null,Object? updateTime = null,}) {
  return _then(_self.copyWith(
slots: null == slots ? _self.slots : slots // ignore: cast_nullable_to_non_nullable
as List<TimetableSlot>,semesterId: null == semesterId ? _self.semesterId : semesterId // ignore: cast_nullable_to_non_nullable
as String,updateTime: null == updateTime ? _self.updateTime : updateTime // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}

}


/// @nodoc


class _TimetableData implements TimetableData {
  const _TimetableData({required final  List<TimetableSlot> slots, required this.semesterId, required this.updateTime}): _slots = slots;
  

 final  List<TimetableSlot> _slots;
@override List<TimetableSlot> get slots {
  if (_slots is EqualUnmodifiableListView) return _slots;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_slots);
}

@override final  String semesterId;
@override final  BigInt updateTime;

/// Create a copy of TimetableData
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$TimetableDataCopyWith<_TimetableData> get copyWith => __$TimetableDataCopyWithImpl<_TimetableData>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _TimetableData&&const DeepCollectionEquality().equals(other._slots, _slots)&&(identical(other.semesterId, semesterId) || other.semesterId == semesterId)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(_slots),semesterId,updateTime);

@override
String toString() {
  return 'TimetableData(slots: $slots, semesterId: $semesterId, updateTime: $updateTime)';
}


}

/// @nodoc
abstract mixin class _$TimetableDataCopyWith<$Res> implements $TimetableDataCopyWith<$Res> {
  factory _$TimetableDataCopyWith(_TimetableData value, $Res Function(_TimetableData) _then) = __$TimetableDataCopyWithImpl;
@override @useResult
$Res call({
 List<TimetableSlot> slots, String semesterId, BigInt updateTime
});




}
/// @nodoc
class __$TimetableDataCopyWithImpl<$Res>
    implements _$TimetableDataCopyWith<$Res> {
  __$TimetableDataCopyWithImpl(this._self, this._then);

  final _TimetableData _self;
  final $Res Function(_TimetableData) _then;

/// Create a copy of TimetableData
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? slots = null,Object? semesterId = null,Object? updateTime = null,}) {
  return _then(_TimetableData(
slots: null == slots ? _self._slots : slots // ignore: cast_nullable_to_non_nullable
as List<TimetableSlot>,semesterId: null == semesterId ? _self.semesterId : semesterId // ignore: cast_nullable_to_non_nullable
as String,updateTime: null == updateTime ? _self.updateTime : updateTime // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc
mixin _$TimetableSlot {

 String get serial; String get day; String get slot; String get courseCode; String get courseType; String get roomNo; String get block; String get startTime; String get endTime; String get name;
/// Create a copy of TimetableSlot
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$TimetableSlotCopyWith<TimetableSlot> get copyWith => _$TimetableSlotCopyWithImpl<TimetableSlot>(this as TimetableSlot, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TimetableSlot&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.day, day) || other.day == day)&&(identical(other.slot, slot) || other.slot == slot)&&(identical(other.courseCode, courseCode) || other.courseCode == courseCode)&&(identical(other.courseType, courseType) || other.courseType == courseType)&&(identical(other.roomNo, roomNo) || other.roomNo == roomNo)&&(identical(other.block, block) || other.block == block)&&(identical(other.startTime, startTime) || other.startTime == startTime)&&(identical(other.endTime, endTime) || other.endTime == endTime)&&(identical(other.name, name) || other.name == name));
}


@override
int get hashCode => Object.hash(runtimeType,serial,day,slot,courseCode,courseType,roomNo,block,startTime,endTime,name);

@override
String toString() {
  return 'TimetableSlot(serial: $serial, day: $day, slot: $slot, courseCode: $courseCode, courseType: $courseType, roomNo: $roomNo, block: $block, startTime: $startTime, endTime: $endTime, name: $name)';
}


}

/// @nodoc
abstract mixin class $TimetableSlotCopyWith<$Res>  {
  factory $TimetableSlotCopyWith(TimetableSlot value, $Res Function(TimetableSlot) _then) = _$TimetableSlotCopyWithImpl;
@useResult
$Res call({
 String serial, String day, String slot, String courseCode, String courseType, String roomNo, String block, String startTime, String endTime, String name
});




}
/// @nodoc
class _$TimetableSlotCopyWithImpl<$Res>
    implements $TimetableSlotCopyWith<$Res> {
  _$TimetableSlotCopyWithImpl(this._self, this._then);

  final TimetableSlot _self;
  final $Res Function(TimetableSlot) _then;

/// Create a copy of TimetableSlot
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? serial = null,Object? day = null,Object? slot = null,Object? courseCode = null,Object? courseType = null,Object? roomNo = null,Object? block = null,Object? startTime = null,Object? endTime = null,Object? name = null,}) {
  return _then(_self.copyWith(
serial: null == serial ? _self.serial : serial // ignore: cast_nullable_to_non_nullable
as String,day: null == day ? _self.day : day // ignore: cast_nullable_to_non_nullable
as String,slot: null == slot ? _self.slot : slot // ignore: cast_nullable_to_non_nullable
as String,courseCode: null == courseCode ? _self.courseCode : courseCode // ignore: cast_nullable_to_non_nullable
as String,courseType: null == courseType ? _self.courseType : courseType // ignore: cast_nullable_to_non_nullable
as String,roomNo: null == roomNo ? _self.roomNo : roomNo // ignore: cast_nullable_to_non_nullable
as String,block: null == block ? _self.block : block // ignore: cast_nullable_to_non_nullable
as String,startTime: null == startTime ? _self.startTime : startTime // ignore: cast_nullable_to_non_nullable
as String,endTime: null == endTime ? _self.endTime : endTime // ignore: cast_nullable_to_non_nullable
as String,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// @nodoc


class _TimetableSlot implements TimetableSlot {
  const _TimetableSlot({required this.serial, required this.day, required this.slot, required this.courseCode, required this.courseType, required this.roomNo, required this.block, required this.startTime, required this.endTime, required this.name});
  

@override final  String serial;
@override final  String day;
@override final  String slot;
@override final  String courseCode;
@override final  String courseType;
@override final  String roomNo;
@override final  String block;
@override final  String startTime;
@override final  String endTime;
@override final  String name;

/// Create a copy of TimetableSlot
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$TimetableSlotCopyWith<_TimetableSlot> get copyWith => __$TimetableSlotCopyWithImpl<_TimetableSlot>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _TimetableSlot&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.day, day) || other.day == day)&&(identical(other.slot, slot) || other.slot == slot)&&(identical(other.courseCode, courseCode) || other.courseCode == courseCode)&&(identical(other.courseType, courseType) || other.courseType == courseType)&&(identical(other.roomNo, roomNo) || other.roomNo == roomNo)&&(identical(other.block, block) || other.block == block)&&(identical(other.startTime, startTime) || other.startTime == startTime)&&(identical(other.endTime, endTime) || other.endTime == endTime)&&(identical(other.name, name) || other.name == name));
}


@override
int get hashCode => Object.hash(runtimeType,serial,day,slot,courseCode,courseType,roomNo,block,startTime,endTime,name);

@override
String toString() {
  return 'TimetableSlot(serial: $serial, day: $day, slot: $slot, courseCode: $courseCode, courseType: $courseType, roomNo: $roomNo, block: $block, startTime: $startTime, endTime: $endTime, name: $name)';
}


}

/// @nodoc
abstract mixin class _$TimetableSlotCopyWith<$Res> implements $TimetableSlotCopyWith<$Res> {
  factory _$TimetableSlotCopyWith(_TimetableSlot value, $Res Function(_TimetableSlot) _then) = __$TimetableSlotCopyWithImpl;
@override @useResult
$Res call({
 String serial, String day, String slot, String courseCode, String courseType, String roomNo, String block, String startTime, String endTime, String name
});




}
/// @nodoc
class __$TimetableSlotCopyWithImpl<$Res>
    implements _$TimetableSlotCopyWith<$Res> {
  __$TimetableSlotCopyWithImpl(this._self, this._then);

  final _TimetableSlot _self;
  final $Res Function(_TimetableSlot) _then;

/// Create a copy of TimetableSlot
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? serial = null,Object? day = null,Object? slot = null,Object? courseCode = null,Object? courseType = null,Object? roomNo = null,Object? block = null,Object? startTime = null,Object? endTime = null,Object? name = null,}) {
  return _then(_TimetableSlot(
serial: null == serial ? _self.serial : serial // ignore: cast_nullable_to_non_nullable
as String,day: null == day ? _self.day : day // ignore: cast_nullable_to_non_nullable
as String,slot: null == slot ? _self.slot : slot // ignore: cast_nullable_to_non_nullable
as String,courseCode: null == courseCode ? _self.courseCode : courseCode // ignore: cast_nullable_to_non_nullable
as String,courseType: null == courseType ? _self.courseType : courseType // ignore: cast_nullable_to_non_nullable
as String,roomNo: null == roomNo ? _self.roomNo : roomNo // ignore: cast_nullable_to_non_nullable
as String,block: null == block ? _self.block : block // ignore: cast_nullable_to_non_nullable
as String,startTime: null == startTime ? _self.startTime : startTime // ignore: cast_nullable_to_non_nullable
as String,endTime: null == endTime ? _self.endTime : endTime // ignore: cast_nullable_to_non_nullable
as String,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
