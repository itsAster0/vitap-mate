// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
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

  /// Serializes this AttendanceData to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AttendanceData&&const DeepCollectionEquality().equals(other.records, records)&&(identical(other.semesterId, semesterId) || other.semesterId == semesterId)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}

@JsonKey(includeFromJson: false, includeToJson: false)
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


/// Adds pattern-matching-related methods to [AttendanceData].
extension AttendanceDataPatterns on AttendanceData {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _AttendanceData value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _AttendanceData() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _AttendanceData value)  $default,){
final _that = this;
switch (_that) {
case _AttendanceData():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _AttendanceData value)?  $default,){
final _that = this;
switch (_that) {
case _AttendanceData() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( List<AttendanceRecord> records,  String semesterId,  BigInt updateTime)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _AttendanceData() when $default != null:
return $default(_that.records,_that.semesterId,_that.updateTime);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( List<AttendanceRecord> records,  String semesterId,  BigInt updateTime)  $default,) {final _that = this;
switch (_that) {
case _AttendanceData():
return $default(_that.records,_that.semesterId,_that.updateTime);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( List<AttendanceRecord> records,  String semesterId,  BigInt updateTime)?  $default,) {final _that = this;
switch (_that) {
case _AttendanceData() when $default != null:
return $default(_that.records,_that.semesterId,_that.updateTime);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _AttendanceData implements AttendanceData {
  const _AttendanceData({required final  List<AttendanceRecord> records, required this.semesterId, required this.updateTime}): _records = records;
  factory _AttendanceData.fromJson(Map<String, dynamic> json) => _$AttendanceDataFromJson(json);

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
Map<String, dynamic> toJson() {
  return _$AttendanceDataToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _AttendanceData&&const DeepCollectionEquality().equals(other._records, _records)&&(identical(other.semesterId, semesterId) || other.semesterId == semesterId)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}

@JsonKey(includeFromJson: false, includeToJson: false)
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

  /// Serializes this AttendanceRecord to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AttendanceRecord&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.category, category) || other.category == category)&&(identical(other.courseName, courseName) || other.courseName == courseName)&&(identical(other.courseCode, courseCode) || other.courseCode == courseCode)&&(identical(other.courseType, courseType) || other.courseType == courseType)&&(identical(other.facultyDetail, facultyDetail) || other.facultyDetail == facultyDetail)&&(identical(other.classesAttended, classesAttended) || other.classesAttended == classesAttended)&&(identical(other.totalClasses, totalClasses) || other.totalClasses == totalClasses)&&(identical(other.attendancePercentage, attendancePercentage) || other.attendancePercentage == attendancePercentage)&&(identical(other.attendenceFatCat, attendenceFatCat) || other.attendenceFatCat == attendenceFatCat)&&(identical(other.debarStatus, debarStatus) || other.debarStatus == debarStatus)&&(identical(other.courseId, courseId) || other.courseId == courseId));
}

@JsonKey(includeFromJson: false, includeToJson: false)
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


/// Adds pattern-matching-related methods to [AttendanceRecord].
extension AttendanceRecordPatterns on AttendanceRecord {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _AttendanceRecord value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _AttendanceRecord() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _AttendanceRecord value)  $default,){
final _that = this;
switch (_that) {
case _AttendanceRecord():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _AttendanceRecord value)?  $default,){
final _that = this;
switch (_that) {
case _AttendanceRecord() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String serial,  String category,  String courseName,  String courseCode,  String courseType,  String facultyDetail,  String classesAttended,  String totalClasses,  String attendancePercentage,  String attendenceFatCat,  String debarStatus,  String courseId)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _AttendanceRecord() when $default != null:
return $default(_that.serial,_that.category,_that.courseName,_that.courseCode,_that.courseType,_that.facultyDetail,_that.classesAttended,_that.totalClasses,_that.attendancePercentage,_that.attendenceFatCat,_that.debarStatus,_that.courseId);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String serial,  String category,  String courseName,  String courseCode,  String courseType,  String facultyDetail,  String classesAttended,  String totalClasses,  String attendancePercentage,  String attendenceFatCat,  String debarStatus,  String courseId)  $default,) {final _that = this;
switch (_that) {
case _AttendanceRecord():
return $default(_that.serial,_that.category,_that.courseName,_that.courseCode,_that.courseType,_that.facultyDetail,_that.classesAttended,_that.totalClasses,_that.attendancePercentage,_that.attendenceFatCat,_that.debarStatus,_that.courseId);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String serial,  String category,  String courseName,  String courseCode,  String courseType,  String facultyDetail,  String classesAttended,  String totalClasses,  String attendancePercentage,  String attendenceFatCat,  String debarStatus,  String courseId)?  $default,) {final _that = this;
switch (_that) {
case _AttendanceRecord() when $default != null:
return $default(_that.serial,_that.category,_that.courseName,_that.courseCode,_that.courseType,_that.facultyDetail,_that.classesAttended,_that.totalClasses,_that.attendancePercentage,_that.attendenceFatCat,_that.debarStatus,_that.courseId);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _AttendanceRecord implements AttendanceRecord {
  const _AttendanceRecord({required this.serial, required this.category, required this.courseName, required this.courseCode, required this.courseType, required this.facultyDetail, required this.classesAttended, required this.totalClasses, required this.attendancePercentage, required this.attendenceFatCat, required this.debarStatus, required this.courseId});
  factory _AttendanceRecord.fromJson(Map<String, dynamic> json) => _$AttendanceRecordFromJson(json);

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
Map<String, dynamic> toJson() {
  return _$AttendanceRecordToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _AttendanceRecord&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.category, category) || other.category == category)&&(identical(other.courseName, courseName) || other.courseName == courseName)&&(identical(other.courseCode, courseCode) || other.courseCode == courseCode)&&(identical(other.courseType, courseType) || other.courseType == courseType)&&(identical(other.facultyDetail, facultyDetail) || other.facultyDetail == facultyDetail)&&(identical(other.classesAttended, classesAttended) || other.classesAttended == classesAttended)&&(identical(other.totalClasses, totalClasses) || other.totalClasses == totalClasses)&&(identical(other.attendancePercentage, attendancePercentage) || other.attendancePercentage == attendancePercentage)&&(identical(other.attendenceFatCat, attendenceFatCat) || other.attendenceFatCat == attendenceFatCat)&&(identical(other.debarStatus, debarStatus) || other.debarStatus == debarStatus)&&(identical(other.courseId, courseId) || other.courseId == courseId));
}

@JsonKey(includeFromJson: false, includeToJson: false)
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

  /// Serializes this ExamScheduleData to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ExamScheduleData&&const DeepCollectionEquality().equals(other.exams, exams)&&(identical(other.semesterId, semesterId) || other.semesterId == semesterId)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}

@JsonKey(includeFromJson: false, includeToJson: false)
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


/// Adds pattern-matching-related methods to [ExamScheduleData].
extension ExamScheduleDataPatterns on ExamScheduleData {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _ExamScheduleData value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _ExamScheduleData() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _ExamScheduleData value)  $default,){
final _that = this;
switch (_that) {
case _ExamScheduleData():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _ExamScheduleData value)?  $default,){
final _that = this;
switch (_that) {
case _ExamScheduleData() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( List<PerExamScheduleRecord> exams,  String semesterId,  BigInt updateTime)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _ExamScheduleData() when $default != null:
return $default(_that.exams,_that.semesterId,_that.updateTime);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( List<PerExamScheduleRecord> exams,  String semesterId,  BigInt updateTime)  $default,) {final _that = this;
switch (_that) {
case _ExamScheduleData():
return $default(_that.exams,_that.semesterId,_that.updateTime);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( List<PerExamScheduleRecord> exams,  String semesterId,  BigInt updateTime)?  $default,) {final _that = this;
switch (_that) {
case _ExamScheduleData() when $default != null:
return $default(_that.exams,_that.semesterId,_that.updateTime);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _ExamScheduleData implements ExamScheduleData {
  const _ExamScheduleData({required final  List<PerExamScheduleRecord> exams, required this.semesterId, required this.updateTime}): _exams = exams;
  factory _ExamScheduleData.fromJson(Map<String, dynamic> json) => _$ExamScheduleDataFromJson(json);

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
Map<String, dynamic> toJson() {
  return _$ExamScheduleDataToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _ExamScheduleData&&const DeepCollectionEquality().equals(other._exams, _exams)&&(identical(other.semesterId, semesterId) || other.semesterId == semesterId)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}

@JsonKey(includeFromJson: false, includeToJson: false)
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

  /// Serializes this ExamScheduleRecord to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ExamScheduleRecord&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.slot, slot) || other.slot == slot)&&(identical(other.courseName, courseName) || other.courseName == courseName)&&(identical(other.courseCode, courseCode) || other.courseCode == courseCode)&&(identical(other.courseType, courseType) || other.courseType == courseType)&&(identical(other.courseId, courseId) || other.courseId == courseId)&&(identical(other.examDate, examDate) || other.examDate == examDate)&&(identical(other.examSession, examSession) || other.examSession == examSession)&&(identical(other.reportingTime, reportingTime) || other.reportingTime == reportingTime)&&(identical(other.examTime, examTime) || other.examTime == examTime)&&(identical(other.venue, venue) || other.venue == venue)&&(identical(other.seatLocation, seatLocation) || other.seatLocation == seatLocation)&&(identical(other.seatNo, seatNo) || other.seatNo == seatNo));
}

@JsonKey(includeFromJson: false, includeToJson: false)
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


/// Adds pattern-matching-related methods to [ExamScheduleRecord].
extension ExamScheduleRecordPatterns on ExamScheduleRecord {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _ExamScheduleRecord value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _ExamScheduleRecord() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _ExamScheduleRecord value)  $default,){
final _that = this;
switch (_that) {
case _ExamScheduleRecord():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _ExamScheduleRecord value)?  $default,){
final _that = this;
switch (_that) {
case _ExamScheduleRecord() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String serial,  String slot,  String courseName,  String courseCode,  String courseType,  String courseId,  String examDate,  String examSession,  String reportingTime,  String examTime,  String venue,  String seatLocation,  String seatNo)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _ExamScheduleRecord() when $default != null:
return $default(_that.serial,_that.slot,_that.courseName,_that.courseCode,_that.courseType,_that.courseId,_that.examDate,_that.examSession,_that.reportingTime,_that.examTime,_that.venue,_that.seatLocation,_that.seatNo);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String serial,  String slot,  String courseName,  String courseCode,  String courseType,  String courseId,  String examDate,  String examSession,  String reportingTime,  String examTime,  String venue,  String seatLocation,  String seatNo)  $default,) {final _that = this;
switch (_that) {
case _ExamScheduleRecord():
return $default(_that.serial,_that.slot,_that.courseName,_that.courseCode,_that.courseType,_that.courseId,_that.examDate,_that.examSession,_that.reportingTime,_that.examTime,_that.venue,_that.seatLocation,_that.seatNo);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String serial,  String slot,  String courseName,  String courseCode,  String courseType,  String courseId,  String examDate,  String examSession,  String reportingTime,  String examTime,  String venue,  String seatLocation,  String seatNo)?  $default,) {final _that = this;
switch (_that) {
case _ExamScheduleRecord() when $default != null:
return $default(_that.serial,_that.slot,_that.courseName,_that.courseCode,_that.courseType,_that.courseId,_that.examDate,_that.examSession,_that.reportingTime,_that.examTime,_that.venue,_that.seatLocation,_that.seatNo);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _ExamScheduleRecord implements ExamScheduleRecord {
  const _ExamScheduleRecord({required this.serial, required this.slot, required this.courseName, required this.courseCode, required this.courseType, required this.courseId, required this.examDate, required this.examSession, required this.reportingTime, required this.examTime, required this.venue, required this.seatLocation, required this.seatNo});
  factory _ExamScheduleRecord.fromJson(Map<String, dynamic> json) => _$ExamScheduleRecordFromJson(json);

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
Map<String, dynamic> toJson() {
  return _$ExamScheduleRecordToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _ExamScheduleRecord&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.slot, slot) || other.slot == slot)&&(identical(other.courseName, courseName) || other.courseName == courseName)&&(identical(other.courseCode, courseCode) || other.courseCode == courseCode)&&(identical(other.courseType, courseType) || other.courseType == courseType)&&(identical(other.courseId, courseId) || other.courseId == courseId)&&(identical(other.examDate, examDate) || other.examDate == examDate)&&(identical(other.examSession, examSession) || other.examSession == examSession)&&(identical(other.reportingTime, reportingTime) || other.reportingTime == reportingTime)&&(identical(other.examTime, examTime) || other.examTime == examTime)&&(identical(other.venue, venue) || other.venue == venue)&&(identical(other.seatLocation, seatLocation) || other.seatLocation == seatLocation)&&(identical(other.seatNo, seatNo) || other.seatNo == seatNo));
}

@JsonKey(includeFromJson: false, includeToJson: false)
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

  /// Serializes this FullAttendanceData to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is FullAttendanceData&&const DeepCollectionEquality().equals(other.records, records)&&(identical(other.semesterId, semesterId) || other.semesterId == semesterId)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime)&&(identical(other.courseId, courseId) || other.courseId == courseId)&&(identical(other.courseType, courseType) || other.courseType == courseType));
}

@JsonKey(includeFromJson: false, includeToJson: false)
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


/// Adds pattern-matching-related methods to [FullAttendanceData].
extension FullAttendanceDataPatterns on FullAttendanceData {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _FullAttendanceData value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _FullAttendanceData() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _FullAttendanceData value)  $default,){
final _that = this;
switch (_that) {
case _FullAttendanceData():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _FullAttendanceData value)?  $default,){
final _that = this;
switch (_that) {
case _FullAttendanceData() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( List<FullAttendanceRecord> records,  String semesterId,  BigInt updateTime,  String courseId,  String courseType)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _FullAttendanceData() when $default != null:
return $default(_that.records,_that.semesterId,_that.updateTime,_that.courseId,_that.courseType);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( List<FullAttendanceRecord> records,  String semesterId,  BigInt updateTime,  String courseId,  String courseType)  $default,) {final _that = this;
switch (_that) {
case _FullAttendanceData():
return $default(_that.records,_that.semesterId,_that.updateTime,_that.courseId,_that.courseType);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( List<FullAttendanceRecord> records,  String semesterId,  BigInt updateTime,  String courseId,  String courseType)?  $default,) {final _that = this;
switch (_that) {
case _FullAttendanceData() when $default != null:
return $default(_that.records,_that.semesterId,_that.updateTime,_that.courseId,_that.courseType);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _FullAttendanceData implements FullAttendanceData {
  const _FullAttendanceData({required final  List<FullAttendanceRecord> records, required this.semesterId, required this.updateTime, required this.courseId, required this.courseType}): _records = records;
  factory _FullAttendanceData.fromJson(Map<String, dynamic> json) => _$FullAttendanceDataFromJson(json);

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
Map<String, dynamic> toJson() {
  return _$FullAttendanceDataToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _FullAttendanceData&&const DeepCollectionEquality().equals(other._records, _records)&&(identical(other.semesterId, semesterId) || other.semesterId == semesterId)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime)&&(identical(other.courseId, courseId) || other.courseId == courseId)&&(identical(other.courseType, courseType) || other.courseType == courseType));
}

@JsonKey(includeFromJson: false, includeToJson: false)
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

  /// Serializes this FullAttendanceRecord to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is FullAttendanceRecord&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.date, date) || other.date == date)&&(identical(other.slot, slot) || other.slot == slot)&&(identical(other.dayTime, dayTime) || other.dayTime == dayTime)&&(identical(other.status, status) || other.status == status)&&(identical(other.remark, remark) || other.remark == remark));
}

@JsonKey(includeFromJson: false, includeToJson: false)
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


/// Adds pattern-matching-related methods to [FullAttendanceRecord].
extension FullAttendanceRecordPatterns on FullAttendanceRecord {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _FullAttendanceRecord value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _FullAttendanceRecord() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _FullAttendanceRecord value)  $default,){
final _that = this;
switch (_that) {
case _FullAttendanceRecord():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _FullAttendanceRecord value)?  $default,){
final _that = this;
switch (_that) {
case _FullAttendanceRecord() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String serial,  String date,  String slot,  String dayTime,  String status,  String remark)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _FullAttendanceRecord() when $default != null:
return $default(_that.serial,_that.date,_that.slot,_that.dayTime,_that.status,_that.remark);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String serial,  String date,  String slot,  String dayTime,  String status,  String remark)  $default,) {final _that = this;
switch (_that) {
case _FullAttendanceRecord():
return $default(_that.serial,_that.date,_that.slot,_that.dayTime,_that.status,_that.remark);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String serial,  String date,  String slot,  String dayTime,  String status,  String remark)?  $default,) {final _that = this;
switch (_that) {
case _FullAttendanceRecord() when $default != null:
return $default(_that.serial,_that.date,_that.slot,_that.dayTime,_that.status,_that.remark);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _FullAttendanceRecord implements FullAttendanceRecord {
  const _FullAttendanceRecord({required this.serial, required this.date, required this.slot, required this.dayTime, required this.status, required this.remark});
  factory _FullAttendanceRecord.fromJson(Map<String, dynamic> json) => _$FullAttendanceRecordFromJson(json);

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
Map<String, dynamic> toJson() {
  return _$FullAttendanceRecordToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _FullAttendanceRecord&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.date, date) || other.date == date)&&(identical(other.slot, slot) || other.slot == slot)&&(identical(other.dayTime, dayTime) || other.dayTime == dayTime)&&(identical(other.status, status) || other.status == status)&&(identical(other.remark, remark) || other.remark == remark));
}

@JsonKey(includeFromJson: false, includeToJson: false)
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
mixin _$GradeCourseRecord {

 String get serial; String get courseCode; String get courseTitle; String get courseType; String get gradingType; String get grandTotal; String get grade; String get courseId;
/// Create a copy of GradeCourseRecord
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GradeCourseRecordCopyWith<GradeCourseRecord> get copyWith => _$GradeCourseRecordCopyWithImpl<GradeCourseRecord>(this as GradeCourseRecord, _$identity);

  /// Serializes this GradeCourseRecord to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is GradeCourseRecord&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.courseCode, courseCode) || other.courseCode == courseCode)&&(identical(other.courseTitle, courseTitle) || other.courseTitle == courseTitle)&&(identical(other.courseType, courseType) || other.courseType == courseType)&&(identical(other.gradingType, gradingType) || other.gradingType == gradingType)&&(identical(other.grandTotal, grandTotal) || other.grandTotal == grandTotal)&&(identical(other.grade, grade) || other.grade == grade)&&(identical(other.courseId, courseId) || other.courseId == courseId));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,serial,courseCode,courseTitle,courseType,gradingType,grandTotal,grade,courseId);

@override
String toString() {
  return 'GradeCourseRecord(serial: $serial, courseCode: $courseCode, courseTitle: $courseTitle, courseType: $courseType, gradingType: $gradingType, grandTotal: $grandTotal, grade: $grade, courseId: $courseId)';
}


}

/// @nodoc
abstract mixin class $GradeCourseRecordCopyWith<$Res>  {
  factory $GradeCourseRecordCopyWith(GradeCourseRecord value, $Res Function(GradeCourseRecord) _then) = _$GradeCourseRecordCopyWithImpl;
@useResult
$Res call({
 String serial, String courseCode, String courseTitle, String courseType, String gradingType, String grandTotal, String grade, String courseId
});




}
/// @nodoc
class _$GradeCourseRecordCopyWithImpl<$Res>
    implements $GradeCourseRecordCopyWith<$Res> {
  _$GradeCourseRecordCopyWithImpl(this._self, this._then);

  final GradeCourseRecord _self;
  final $Res Function(GradeCourseRecord) _then;

/// Create a copy of GradeCourseRecord
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? serial = null,Object? courseCode = null,Object? courseTitle = null,Object? courseType = null,Object? gradingType = null,Object? grandTotal = null,Object? grade = null,Object? courseId = null,}) {
  return _then(_self.copyWith(
serial: null == serial ? _self.serial : serial // ignore: cast_nullable_to_non_nullable
as String,courseCode: null == courseCode ? _self.courseCode : courseCode // ignore: cast_nullable_to_non_nullable
as String,courseTitle: null == courseTitle ? _self.courseTitle : courseTitle // ignore: cast_nullable_to_non_nullable
as String,courseType: null == courseType ? _self.courseType : courseType // ignore: cast_nullable_to_non_nullable
as String,gradingType: null == gradingType ? _self.gradingType : gradingType // ignore: cast_nullable_to_non_nullable
as String,grandTotal: null == grandTotal ? _self.grandTotal : grandTotal // ignore: cast_nullable_to_non_nullable
as String,grade: null == grade ? _self.grade : grade // ignore: cast_nullable_to_non_nullable
as String,courseId: null == courseId ? _self.courseId : courseId // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// Adds pattern-matching-related methods to [GradeCourseRecord].
extension GradeCourseRecordPatterns on GradeCourseRecord {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _GradeCourseRecord value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _GradeCourseRecord() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _GradeCourseRecord value)  $default,){
final _that = this;
switch (_that) {
case _GradeCourseRecord():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _GradeCourseRecord value)?  $default,){
final _that = this;
switch (_that) {
case _GradeCourseRecord() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String serial,  String courseCode,  String courseTitle,  String courseType,  String gradingType,  String grandTotal,  String grade,  String courseId)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _GradeCourseRecord() when $default != null:
return $default(_that.serial,_that.courseCode,_that.courseTitle,_that.courseType,_that.gradingType,_that.grandTotal,_that.grade,_that.courseId);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String serial,  String courseCode,  String courseTitle,  String courseType,  String gradingType,  String grandTotal,  String grade,  String courseId)  $default,) {final _that = this;
switch (_that) {
case _GradeCourseRecord():
return $default(_that.serial,_that.courseCode,_that.courseTitle,_that.courseType,_that.gradingType,_that.grandTotal,_that.grade,_that.courseId);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String serial,  String courseCode,  String courseTitle,  String courseType,  String gradingType,  String grandTotal,  String grade,  String courseId)?  $default,) {final _that = this;
switch (_that) {
case _GradeCourseRecord() when $default != null:
return $default(_that.serial,_that.courseCode,_that.courseTitle,_that.courseType,_that.gradingType,_that.grandTotal,_that.grade,_that.courseId);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _GradeCourseRecord implements GradeCourseRecord {
  const _GradeCourseRecord({required this.serial, required this.courseCode, required this.courseTitle, required this.courseType, required this.gradingType, required this.grandTotal, required this.grade, required this.courseId});
  factory _GradeCourseRecord.fromJson(Map<String, dynamic> json) => _$GradeCourseRecordFromJson(json);

@override final  String serial;
@override final  String courseCode;
@override final  String courseTitle;
@override final  String courseType;
@override final  String gradingType;
@override final  String grandTotal;
@override final  String grade;
@override final  String courseId;

/// Create a copy of GradeCourseRecord
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$GradeCourseRecordCopyWith<_GradeCourseRecord> get copyWith => __$GradeCourseRecordCopyWithImpl<_GradeCourseRecord>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$GradeCourseRecordToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _GradeCourseRecord&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.courseCode, courseCode) || other.courseCode == courseCode)&&(identical(other.courseTitle, courseTitle) || other.courseTitle == courseTitle)&&(identical(other.courseType, courseType) || other.courseType == courseType)&&(identical(other.gradingType, gradingType) || other.gradingType == gradingType)&&(identical(other.grandTotal, grandTotal) || other.grandTotal == grandTotal)&&(identical(other.grade, grade) || other.grade == grade)&&(identical(other.courseId, courseId) || other.courseId == courseId));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,serial,courseCode,courseTitle,courseType,gradingType,grandTotal,grade,courseId);

@override
String toString() {
  return 'GradeCourseRecord(serial: $serial, courseCode: $courseCode, courseTitle: $courseTitle, courseType: $courseType, gradingType: $gradingType, grandTotal: $grandTotal, grade: $grade, courseId: $courseId)';
}


}

/// @nodoc
abstract mixin class _$GradeCourseRecordCopyWith<$Res> implements $GradeCourseRecordCopyWith<$Res> {
  factory _$GradeCourseRecordCopyWith(_GradeCourseRecord value, $Res Function(_GradeCourseRecord) _then) = __$GradeCourseRecordCopyWithImpl;
@override @useResult
$Res call({
 String serial, String courseCode, String courseTitle, String courseType, String gradingType, String grandTotal, String grade, String courseId
});




}
/// @nodoc
class __$GradeCourseRecordCopyWithImpl<$Res>
    implements _$GradeCourseRecordCopyWith<$Res> {
  __$GradeCourseRecordCopyWithImpl(this._self, this._then);

  final _GradeCourseRecord _self;
  final $Res Function(_GradeCourseRecord) _then;

/// Create a copy of GradeCourseRecord
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? serial = null,Object? courseCode = null,Object? courseTitle = null,Object? courseType = null,Object? gradingType = null,Object? grandTotal = null,Object? grade = null,Object? courseId = null,}) {
  return _then(_GradeCourseRecord(
serial: null == serial ? _self.serial : serial // ignore: cast_nullable_to_non_nullable
as String,courseCode: null == courseCode ? _self.courseCode : courseCode // ignore: cast_nullable_to_non_nullable
as String,courseTitle: null == courseTitle ? _self.courseTitle : courseTitle // ignore: cast_nullable_to_non_nullable
as String,courseType: null == courseType ? _self.courseType : courseType // ignore: cast_nullable_to_non_nullable
as String,gradingType: null == gradingType ? _self.gradingType : gradingType // ignore: cast_nullable_to_non_nullable
as String,grandTotal: null == grandTotal ? _self.grandTotal : grandTotal // ignore: cast_nullable_to_non_nullable
as String,grade: null == grade ? _self.grade : grade // ignore: cast_nullable_to_non_nullable
as String,courseId: null == courseId ? _self.courseId : courseId // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}


/// @nodoc
mixin _$GradeDetailMark {

 String get serial; String get markTitle; String get maxMark; String get weightage; String get status; String get scoredMark; String get weightageMark;
/// Create a copy of GradeDetailMark
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GradeDetailMarkCopyWith<GradeDetailMark> get copyWith => _$GradeDetailMarkCopyWithImpl<GradeDetailMark>(this as GradeDetailMark, _$identity);

  /// Serializes this GradeDetailMark to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is GradeDetailMark&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.markTitle, markTitle) || other.markTitle == markTitle)&&(identical(other.maxMark, maxMark) || other.maxMark == maxMark)&&(identical(other.weightage, weightage) || other.weightage == weightage)&&(identical(other.status, status) || other.status == status)&&(identical(other.scoredMark, scoredMark) || other.scoredMark == scoredMark)&&(identical(other.weightageMark, weightageMark) || other.weightageMark == weightageMark));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,serial,markTitle,maxMark,weightage,status,scoredMark,weightageMark);

@override
String toString() {
  return 'GradeDetailMark(serial: $serial, markTitle: $markTitle, maxMark: $maxMark, weightage: $weightage, status: $status, scoredMark: $scoredMark, weightageMark: $weightageMark)';
}


}

/// @nodoc
abstract mixin class $GradeDetailMarkCopyWith<$Res>  {
  factory $GradeDetailMarkCopyWith(GradeDetailMark value, $Res Function(GradeDetailMark) _then) = _$GradeDetailMarkCopyWithImpl;
@useResult
$Res call({
 String serial, String markTitle, String maxMark, String weightage, String status, String scoredMark, String weightageMark
});




}
/// @nodoc
class _$GradeDetailMarkCopyWithImpl<$Res>
    implements $GradeDetailMarkCopyWith<$Res> {
  _$GradeDetailMarkCopyWithImpl(this._self, this._then);

  final GradeDetailMark _self;
  final $Res Function(GradeDetailMark) _then;

/// Create a copy of GradeDetailMark
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? serial = null,Object? markTitle = null,Object? maxMark = null,Object? weightage = null,Object? status = null,Object? scoredMark = null,Object? weightageMark = null,}) {
  return _then(_self.copyWith(
serial: null == serial ? _self.serial : serial // ignore: cast_nullable_to_non_nullable
as String,markTitle: null == markTitle ? _self.markTitle : markTitle // ignore: cast_nullable_to_non_nullable
as String,maxMark: null == maxMark ? _self.maxMark : maxMark // ignore: cast_nullable_to_non_nullable
as String,weightage: null == weightage ? _self.weightage : weightage // ignore: cast_nullable_to_non_nullable
as String,status: null == status ? _self.status : status // ignore: cast_nullable_to_non_nullable
as String,scoredMark: null == scoredMark ? _self.scoredMark : scoredMark // ignore: cast_nullable_to_non_nullable
as String,weightageMark: null == weightageMark ? _self.weightageMark : weightageMark // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// Adds pattern-matching-related methods to [GradeDetailMark].
extension GradeDetailMarkPatterns on GradeDetailMark {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _GradeDetailMark value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _GradeDetailMark() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _GradeDetailMark value)  $default,){
final _that = this;
switch (_that) {
case _GradeDetailMark():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _GradeDetailMark value)?  $default,){
final _that = this;
switch (_that) {
case _GradeDetailMark() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String serial,  String markTitle,  String maxMark,  String weightage,  String status,  String scoredMark,  String weightageMark)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _GradeDetailMark() when $default != null:
return $default(_that.serial,_that.markTitle,_that.maxMark,_that.weightage,_that.status,_that.scoredMark,_that.weightageMark);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String serial,  String markTitle,  String maxMark,  String weightage,  String status,  String scoredMark,  String weightageMark)  $default,) {final _that = this;
switch (_that) {
case _GradeDetailMark():
return $default(_that.serial,_that.markTitle,_that.maxMark,_that.weightage,_that.status,_that.scoredMark,_that.weightageMark);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String serial,  String markTitle,  String maxMark,  String weightage,  String status,  String scoredMark,  String weightageMark)?  $default,) {final _that = this;
switch (_that) {
case _GradeDetailMark() when $default != null:
return $default(_that.serial,_that.markTitle,_that.maxMark,_that.weightage,_that.status,_that.scoredMark,_that.weightageMark);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _GradeDetailMark implements GradeDetailMark {
  const _GradeDetailMark({required this.serial, required this.markTitle, required this.maxMark, required this.weightage, required this.status, required this.scoredMark, required this.weightageMark});
  factory _GradeDetailMark.fromJson(Map<String, dynamic> json) => _$GradeDetailMarkFromJson(json);

@override final  String serial;
@override final  String markTitle;
@override final  String maxMark;
@override final  String weightage;
@override final  String status;
@override final  String scoredMark;
@override final  String weightageMark;

/// Create a copy of GradeDetailMark
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$GradeDetailMarkCopyWith<_GradeDetailMark> get copyWith => __$GradeDetailMarkCopyWithImpl<_GradeDetailMark>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$GradeDetailMarkToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _GradeDetailMark&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.markTitle, markTitle) || other.markTitle == markTitle)&&(identical(other.maxMark, maxMark) || other.maxMark == maxMark)&&(identical(other.weightage, weightage) || other.weightage == weightage)&&(identical(other.status, status) || other.status == status)&&(identical(other.scoredMark, scoredMark) || other.scoredMark == scoredMark)&&(identical(other.weightageMark, weightageMark) || other.weightageMark == weightageMark));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,serial,markTitle,maxMark,weightage,status,scoredMark,weightageMark);

@override
String toString() {
  return 'GradeDetailMark(serial: $serial, markTitle: $markTitle, maxMark: $maxMark, weightage: $weightage, status: $status, scoredMark: $scoredMark, weightageMark: $weightageMark)';
}


}

/// @nodoc
abstract mixin class _$GradeDetailMarkCopyWith<$Res> implements $GradeDetailMarkCopyWith<$Res> {
  factory _$GradeDetailMarkCopyWith(_GradeDetailMark value, $Res Function(_GradeDetailMark) _then) = __$GradeDetailMarkCopyWithImpl;
@override @useResult
$Res call({
 String serial, String markTitle, String maxMark, String weightage, String status, String scoredMark, String weightageMark
});




}
/// @nodoc
class __$GradeDetailMarkCopyWithImpl<$Res>
    implements _$GradeDetailMarkCopyWith<$Res> {
  __$GradeDetailMarkCopyWithImpl(this._self, this._then);

  final _GradeDetailMark _self;
  final $Res Function(_GradeDetailMark) _then;

/// Create a copy of GradeDetailMark
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? serial = null,Object? markTitle = null,Object? maxMark = null,Object? weightage = null,Object? status = null,Object? scoredMark = null,Object? weightageMark = null,}) {
  return _then(_GradeDetailMark(
serial: null == serial ? _self.serial : serial // ignore: cast_nullable_to_non_nullable
as String,markTitle: null == markTitle ? _self.markTitle : markTitle // ignore: cast_nullable_to_non_nullable
as String,maxMark: null == maxMark ? _self.maxMark : maxMark // ignore: cast_nullable_to_non_nullable
as String,weightage: null == weightage ? _self.weightage : weightage // ignore: cast_nullable_to_non_nullable
as String,status: null == status ? _self.status : status // ignore: cast_nullable_to_non_nullable
as String,scoredMark: null == scoredMark ? _self.scoredMark : scoredMark // ignore: cast_nullable_to_non_nullable
as String,weightageMark: null == weightageMark ? _self.weightageMark : weightageMark // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}


/// @nodoc
mixin _$GradeDetailsData {

 String get semesterId; String get courseId; String get classNumber; String get classCourseType; String get grandTotal; List<GradeDetailMark> get marks; List<GradeRange> get gradeRanges; BigInt get updateTime;
/// Create a copy of GradeDetailsData
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GradeDetailsDataCopyWith<GradeDetailsData> get copyWith => _$GradeDetailsDataCopyWithImpl<GradeDetailsData>(this as GradeDetailsData, _$identity);

  /// Serializes this GradeDetailsData to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is GradeDetailsData&&(identical(other.semesterId, semesterId) || other.semesterId == semesterId)&&(identical(other.courseId, courseId) || other.courseId == courseId)&&(identical(other.classNumber, classNumber) || other.classNumber == classNumber)&&(identical(other.classCourseType, classCourseType) || other.classCourseType == classCourseType)&&(identical(other.grandTotal, grandTotal) || other.grandTotal == grandTotal)&&const DeepCollectionEquality().equals(other.marks, marks)&&const DeepCollectionEquality().equals(other.gradeRanges, gradeRanges)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,semesterId,courseId,classNumber,classCourseType,grandTotal,const DeepCollectionEquality().hash(marks),const DeepCollectionEquality().hash(gradeRanges),updateTime);

@override
String toString() {
  return 'GradeDetailsData(semesterId: $semesterId, courseId: $courseId, classNumber: $classNumber, classCourseType: $classCourseType, grandTotal: $grandTotal, marks: $marks, gradeRanges: $gradeRanges, updateTime: $updateTime)';
}


}

/// @nodoc
abstract mixin class $GradeDetailsDataCopyWith<$Res>  {
  factory $GradeDetailsDataCopyWith(GradeDetailsData value, $Res Function(GradeDetailsData) _then) = _$GradeDetailsDataCopyWithImpl;
@useResult
$Res call({
 String semesterId, String courseId, String classNumber, String classCourseType, String grandTotal, List<GradeDetailMark> marks, List<GradeRange> gradeRanges, BigInt updateTime
});




}
/// @nodoc
class _$GradeDetailsDataCopyWithImpl<$Res>
    implements $GradeDetailsDataCopyWith<$Res> {
  _$GradeDetailsDataCopyWithImpl(this._self, this._then);

  final GradeDetailsData _self;
  final $Res Function(GradeDetailsData) _then;

/// Create a copy of GradeDetailsData
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? semesterId = null,Object? courseId = null,Object? classNumber = null,Object? classCourseType = null,Object? grandTotal = null,Object? marks = null,Object? gradeRanges = null,Object? updateTime = null,}) {
  return _then(_self.copyWith(
semesterId: null == semesterId ? _self.semesterId : semesterId // ignore: cast_nullable_to_non_nullable
as String,courseId: null == courseId ? _self.courseId : courseId // ignore: cast_nullable_to_non_nullable
as String,classNumber: null == classNumber ? _self.classNumber : classNumber // ignore: cast_nullable_to_non_nullable
as String,classCourseType: null == classCourseType ? _self.classCourseType : classCourseType // ignore: cast_nullable_to_non_nullable
as String,grandTotal: null == grandTotal ? _self.grandTotal : grandTotal // ignore: cast_nullable_to_non_nullable
as String,marks: null == marks ? _self.marks : marks // ignore: cast_nullable_to_non_nullable
as List<GradeDetailMark>,gradeRanges: null == gradeRanges ? _self.gradeRanges : gradeRanges // ignore: cast_nullable_to_non_nullable
as List<GradeRange>,updateTime: null == updateTime ? _self.updateTime : updateTime // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}

}


/// Adds pattern-matching-related methods to [GradeDetailsData].
extension GradeDetailsDataPatterns on GradeDetailsData {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _GradeDetailsData value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _GradeDetailsData() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _GradeDetailsData value)  $default,){
final _that = this;
switch (_that) {
case _GradeDetailsData():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _GradeDetailsData value)?  $default,){
final _that = this;
switch (_that) {
case _GradeDetailsData() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String semesterId,  String courseId,  String classNumber,  String classCourseType,  String grandTotal,  List<GradeDetailMark> marks,  List<GradeRange> gradeRanges,  BigInt updateTime)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _GradeDetailsData() when $default != null:
return $default(_that.semesterId,_that.courseId,_that.classNumber,_that.classCourseType,_that.grandTotal,_that.marks,_that.gradeRanges,_that.updateTime);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String semesterId,  String courseId,  String classNumber,  String classCourseType,  String grandTotal,  List<GradeDetailMark> marks,  List<GradeRange> gradeRanges,  BigInt updateTime)  $default,) {final _that = this;
switch (_that) {
case _GradeDetailsData():
return $default(_that.semesterId,_that.courseId,_that.classNumber,_that.classCourseType,_that.grandTotal,_that.marks,_that.gradeRanges,_that.updateTime);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String semesterId,  String courseId,  String classNumber,  String classCourseType,  String grandTotal,  List<GradeDetailMark> marks,  List<GradeRange> gradeRanges,  BigInt updateTime)?  $default,) {final _that = this;
switch (_that) {
case _GradeDetailsData() when $default != null:
return $default(_that.semesterId,_that.courseId,_that.classNumber,_that.classCourseType,_that.grandTotal,_that.marks,_that.gradeRanges,_that.updateTime);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _GradeDetailsData implements GradeDetailsData {
  const _GradeDetailsData({required this.semesterId, required this.courseId, required this.classNumber, required this.classCourseType, required this.grandTotal, required final  List<GradeDetailMark> marks, required final  List<GradeRange> gradeRanges, required this.updateTime}): _marks = marks,_gradeRanges = gradeRanges;
  factory _GradeDetailsData.fromJson(Map<String, dynamic> json) => _$GradeDetailsDataFromJson(json);

@override final  String semesterId;
@override final  String courseId;
@override final  String classNumber;
@override final  String classCourseType;
@override final  String grandTotal;
 final  List<GradeDetailMark> _marks;
@override List<GradeDetailMark> get marks {
  if (_marks is EqualUnmodifiableListView) return _marks;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_marks);
}

 final  List<GradeRange> _gradeRanges;
@override List<GradeRange> get gradeRanges {
  if (_gradeRanges is EqualUnmodifiableListView) return _gradeRanges;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_gradeRanges);
}

@override final  BigInt updateTime;

/// Create a copy of GradeDetailsData
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$GradeDetailsDataCopyWith<_GradeDetailsData> get copyWith => __$GradeDetailsDataCopyWithImpl<_GradeDetailsData>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$GradeDetailsDataToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _GradeDetailsData&&(identical(other.semesterId, semesterId) || other.semesterId == semesterId)&&(identical(other.courseId, courseId) || other.courseId == courseId)&&(identical(other.classNumber, classNumber) || other.classNumber == classNumber)&&(identical(other.classCourseType, classCourseType) || other.classCourseType == classCourseType)&&(identical(other.grandTotal, grandTotal) || other.grandTotal == grandTotal)&&const DeepCollectionEquality().equals(other._marks, _marks)&&const DeepCollectionEquality().equals(other._gradeRanges, _gradeRanges)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,semesterId,courseId,classNumber,classCourseType,grandTotal,const DeepCollectionEquality().hash(_marks),const DeepCollectionEquality().hash(_gradeRanges),updateTime);

@override
String toString() {
  return 'GradeDetailsData(semesterId: $semesterId, courseId: $courseId, classNumber: $classNumber, classCourseType: $classCourseType, grandTotal: $grandTotal, marks: $marks, gradeRanges: $gradeRanges, updateTime: $updateTime)';
}


}

/// @nodoc
abstract mixin class _$GradeDetailsDataCopyWith<$Res> implements $GradeDetailsDataCopyWith<$Res> {
  factory _$GradeDetailsDataCopyWith(_GradeDetailsData value, $Res Function(_GradeDetailsData) _then) = __$GradeDetailsDataCopyWithImpl;
@override @useResult
$Res call({
 String semesterId, String courseId, String classNumber, String classCourseType, String grandTotal, List<GradeDetailMark> marks, List<GradeRange> gradeRanges, BigInt updateTime
});




}
/// @nodoc
class __$GradeDetailsDataCopyWithImpl<$Res>
    implements _$GradeDetailsDataCopyWith<$Res> {
  __$GradeDetailsDataCopyWithImpl(this._self, this._then);

  final _GradeDetailsData _self;
  final $Res Function(_GradeDetailsData) _then;

/// Create a copy of GradeDetailsData
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? semesterId = null,Object? courseId = null,Object? classNumber = null,Object? classCourseType = null,Object? grandTotal = null,Object? marks = null,Object? gradeRanges = null,Object? updateTime = null,}) {
  return _then(_GradeDetailsData(
semesterId: null == semesterId ? _self.semesterId : semesterId // ignore: cast_nullable_to_non_nullable
as String,courseId: null == courseId ? _self.courseId : courseId // ignore: cast_nullable_to_non_nullable
as String,classNumber: null == classNumber ? _self.classNumber : classNumber // ignore: cast_nullable_to_non_nullable
as String,classCourseType: null == classCourseType ? _self.classCourseType : classCourseType // ignore: cast_nullable_to_non_nullable
as String,grandTotal: null == grandTotal ? _self.grandTotal : grandTotal // ignore: cast_nullable_to_non_nullable
as String,marks: null == marks ? _self._marks : marks // ignore: cast_nullable_to_non_nullable
as List<GradeDetailMark>,gradeRanges: null == gradeRanges ? _self._gradeRanges : gradeRanges // ignore: cast_nullable_to_non_nullable
as List<GradeRange>,updateTime: null == updateTime ? _self.updateTime : updateTime // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}


/// @nodoc
mixin _$GradeHistoryAttempt {

 String get courseCode; String get courseTitle; String get courseType; String get credits; String get grade; String get examMonth; String get resultDeclared;
/// Create a copy of GradeHistoryAttempt
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GradeHistoryAttemptCopyWith<GradeHistoryAttempt> get copyWith => _$GradeHistoryAttemptCopyWithImpl<GradeHistoryAttempt>(this as GradeHistoryAttempt, _$identity);

  /// Serializes this GradeHistoryAttempt to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is GradeHistoryAttempt&&(identical(other.courseCode, courseCode) || other.courseCode == courseCode)&&(identical(other.courseTitle, courseTitle) || other.courseTitle == courseTitle)&&(identical(other.courseType, courseType) || other.courseType == courseType)&&(identical(other.credits, credits) || other.credits == credits)&&(identical(other.grade, grade) || other.grade == grade)&&(identical(other.examMonth, examMonth) || other.examMonth == examMonth)&&(identical(other.resultDeclared, resultDeclared) || other.resultDeclared == resultDeclared));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,courseCode,courseTitle,courseType,credits,grade,examMonth,resultDeclared);

@override
String toString() {
  return 'GradeHistoryAttempt(courseCode: $courseCode, courseTitle: $courseTitle, courseType: $courseType, credits: $credits, grade: $grade, examMonth: $examMonth, resultDeclared: $resultDeclared)';
}


}

/// @nodoc
abstract mixin class $GradeHistoryAttemptCopyWith<$Res>  {
  factory $GradeHistoryAttemptCopyWith(GradeHistoryAttempt value, $Res Function(GradeHistoryAttempt) _then) = _$GradeHistoryAttemptCopyWithImpl;
@useResult
$Res call({
 String courseCode, String courseTitle, String courseType, String credits, String grade, String examMonth, String resultDeclared
});




}
/// @nodoc
class _$GradeHistoryAttemptCopyWithImpl<$Res>
    implements $GradeHistoryAttemptCopyWith<$Res> {
  _$GradeHistoryAttemptCopyWithImpl(this._self, this._then);

  final GradeHistoryAttempt _self;
  final $Res Function(GradeHistoryAttempt) _then;

/// Create a copy of GradeHistoryAttempt
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? courseCode = null,Object? courseTitle = null,Object? courseType = null,Object? credits = null,Object? grade = null,Object? examMonth = null,Object? resultDeclared = null,}) {
  return _then(_self.copyWith(
courseCode: null == courseCode ? _self.courseCode : courseCode // ignore: cast_nullable_to_non_nullable
as String,courseTitle: null == courseTitle ? _self.courseTitle : courseTitle // ignore: cast_nullable_to_non_nullable
as String,courseType: null == courseType ? _self.courseType : courseType // ignore: cast_nullable_to_non_nullable
as String,credits: null == credits ? _self.credits : credits // ignore: cast_nullable_to_non_nullable
as String,grade: null == grade ? _self.grade : grade // ignore: cast_nullable_to_non_nullable
as String,examMonth: null == examMonth ? _self.examMonth : examMonth // ignore: cast_nullable_to_non_nullable
as String,resultDeclared: null == resultDeclared ? _self.resultDeclared : resultDeclared // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// Adds pattern-matching-related methods to [GradeHistoryAttempt].
extension GradeHistoryAttemptPatterns on GradeHistoryAttempt {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _GradeHistoryAttempt value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _GradeHistoryAttempt() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _GradeHistoryAttempt value)  $default,){
final _that = this;
switch (_that) {
case _GradeHistoryAttempt():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _GradeHistoryAttempt value)?  $default,){
final _that = this;
switch (_that) {
case _GradeHistoryAttempt() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String courseCode,  String courseTitle,  String courseType,  String credits,  String grade,  String examMonth,  String resultDeclared)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _GradeHistoryAttempt() when $default != null:
return $default(_that.courseCode,_that.courseTitle,_that.courseType,_that.credits,_that.grade,_that.examMonth,_that.resultDeclared);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String courseCode,  String courseTitle,  String courseType,  String credits,  String grade,  String examMonth,  String resultDeclared)  $default,) {final _that = this;
switch (_that) {
case _GradeHistoryAttempt():
return $default(_that.courseCode,_that.courseTitle,_that.courseType,_that.credits,_that.grade,_that.examMonth,_that.resultDeclared);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String courseCode,  String courseTitle,  String courseType,  String credits,  String grade,  String examMonth,  String resultDeclared)?  $default,) {final _that = this;
switch (_that) {
case _GradeHistoryAttempt() when $default != null:
return $default(_that.courseCode,_that.courseTitle,_that.courseType,_that.credits,_that.grade,_that.examMonth,_that.resultDeclared);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _GradeHistoryAttempt implements GradeHistoryAttempt {
  const _GradeHistoryAttempt({required this.courseCode, required this.courseTitle, required this.courseType, required this.credits, required this.grade, required this.examMonth, required this.resultDeclared});
  factory _GradeHistoryAttempt.fromJson(Map<String, dynamic> json) => _$GradeHistoryAttemptFromJson(json);

@override final  String courseCode;
@override final  String courseTitle;
@override final  String courseType;
@override final  String credits;
@override final  String grade;
@override final  String examMonth;
@override final  String resultDeclared;

/// Create a copy of GradeHistoryAttempt
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$GradeHistoryAttemptCopyWith<_GradeHistoryAttempt> get copyWith => __$GradeHistoryAttemptCopyWithImpl<_GradeHistoryAttempt>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$GradeHistoryAttemptToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _GradeHistoryAttempt&&(identical(other.courseCode, courseCode) || other.courseCode == courseCode)&&(identical(other.courseTitle, courseTitle) || other.courseTitle == courseTitle)&&(identical(other.courseType, courseType) || other.courseType == courseType)&&(identical(other.credits, credits) || other.credits == credits)&&(identical(other.grade, grade) || other.grade == grade)&&(identical(other.examMonth, examMonth) || other.examMonth == examMonth)&&(identical(other.resultDeclared, resultDeclared) || other.resultDeclared == resultDeclared));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,courseCode,courseTitle,courseType,credits,grade,examMonth,resultDeclared);

@override
String toString() {
  return 'GradeHistoryAttempt(courseCode: $courseCode, courseTitle: $courseTitle, courseType: $courseType, credits: $credits, grade: $grade, examMonth: $examMonth, resultDeclared: $resultDeclared)';
}


}

/// @nodoc
abstract mixin class _$GradeHistoryAttemptCopyWith<$Res> implements $GradeHistoryAttemptCopyWith<$Res> {
  factory _$GradeHistoryAttemptCopyWith(_GradeHistoryAttempt value, $Res Function(_GradeHistoryAttempt) _then) = __$GradeHistoryAttemptCopyWithImpl;
@override @useResult
$Res call({
 String courseCode, String courseTitle, String courseType, String credits, String grade, String examMonth, String resultDeclared
});




}
/// @nodoc
class __$GradeHistoryAttemptCopyWithImpl<$Res>
    implements _$GradeHistoryAttemptCopyWith<$Res> {
  __$GradeHistoryAttemptCopyWithImpl(this._self, this._then);

  final _GradeHistoryAttempt _self;
  final $Res Function(_GradeHistoryAttempt) _then;

/// Create a copy of GradeHistoryAttempt
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? courseCode = null,Object? courseTitle = null,Object? courseType = null,Object? credits = null,Object? grade = null,Object? examMonth = null,Object? resultDeclared = null,}) {
  return _then(_GradeHistoryAttempt(
courseCode: null == courseCode ? _self.courseCode : courseCode // ignore: cast_nullable_to_non_nullable
as String,courseTitle: null == courseTitle ? _self.courseTitle : courseTitle // ignore: cast_nullable_to_non_nullable
as String,courseType: null == courseType ? _self.courseType : courseType // ignore: cast_nullable_to_non_nullable
as String,credits: null == credits ? _self.credits : credits // ignore: cast_nullable_to_non_nullable
as String,grade: null == grade ? _self.grade : grade // ignore: cast_nullable_to_non_nullable
as String,examMonth: null == examMonth ? _self.examMonth : examMonth // ignore: cast_nullable_to_non_nullable
as String,resultDeclared: null == resultDeclared ? _self.resultDeclared : resultDeclared // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}


/// @nodoc
mixin _$GradeHistoryCgpa {

 String get creditsRegistered; String get creditsEarned; String get cgpa; String get sGrades; String get aGrades; String get bGrades; String get cGrades; String get dGrades; String get eGrades; String get fGrades; String get nGrades;
/// Create a copy of GradeHistoryCgpa
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GradeHistoryCgpaCopyWith<GradeHistoryCgpa> get copyWith => _$GradeHistoryCgpaCopyWithImpl<GradeHistoryCgpa>(this as GradeHistoryCgpa, _$identity);

  /// Serializes this GradeHistoryCgpa to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is GradeHistoryCgpa&&(identical(other.creditsRegistered, creditsRegistered) || other.creditsRegistered == creditsRegistered)&&(identical(other.creditsEarned, creditsEarned) || other.creditsEarned == creditsEarned)&&(identical(other.cgpa, cgpa) || other.cgpa == cgpa)&&(identical(other.sGrades, sGrades) || other.sGrades == sGrades)&&(identical(other.aGrades, aGrades) || other.aGrades == aGrades)&&(identical(other.bGrades, bGrades) || other.bGrades == bGrades)&&(identical(other.cGrades, cGrades) || other.cGrades == cGrades)&&(identical(other.dGrades, dGrades) || other.dGrades == dGrades)&&(identical(other.eGrades, eGrades) || other.eGrades == eGrades)&&(identical(other.fGrades, fGrades) || other.fGrades == fGrades)&&(identical(other.nGrades, nGrades) || other.nGrades == nGrades));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,creditsRegistered,creditsEarned,cgpa,sGrades,aGrades,bGrades,cGrades,dGrades,eGrades,fGrades,nGrades);

@override
String toString() {
  return 'GradeHistoryCgpa(creditsRegistered: $creditsRegistered, creditsEarned: $creditsEarned, cgpa: $cgpa, sGrades: $sGrades, aGrades: $aGrades, bGrades: $bGrades, cGrades: $cGrades, dGrades: $dGrades, eGrades: $eGrades, fGrades: $fGrades, nGrades: $nGrades)';
}


}

/// @nodoc
abstract mixin class $GradeHistoryCgpaCopyWith<$Res>  {
  factory $GradeHistoryCgpaCopyWith(GradeHistoryCgpa value, $Res Function(GradeHistoryCgpa) _then) = _$GradeHistoryCgpaCopyWithImpl;
@useResult
$Res call({
 String creditsRegistered, String creditsEarned, String cgpa, String sGrades, String aGrades, String bGrades, String cGrades, String dGrades, String eGrades, String fGrades, String nGrades
});




}
/// @nodoc
class _$GradeHistoryCgpaCopyWithImpl<$Res>
    implements $GradeHistoryCgpaCopyWith<$Res> {
  _$GradeHistoryCgpaCopyWithImpl(this._self, this._then);

  final GradeHistoryCgpa _self;
  final $Res Function(GradeHistoryCgpa) _then;

/// Create a copy of GradeHistoryCgpa
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? creditsRegistered = null,Object? creditsEarned = null,Object? cgpa = null,Object? sGrades = null,Object? aGrades = null,Object? bGrades = null,Object? cGrades = null,Object? dGrades = null,Object? eGrades = null,Object? fGrades = null,Object? nGrades = null,}) {
  return _then(_self.copyWith(
creditsRegistered: null == creditsRegistered ? _self.creditsRegistered : creditsRegistered // ignore: cast_nullable_to_non_nullable
as String,creditsEarned: null == creditsEarned ? _self.creditsEarned : creditsEarned // ignore: cast_nullable_to_non_nullable
as String,cgpa: null == cgpa ? _self.cgpa : cgpa // ignore: cast_nullable_to_non_nullable
as String,sGrades: null == sGrades ? _self.sGrades : sGrades // ignore: cast_nullable_to_non_nullable
as String,aGrades: null == aGrades ? _self.aGrades : aGrades // ignore: cast_nullable_to_non_nullable
as String,bGrades: null == bGrades ? _self.bGrades : bGrades // ignore: cast_nullable_to_non_nullable
as String,cGrades: null == cGrades ? _self.cGrades : cGrades // ignore: cast_nullable_to_non_nullable
as String,dGrades: null == dGrades ? _self.dGrades : dGrades // ignore: cast_nullable_to_non_nullable
as String,eGrades: null == eGrades ? _self.eGrades : eGrades // ignore: cast_nullable_to_non_nullable
as String,fGrades: null == fGrades ? _self.fGrades : fGrades // ignore: cast_nullable_to_non_nullable
as String,nGrades: null == nGrades ? _self.nGrades : nGrades // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// Adds pattern-matching-related methods to [GradeHistoryCgpa].
extension GradeHistoryCgpaPatterns on GradeHistoryCgpa {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _GradeHistoryCgpa value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _GradeHistoryCgpa() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _GradeHistoryCgpa value)  $default,){
final _that = this;
switch (_that) {
case _GradeHistoryCgpa():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _GradeHistoryCgpa value)?  $default,){
final _that = this;
switch (_that) {
case _GradeHistoryCgpa() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String creditsRegistered,  String creditsEarned,  String cgpa,  String sGrades,  String aGrades,  String bGrades,  String cGrades,  String dGrades,  String eGrades,  String fGrades,  String nGrades)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _GradeHistoryCgpa() when $default != null:
return $default(_that.creditsRegistered,_that.creditsEarned,_that.cgpa,_that.sGrades,_that.aGrades,_that.bGrades,_that.cGrades,_that.dGrades,_that.eGrades,_that.fGrades,_that.nGrades);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String creditsRegistered,  String creditsEarned,  String cgpa,  String sGrades,  String aGrades,  String bGrades,  String cGrades,  String dGrades,  String eGrades,  String fGrades,  String nGrades)  $default,) {final _that = this;
switch (_that) {
case _GradeHistoryCgpa():
return $default(_that.creditsRegistered,_that.creditsEarned,_that.cgpa,_that.sGrades,_that.aGrades,_that.bGrades,_that.cGrades,_that.dGrades,_that.eGrades,_that.fGrades,_that.nGrades);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String creditsRegistered,  String creditsEarned,  String cgpa,  String sGrades,  String aGrades,  String bGrades,  String cGrades,  String dGrades,  String eGrades,  String fGrades,  String nGrades)?  $default,) {final _that = this;
switch (_that) {
case _GradeHistoryCgpa() when $default != null:
return $default(_that.creditsRegistered,_that.creditsEarned,_that.cgpa,_that.sGrades,_that.aGrades,_that.bGrades,_that.cGrades,_that.dGrades,_that.eGrades,_that.fGrades,_that.nGrades);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _GradeHistoryCgpa implements GradeHistoryCgpa {
  const _GradeHistoryCgpa({required this.creditsRegistered, required this.creditsEarned, required this.cgpa, required this.sGrades, required this.aGrades, required this.bGrades, required this.cGrades, required this.dGrades, required this.eGrades, required this.fGrades, required this.nGrades});
  factory _GradeHistoryCgpa.fromJson(Map<String, dynamic> json) => _$GradeHistoryCgpaFromJson(json);

@override final  String creditsRegistered;
@override final  String creditsEarned;
@override final  String cgpa;
@override final  String sGrades;
@override final  String aGrades;
@override final  String bGrades;
@override final  String cGrades;
@override final  String dGrades;
@override final  String eGrades;
@override final  String fGrades;
@override final  String nGrades;

/// Create a copy of GradeHistoryCgpa
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$GradeHistoryCgpaCopyWith<_GradeHistoryCgpa> get copyWith => __$GradeHistoryCgpaCopyWithImpl<_GradeHistoryCgpa>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$GradeHistoryCgpaToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _GradeHistoryCgpa&&(identical(other.creditsRegistered, creditsRegistered) || other.creditsRegistered == creditsRegistered)&&(identical(other.creditsEarned, creditsEarned) || other.creditsEarned == creditsEarned)&&(identical(other.cgpa, cgpa) || other.cgpa == cgpa)&&(identical(other.sGrades, sGrades) || other.sGrades == sGrades)&&(identical(other.aGrades, aGrades) || other.aGrades == aGrades)&&(identical(other.bGrades, bGrades) || other.bGrades == bGrades)&&(identical(other.cGrades, cGrades) || other.cGrades == cGrades)&&(identical(other.dGrades, dGrades) || other.dGrades == dGrades)&&(identical(other.eGrades, eGrades) || other.eGrades == eGrades)&&(identical(other.fGrades, fGrades) || other.fGrades == fGrades)&&(identical(other.nGrades, nGrades) || other.nGrades == nGrades));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,creditsRegistered,creditsEarned,cgpa,sGrades,aGrades,bGrades,cGrades,dGrades,eGrades,fGrades,nGrades);

@override
String toString() {
  return 'GradeHistoryCgpa(creditsRegistered: $creditsRegistered, creditsEarned: $creditsEarned, cgpa: $cgpa, sGrades: $sGrades, aGrades: $aGrades, bGrades: $bGrades, cGrades: $cGrades, dGrades: $dGrades, eGrades: $eGrades, fGrades: $fGrades, nGrades: $nGrades)';
}


}

/// @nodoc
abstract mixin class _$GradeHistoryCgpaCopyWith<$Res> implements $GradeHistoryCgpaCopyWith<$Res> {
  factory _$GradeHistoryCgpaCopyWith(_GradeHistoryCgpa value, $Res Function(_GradeHistoryCgpa) _then) = __$GradeHistoryCgpaCopyWithImpl;
@override @useResult
$Res call({
 String creditsRegistered, String creditsEarned, String cgpa, String sGrades, String aGrades, String bGrades, String cGrades, String dGrades, String eGrades, String fGrades, String nGrades
});




}
/// @nodoc
class __$GradeHistoryCgpaCopyWithImpl<$Res>
    implements _$GradeHistoryCgpaCopyWith<$Res> {
  __$GradeHistoryCgpaCopyWithImpl(this._self, this._then);

  final _GradeHistoryCgpa _self;
  final $Res Function(_GradeHistoryCgpa) _then;

/// Create a copy of GradeHistoryCgpa
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? creditsRegistered = null,Object? creditsEarned = null,Object? cgpa = null,Object? sGrades = null,Object? aGrades = null,Object? bGrades = null,Object? cGrades = null,Object? dGrades = null,Object? eGrades = null,Object? fGrades = null,Object? nGrades = null,}) {
  return _then(_GradeHistoryCgpa(
creditsRegistered: null == creditsRegistered ? _self.creditsRegistered : creditsRegistered // ignore: cast_nullable_to_non_nullable
as String,creditsEarned: null == creditsEarned ? _self.creditsEarned : creditsEarned // ignore: cast_nullable_to_non_nullable
as String,cgpa: null == cgpa ? _self.cgpa : cgpa // ignore: cast_nullable_to_non_nullable
as String,sGrades: null == sGrades ? _self.sGrades : sGrades // ignore: cast_nullable_to_non_nullable
as String,aGrades: null == aGrades ? _self.aGrades : aGrades // ignore: cast_nullable_to_non_nullable
as String,bGrades: null == bGrades ? _self.bGrades : bGrades // ignore: cast_nullable_to_non_nullable
as String,cGrades: null == cGrades ? _self.cGrades : cGrades // ignore: cast_nullable_to_non_nullable
as String,dGrades: null == dGrades ? _self.dGrades : dGrades // ignore: cast_nullable_to_non_nullable
as String,eGrades: null == eGrades ? _self.eGrades : eGrades // ignore: cast_nullable_to_non_nullable
as String,fGrades: null == fGrades ? _self.fGrades : fGrades // ignore: cast_nullable_to_non_nullable
as String,nGrades: null == nGrades ? _self.nGrades : nGrades // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}


/// @nodoc
mixin _$GradeHistoryData {

 GradeHistoryStudentInfo get student; List<GradeHistoryRecord> get records; GradeHistoryCgpa get cgpa; BigInt get updateTime;
/// Create a copy of GradeHistoryData
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GradeHistoryDataCopyWith<GradeHistoryData> get copyWith => _$GradeHistoryDataCopyWithImpl<GradeHistoryData>(this as GradeHistoryData, _$identity);

  /// Serializes this GradeHistoryData to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is GradeHistoryData&&(identical(other.student, student) || other.student == student)&&const DeepCollectionEquality().equals(other.records, records)&&(identical(other.cgpa, cgpa) || other.cgpa == cgpa)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,student,const DeepCollectionEquality().hash(records),cgpa,updateTime);

@override
String toString() {
  return 'GradeHistoryData(student: $student, records: $records, cgpa: $cgpa, updateTime: $updateTime)';
}


}

/// @nodoc
abstract mixin class $GradeHistoryDataCopyWith<$Res>  {
  factory $GradeHistoryDataCopyWith(GradeHistoryData value, $Res Function(GradeHistoryData) _then) = _$GradeHistoryDataCopyWithImpl;
@useResult
$Res call({
 GradeHistoryStudentInfo student, List<GradeHistoryRecord> records, GradeHistoryCgpa cgpa, BigInt updateTime
});


$GradeHistoryStudentInfoCopyWith<$Res> get student;$GradeHistoryCgpaCopyWith<$Res> get cgpa;

}
/// @nodoc
class _$GradeHistoryDataCopyWithImpl<$Res>
    implements $GradeHistoryDataCopyWith<$Res> {
  _$GradeHistoryDataCopyWithImpl(this._self, this._then);

  final GradeHistoryData _self;
  final $Res Function(GradeHistoryData) _then;

/// Create a copy of GradeHistoryData
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? student = null,Object? records = null,Object? cgpa = null,Object? updateTime = null,}) {
  return _then(_self.copyWith(
student: null == student ? _self.student : student // ignore: cast_nullable_to_non_nullable
as GradeHistoryStudentInfo,records: null == records ? _self.records : records // ignore: cast_nullable_to_non_nullable
as List<GradeHistoryRecord>,cgpa: null == cgpa ? _self.cgpa : cgpa // ignore: cast_nullable_to_non_nullable
as GradeHistoryCgpa,updateTime: null == updateTime ? _self.updateTime : updateTime // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}
/// Create a copy of GradeHistoryData
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$GradeHistoryStudentInfoCopyWith<$Res> get student {
  
  return $GradeHistoryStudentInfoCopyWith<$Res>(_self.student, (value) {
    return _then(_self.copyWith(student: value));
  });
}/// Create a copy of GradeHistoryData
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$GradeHistoryCgpaCopyWith<$Res> get cgpa {
  
  return $GradeHistoryCgpaCopyWith<$Res>(_self.cgpa, (value) {
    return _then(_self.copyWith(cgpa: value));
  });
}
}


/// Adds pattern-matching-related methods to [GradeHistoryData].
extension GradeHistoryDataPatterns on GradeHistoryData {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _GradeHistoryData value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _GradeHistoryData() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _GradeHistoryData value)  $default,){
final _that = this;
switch (_that) {
case _GradeHistoryData():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _GradeHistoryData value)?  $default,){
final _that = this;
switch (_that) {
case _GradeHistoryData() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( GradeHistoryStudentInfo student,  List<GradeHistoryRecord> records,  GradeHistoryCgpa cgpa,  BigInt updateTime)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _GradeHistoryData() when $default != null:
return $default(_that.student,_that.records,_that.cgpa,_that.updateTime);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( GradeHistoryStudentInfo student,  List<GradeHistoryRecord> records,  GradeHistoryCgpa cgpa,  BigInt updateTime)  $default,) {final _that = this;
switch (_that) {
case _GradeHistoryData():
return $default(_that.student,_that.records,_that.cgpa,_that.updateTime);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( GradeHistoryStudentInfo student,  List<GradeHistoryRecord> records,  GradeHistoryCgpa cgpa,  BigInt updateTime)?  $default,) {final _that = this;
switch (_that) {
case _GradeHistoryData() when $default != null:
return $default(_that.student,_that.records,_that.cgpa,_that.updateTime);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _GradeHistoryData implements GradeHistoryData {
  const _GradeHistoryData({required this.student, required final  List<GradeHistoryRecord> records, required this.cgpa, required this.updateTime}): _records = records;
  factory _GradeHistoryData.fromJson(Map<String, dynamic> json) => _$GradeHistoryDataFromJson(json);

@override final  GradeHistoryStudentInfo student;
 final  List<GradeHistoryRecord> _records;
@override List<GradeHistoryRecord> get records {
  if (_records is EqualUnmodifiableListView) return _records;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_records);
}

@override final  GradeHistoryCgpa cgpa;
@override final  BigInt updateTime;

/// Create a copy of GradeHistoryData
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$GradeHistoryDataCopyWith<_GradeHistoryData> get copyWith => __$GradeHistoryDataCopyWithImpl<_GradeHistoryData>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$GradeHistoryDataToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _GradeHistoryData&&(identical(other.student, student) || other.student == student)&&const DeepCollectionEquality().equals(other._records, _records)&&(identical(other.cgpa, cgpa) || other.cgpa == cgpa)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,student,const DeepCollectionEquality().hash(_records),cgpa,updateTime);

@override
String toString() {
  return 'GradeHistoryData(student: $student, records: $records, cgpa: $cgpa, updateTime: $updateTime)';
}


}

/// @nodoc
abstract mixin class _$GradeHistoryDataCopyWith<$Res> implements $GradeHistoryDataCopyWith<$Res> {
  factory _$GradeHistoryDataCopyWith(_GradeHistoryData value, $Res Function(_GradeHistoryData) _then) = __$GradeHistoryDataCopyWithImpl;
@override @useResult
$Res call({
 GradeHistoryStudentInfo student, List<GradeHistoryRecord> records, GradeHistoryCgpa cgpa, BigInt updateTime
});


@override $GradeHistoryStudentInfoCopyWith<$Res> get student;@override $GradeHistoryCgpaCopyWith<$Res> get cgpa;

}
/// @nodoc
class __$GradeHistoryDataCopyWithImpl<$Res>
    implements _$GradeHistoryDataCopyWith<$Res> {
  __$GradeHistoryDataCopyWithImpl(this._self, this._then);

  final _GradeHistoryData _self;
  final $Res Function(_GradeHistoryData) _then;

/// Create a copy of GradeHistoryData
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? student = null,Object? records = null,Object? cgpa = null,Object? updateTime = null,}) {
  return _then(_GradeHistoryData(
student: null == student ? _self.student : student // ignore: cast_nullable_to_non_nullable
as GradeHistoryStudentInfo,records: null == records ? _self._records : records // ignore: cast_nullable_to_non_nullable
as List<GradeHistoryRecord>,cgpa: null == cgpa ? _self.cgpa : cgpa // ignore: cast_nullable_to_non_nullable
as GradeHistoryCgpa,updateTime: null == updateTime ? _self.updateTime : updateTime // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}

/// Create a copy of GradeHistoryData
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$GradeHistoryStudentInfoCopyWith<$Res> get student {
  
  return $GradeHistoryStudentInfoCopyWith<$Res>(_self.student, (value) {
    return _then(_self.copyWith(student: value));
  });
}/// Create a copy of GradeHistoryData
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$GradeHistoryCgpaCopyWith<$Res> get cgpa {
  
  return $GradeHistoryCgpaCopyWith<$Res>(_self.cgpa, (value) {
    return _then(_self.copyWith(cgpa: value));
  });
}
}


/// @nodoc
mixin _$GradeHistoryRecord {

 String get serial; String get courseCode; String get courseTitle; String get courseType; String get credits; String get grade; String get examMonth; String get resultDeclared; String get courseDistribution; List<GradeHistoryAttempt> get attempts;
/// Create a copy of GradeHistoryRecord
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GradeHistoryRecordCopyWith<GradeHistoryRecord> get copyWith => _$GradeHistoryRecordCopyWithImpl<GradeHistoryRecord>(this as GradeHistoryRecord, _$identity);

  /// Serializes this GradeHistoryRecord to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is GradeHistoryRecord&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.courseCode, courseCode) || other.courseCode == courseCode)&&(identical(other.courseTitle, courseTitle) || other.courseTitle == courseTitle)&&(identical(other.courseType, courseType) || other.courseType == courseType)&&(identical(other.credits, credits) || other.credits == credits)&&(identical(other.grade, grade) || other.grade == grade)&&(identical(other.examMonth, examMonth) || other.examMonth == examMonth)&&(identical(other.resultDeclared, resultDeclared) || other.resultDeclared == resultDeclared)&&(identical(other.courseDistribution, courseDistribution) || other.courseDistribution == courseDistribution)&&const DeepCollectionEquality().equals(other.attempts, attempts));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,serial,courseCode,courseTitle,courseType,credits,grade,examMonth,resultDeclared,courseDistribution,const DeepCollectionEquality().hash(attempts));

@override
String toString() {
  return 'GradeHistoryRecord(serial: $serial, courseCode: $courseCode, courseTitle: $courseTitle, courseType: $courseType, credits: $credits, grade: $grade, examMonth: $examMonth, resultDeclared: $resultDeclared, courseDistribution: $courseDistribution, attempts: $attempts)';
}


}

/// @nodoc
abstract mixin class $GradeHistoryRecordCopyWith<$Res>  {
  factory $GradeHistoryRecordCopyWith(GradeHistoryRecord value, $Res Function(GradeHistoryRecord) _then) = _$GradeHistoryRecordCopyWithImpl;
@useResult
$Res call({
 String serial, String courseCode, String courseTitle, String courseType, String credits, String grade, String examMonth, String resultDeclared, String courseDistribution, List<GradeHistoryAttempt> attempts
});




}
/// @nodoc
class _$GradeHistoryRecordCopyWithImpl<$Res>
    implements $GradeHistoryRecordCopyWith<$Res> {
  _$GradeHistoryRecordCopyWithImpl(this._self, this._then);

  final GradeHistoryRecord _self;
  final $Res Function(GradeHistoryRecord) _then;

/// Create a copy of GradeHistoryRecord
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? serial = null,Object? courseCode = null,Object? courseTitle = null,Object? courseType = null,Object? credits = null,Object? grade = null,Object? examMonth = null,Object? resultDeclared = null,Object? courseDistribution = null,Object? attempts = null,}) {
  return _then(_self.copyWith(
serial: null == serial ? _self.serial : serial // ignore: cast_nullable_to_non_nullable
as String,courseCode: null == courseCode ? _self.courseCode : courseCode // ignore: cast_nullable_to_non_nullable
as String,courseTitle: null == courseTitle ? _self.courseTitle : courseTitle // ignore: cast_nullable_to_non_nullable
as String,courseType: null == courseType ? _self.courseType : courseType // ignore: cast_nullable_to_non_nullable
as String,credits: null == credits ? _self.credits : credits // ignore: cast_nullable_to_non_nullable
as String,grade: null == grade ? _self.grade : grade // ignore: cast_nullable_to_non_nullable
as String,examMonth: null == examMonth ? _self.examMonth : examMonth // ignore: cast_nullable_to_non_nullable
as String,resultDeclared: null == resultDeclared ? _self.resultDeclared : resultDeclared // ignore: cast_nullable_to_non_nullable
as String,courseDistribution: null == courseDistribution ? _self.courseDistribution : courseDistribution // ignore: cast_nullable_to_non_nullable
as String,attempts: null == attempts ? _self.attempts : attempts // ignore: cast_nullable_to_non_nullable
as List<GradeHistoryAttempt>,
  ));
}

}


/// Adds pattern-matching-related methods to [GradeHistoryRecord].
extension GradeHistoryRecordPatterns on GradeHistoryRecord {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _GradeHistoryRecord value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _GradeHistoryRecord() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _GradeHistoryRecord value)  $default,){
final _that = this;
switch (_that) {
case _GradeHistoryRecord():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _GradeHistoryRecord value)?  $default,){
final _that = this;
switch (_that) {
case _GradeHistoryRecord() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String serial,  String courseCode,  String courseTitle,  String courseType,  String credits,  String grade,  String examMonth,  String resultDeclared,  String courseDistribution,  List<GradeHistoryAttempt> attempts)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _GradeHistoryRecord() when $default != null:
return $default(_that.serial,_that.courseCode,_that.courseTitle,_that.courseType,_that.credits,_that.grade,_that.examMonth,_that.resultDeclared,_that.courseDistribution,_that.attempts);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String serial,  String courseCode,  String courseTitle,  String courseType,  String credits,  String grade,  String examMonth,  String resultDeclared,  String courseDistribution,  List<GradeHistoryAttempt> attempts)  $default,) {final _that = this;
switch (_that) {
case _GradeHistoryRecord():
return $default(_that.serial,_that.courseCode,_that.courseTitle,_that.courseType,_that.credits,_that.grade,_that.examMonth,_that.resultDeclared,_that.courseDistribution,_that.attempts);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String serial,  String courseCode,  String courseTitle,  String courseType,  String credits,  String grade,  String examMonth,  String resultDeclared,  String courseDistribution,  List<GradeHistoryAttempt> attempts)?  $default,) {final _that = this;
switch (_that) {
case _GradeHistoryRecord() when $default != null:
return $default(_that.serial,_that.courseCode,_that.courseTitle,_that.courseType,_that.credits,_that.grade,_that.examMonth,_that.resultDeclared,_that.courseDistribution,_that.attempts);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _GradeHistoryRecord implements GradeHistoryRecord {
  const _GradeHistoryRecord({required this.serial, required this.courseCode, required this.courseTitle, required this.courseType, required this.credits, required this.grade, required this.examMonth, required this.resultDeclared, required this.courseDistribution, required final  List<GradeHistoryAttempt> attempts}): _attempts = attempts;
  factory _GradeHistoryRecord.fromJson(Map<String, dynamic> json) => _$GradeHistoryRecordFromJson(json);

@override final  String serial;
@override final  String courseCode;
@override final  String courseTitle;
@override final  String courseType;
@override final  String credits;
@override final  String grade;
@override final  String examMonth;
@override final  String resultDeclared;
@override final  String courseDistribution;
 final  List<GradeHistoryAttempt> _attempts;
@override List<GradeHistoryAttempt> get attempts {
  if (_attempts is EqualUnmodifiableListView) return _attempts;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_attempts);
}


/// Create a copy of GradeHistoryRecord
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$GradeHistoryRecordCopyWith<_GradeHistoryRecord> get copyWith => __$GradeHistoryRecordCopyWithImpl<_GradeHistoryRecord>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$GradeHistoryRecordToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _GradeHistoryRecord&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.courseCode, courseCode) || other.courseCode == courseCode)&&(identical(other.courseTitle, courseTitle) || other.courseTitle == courseTitle)&&(identical(other.courseType, courseType) || other.courseType == courseType)&&(identical(other.credits, credits) || other.credits == credits)&&(identical(other.grade, grade) || other.grade == grade)&&(identical(other.examMonth, examMonth) || other.examMonth == examMonth)&&(identical(other.resultDeclared, resultDeclared) || other.resultDeclared == resultDeclared)&&(identical(other.courseDistribution, courseDistribution) || other.courseDistribution == courseDistribution)&&const DeepCollectionEquality().equals(other._attempts, _attempts));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,serial,courseCode,courseTitle,courseType,credits,grade,examMonth,resultDeclared,courseDistribution,const DeepCollectionEquality().hash(_attempts));

@override
String toString() {
  return 'GradeHistoryRecord(serial: $serial, courseCode: $courseCode, courseTitle: $courseTitle, courseType: $courseType, credits: $credits, grade: $grade, examMonth: $examMonth, resultDeclared: $resultDeclared, courseDistribution: $courseDistribution, attempts: $attempts)';
}


}

/// @nodoc
abstract mixin class _$GradeHistoryRecordCopyWith<$Res> implements $GradeHistoryRecordCopyWith<$Res> {
  factory _$GradeHistoryRecordCopyWith(_GradeHistoryRecord value, $Res Function(_GradeHistoryRecord) _then) = __$GradeHistoryRecordCopyWithImpl;
@override @useResult
$Res call({
 String serial, String courseCode, String courseTitle, String courseType, String credits, String grade, String examMonth, String resultDeclared, String courseDistribution, List<GradeHistoryAttempt> attempts
});




}
/// @nodoc
class __$GradeHistoryRecordCopyWithImpl<$Res>
    implements _$GradeHistoryRecordCopyWith<$Res> {
  __$GradeHistoryRecordCopyWithImpl(this._self, this._then);

  final _GradeHistoryRecord _self;
  final $Res Function(_GradeHistoryRecord) _then;

/// Create a copy of GradeHistoryRecord
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? serial = null,Object? courseCode = null,Object? courseTitle = null,Object? courseType = null,Object? credits = null,Object? grade = null,Object? examMonth = null,Object? resultDeclared = null,Object? courseDistribution = null,Object? attempts = null,}) {
  return _then(_GradeHistoryRecord(
serial: null == serial ? _self.serial : serial // ignore: cast_nullable_to_non_nullable
as String,courseCode: null == courseCode ? _self.courseCode : courseCode // ignore: cast_nullable_to_non_nullable
as String,courseTitle: null == courseTitle ? _self.courseTitle : courseTitle // ignore: cast_nullable_to_non_nullable
as String,courseType: null == courseType ? _self.courseType : courseType // ignore: cast_nullable_to_non_nullable
as String,credits: null == credits ? _self.credits : credits // ignore: cast_nullable_to_non_nullable
as String,grade: null == grade ? _self.grade : grade // ignore: cast_nullable_to_non_nullable
as String,examMonth: null == examMonth ? _self.examMonth : examMonth // ignore: cast_nullable_to_non_nullable
as String,resultDeclared: null == resultDeclared ? _self.resultDeclared : resultDeclared // ignore: cast_nullable_to_non_nullable
as String,courseDistribution: null == courseDistribution ? _self.courseDistribution : courseDistribution // ignore: cast_nullable_to_non_nullable
as String,attempts: null == attempts ? _self._attempts : attempts // ignore: cast_nullable_to_non_nullable
as List<GradeHistoryAttempt>,
  ));
}


}


/// @nodoc
mixin _$GradeHistoryStudentInfo {

 String get regNo; String get name; String get programmeBranch; String get programmeMode; String get studySystem; String get gender; String get yearJoined; String get eduStatus; String get school; String get campus;
/// Create a copy of GradeHistoryStudentInfo
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GradeHistoryStudentInfoCopyWith<GradeHistoryStudentInfo> get copyWith => _$GradeHistoryStudentInfoCopyWithImpl<GradeHistoryStudentInfo>(this as GradeHistoryStudentInfo, _$identity);

  /// Serializes this GradeHistoryStudentInfo to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is GradeHistoryStudentInfo&&(identical(other.regNo, regNo) || other.regNo == regNo)&&(identical(other.name, name) || other.name == name)&&(identical(other.programmeBranch, programmeBranch) || other.programmeBranch == programmeBranch)&&(identical(other.programmeMode, programmeMode) || other.programmeMode == programmeMode)&&(identical(other.studySystem, studySystem) || other.studySystem == studySystem)&&(identical(other.gender, gender) || other.gender == gender)&&(identical(other.yearJoined, yearJoined) || other.yearJoined == yearJoined)&&(identical(other.eduStatus, eduStatus) || other.eduStatus == eduStatus)&&(identical(other.school, school) || other.school == school)&&(identical(other.campus, campus) || other.campus == campus));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,regNo,name,programmeBranch,programmeMode,studySystem,gender,yearJoined,eduStatus,school,campus);

@override
String toString() {
  return 'GradeHistoryStudentInfo(regNo: $regNo, name: $name, programmeBranch: $programmeBranch, programmeMode: $programmeMode, studySystem: $studySystem, gender: $gender, yearJoined: $yearJoined, eduStatus: $eduStatus, school: $school, campus: $campus)';
}


}

/// @nodoc
abstract mixin class $GradeHistoryStudentInfoCopyWith<$Res>  {
  factory $GradeHistoryStudentInfoCopyWith(GradeHistoryStudentInfo value, $Res Function(GradeHistoryStudentInfo) _then) = _$GradeHistoryStudentInfoCopyWithImpl;
@useResult
$Res call({
 String regNo, String name, String programmeBranch, String programmeMode, String studySystem, String gender, String yearJoined, String eduStatus, String school, String campus
});




}
/// @nodoc
class _$GradeHistoryStudentInfoCopyWithImpl<$Res>
    implements $GradeHistoryStudentInfoCopyWith<$Res> {
  _$GradeHistoryStudentInfoCopyWithImpl(this._self, this._then);

  final GradeHistoryStudentInfo _self;
  final $Res Function(GradeHistoryStudentInfo) _then;

/// Create a copy of GradeHistoryStudentInfo
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? regNo = null,Object? name = null,Object? programmeBranch = null,Object? programmeMode = null,Object? studySystem = null,Object? gender = null,Object? yearJoined = null,Object? eduStatus = null,Object? school = null,Object? campus = null,}) {
  return _then(_self.copyWith(
regNo: null == regNo ? _self.regNo : regNo // ignore: cast_nullable_to_non_nullable
as String,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,programmeBranch: null == programmeBranch ? _self.programmeBranch : programmeBranch // ignore: cast_nullable_to_non_nullable
as String,programmeMode: null == programmeMode ? _self.programmeMode : programmeMode // ignore: cast_nullable_to_non_nullable
as String,studySystem: null == studySystem ? _self.studySystem : studySystem // ignore: cast_nullable_to_non_nullable
as String,gender: null == gender ? _self.gender : gender // ignore: cast_nullable_to_non_nullable
as String,yearJoined: null == yearJoined ? _self.yearJoined : yearJoined // ignore: cast_nullable_to_non_nullable
as String,eduStatus: null == eduStatus ? _self.eduStatus : eduStatus // ignore: cast_nullable_to_non_nullable
as String,school: null == school ? _self.school : school // ignore: cast_nullable_to_non_nullable
as String,campus: null == campus ? _self.campus : campus // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// Adds pattern-matching-related methods to [GradeHistoryStudentInfo].
extension GradeHistoryStudentInfoPatterns on GradeHistoryStudentInfo {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _GradeHistoryStudentInfo value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _GradeHistoryStudentInfo() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _GradeHistoryStudentInfo value)  $default,){
final _that = this;
switch (_that) {
case _GradeHistoryStudentInfo():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _GradeHistoryStudentInfo value)?  $default,){
final _that = this;
switch (_that) {
case _GradeHistoryStudentInfo() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String regNo,  String name,  String programmeBranch,  String programmeMode,  String studySystem,  String gender,  String yearJoined,  String eduStatus,  String school,  String campus)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _GradeHistoryStudentInfo() when $default != null:
return $default(_that.regNo,_that.name,_that.programmeBranch,_that.programmeMode,_that.studySystem,_that.gender,_that.yearJoined,_that.eduStatus,_that.school,_that.campus);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String regNo,  String name,  String programmeBranch,  String programmeMode,  String studySystem,  String gender,  String yearJoined,  String eduStatus,  String school,  String campus)  $default,) {final _that = this;
switch (_that) {
case _GradeHistoryStudentInfo():
return $default(_that.regNo,_that.name,_that.programmeBranch,_that.programmeMode,_that.studySystem,_that.gender,_that.yearJoined,_that.eduStatus,_that.school,_that.campus);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String regNo,  String name,  String programmeBranch,  String programmeMode,  String studySystem,  String gender,  String yearJoined,  String eduStatus,  String school,  String campus)?  $default,) {final _that = this;
switch (_that) {
case _GradeHistoryStudentInfo() when $default != null:
return $default(_that.regNo,_that.name,_that.programmeBranch,_that.programmeMode,_that.studySystem,_that.gender,_that.yearJoined,_that.eduStatus,_that.school,_that.campus);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _GradeHistoryStudentInfo implements GradeHistoryStudentInfo {
  const _GradeHistoryStudentInfo({required this.regNo, required this.name, required this.programmeBranch, required this.programmeMode, required this.studySystem, required this.gender, required this.yearJoined, required this.eduStatus, required this.school, required this.campus});
  factory _GradeHistoryStudentInfo.fromJson(Map<String, dynamic> json) => _$GradeHistoryStudentInfoFromJson(json);

@override final  String regNo;
@override final  String name;
@override final  String programmeBranch;
@override final  String programmeMode;
@override final  String studySystem;
@override final  String gender;
@override final  String yearJoined;
@override final  String eduStatus;
@override final  String school;
@override final  String campus;

/// Create a copy of GradeHistoryStudentInfo
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$GradeHistoryStudentInfoCopyWith<_GradeHistoryStudentInfo> get copyWith => __$GradeHistoryStudentInfoCopyWithImpl<_GradeHistoryStudentInfo>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$GradeHistoryStudentInfoToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _GradeHistoryStudentInfo&&(identical(other.regNo, regNo) || other.regNo == regNo)&&(identical(other.name, name) || other.name == name)&&(identical(other.programmeBranch, programmeBranch) || other.programmeBranch == programmeBranch)&&(identical(other.programmeMode, programmeMode) || other.programmeMode == programmeMode)&&(identical(other.studySystem, studySystem) || other.studySystem == studySystem)&&(identical(other.gender, gender) || other.gender == gender)&&(identical(other.yearJoined, yearJoined) || other.yearJoined == yearJoined)&&(identical(other.eduStatus, eduStatus) || other.eduStatus == eduStatus)&&(identical(other.school, school) || other.school == school)&&(identical(other.campus, campus) || other.campus == campus));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,regNo,name,programmeBranch,programmeMode,studySystem,gender,yearJoined,eduStatus,school,campus);

@override
String toString() {
  return 'GradeHistoryStudentInfo(regNo: $regNo, name: $name, programmeBranch: $programmeBranch, programmeMode: $programmeMode, studySystem: $studySystem, gender: $gender, yearJoined: $yearJoined, eduStatus: $eduStatus, school: $school, campus: $campus)';
}


}

/// @nodoc
abstract mixin class _$GradeHistoryStudentInfoCopyWith<$Res> implements $GradeHistoryStudentInfoCopyWith<$Res> {
  factory _$GradeHistoryStudentInfoCopyWith(_GradeHistoryStudentInfo value, $Res Function(_GradeHistoryStudentInfo) _then) = __$GradeHistoryStudentInfoCopyWithImpl;
@override @useResult
$Res call({
 String regNo, String name, String programmeBranch, String programmeMode, String studySystem, String gender, String yearJoined, String eduStatus, String school, String campus
});




}
/// @nodoc
class __$GradeHistoryStudentInfoCopyWithImpl<$Res>
    implements _$GradeHistoryStudentInfoCopyWith<$Res> {
  __$GradeHistoryStudentInfoCopyWithImpl(this._self, this._then);

  final _GradeHistoryStudentInfo _self;
  final $Res Function(_GradeHistoryStudentInfo) _then;

/// Create a copy of GradeHistoryStudentInfo
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? regNo = null,Object? name = null,Object? programmeBranch = null,Object? programmeMode = null,Object? studySystem = null,Object? gender = null,Object? yearJoined = null,Object? eduStatus = null,Object? school = null,Object? campus = null,}) {
  return _then(_GradeHistoryStudentInfo(
regNo: null == regNo ? _self.regNo : regNo // ignore: cast_nullable_to_non_nullable
as String,name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,programmeBranch: null == programmeBranch ? _self.programmeBranch : programmeBranch // ignore: cast_nullable_to_non_nullable
as String,programmeMode: null == programmeMode ? _self.programmeMode : programmeMode // ignore: cast_nullable_to_non_nullable
as String,studySystem: null == studySystem ? _self.studySystem : studySystem // ignore: cast_nullable_to_non_nullable
as String,gender: null == gender ? _self.gender : gender // ignore: cast_nullable_to_non_nullable
as String,yearJoined: null == yearJoined ? _self.yearJoined : yearJoined // ignore: cast_nullable_to_non_nullable
as String,eduStatus: null == eduStatus ? _self.eduStatus : eduStatus // ignore: cast_nullable_to_non_nullable
as String,school: null == school ? _self.school : school // ignore: cast_nullable_to_non_nullable
as String,campus: null == campus ? _self.campus : campus // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}


/// @nodoc
mixin _$GradeRange {

 String get grade; String get range;
/// Create a copy of GradeRange
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GradeRangeCopyWith<GradeRange> get copyWith => _$GradeRangeCopyWithImpl<GradeRange>(this as GradeRange, _$identity);

  /// Serializes this GradeRange to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is GradeRange&&(identical(other.grade, grade) || other.grade == grade)&&(identical(other.range, range) || other.range == range));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,grade,range);

@override
String toString() {
  return 'GradeRange(grade: $grade, range: $range)';
}


}

/// @nodoc
abstract mixin class $GradeRangeCopyWith<$Res>  {
  factory $GradeRangeCopyWith(GradeRange value, $Res Function(GradeRange) _then) = _$GradeRangeCopyWithImpl;
@useResult
$Res call({
 String grade, String range
});




}
/// @nodoc
class _$GradeRangeCopyWithImpl<$Res>
    implements $GradeRangeCopyWith<$Res> {
  _$GradeRangeCopyWithImpl(this._self, this._then);

  final GradeRange _self;
  final $Res Function(GradeRange) _then;

/// Create a copy of GradeRange
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? grade = null,Object? range = null,}) {
  return _then(_self.copyWith(
grade: null == grade ? _self.grade : grade // ignore: cast_nullable_to_non_nullable
as String,range: null == range ? _self.range : range // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// Adds pattern-matching-related methods to [GradeRange].
extension GradeRangePatterns on GradeRange {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _GradeRange value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _GradeRange() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _GradeRange value)  $default,){
final _that = this;
switch (_that) {
case _GradeRange():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _GradeRange value)?  $default,){
final _that = this;
switch (_that) {
case _GradeRange() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String grade,  String range)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _GradeRange() when $default != null:
return $default(_that.grade,_that.range);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String grade,  String range)  $default,) {final _that = this;
switch (_that) {
case _GradeRange():
return $default(_that.grade,_that.range);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String grade,  String range)?  $default,) {final _that = this;
switch (_that) {
case _GradeRange() when $default != null:
return $default(_that.grade,_that.range);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _GradeRange implements GradeRange {
  const _GradeRange({required this.grade, required this.range});
  factory _GradeRange.fromJson(Map<String, dynamic> json) => _$GradeRangeFromJson(json);

@override final  String grade;
@override final  String range;

/// Create a copy of GradeRange
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$GradeRangeCopyWith<_GradeRange> get copyWith => __$GradeRangeCopyWithImpl<_GradeRange>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$GradeRangeToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _GradeRange&&(identical(other.grade, grade) || other.grade == grade)&&(identical(other.range, range) || other.range == range));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,grade,range);

@override
String toString() {
  return 'GradeRange(grade: $grade, range: $range)';
}


}

/// @nodoc
abstract mixin class _$GradeRangeCopyWith<$Res> implements $GradeRangeCopyWith<$Res> {
  factory _$GradeRangeCopyWith(_GradeRange value, $Res Function(_GradeRange) _then) = __$GradeRangeCopyWithImpl;
@override @useResult
$Res call({
 String grade, String range
});




}
/// @nodoc
class __$GradeRangeCopyWithImpl<$Res>
    implements _$GradeRangeCopyWith<$Res> {
  __$GradeRangeCopyWithImpl(this._self, this._then);

  final _GradeRange _self;
  final $Res Function(_GradeRange) _then;

/// Create a copy of GradeRange
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? grade = null,Object? range = null,}) {
  return _then(_GradeRange(
grade: null == grade ? _self.grade : grade // ignore: cast_nullable_to_non_nullable
as String,range: null == range ? _self.range : range // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}


/// @nodoc
mixin _$GradeViewData {

 List<GradeCourseRecord> get courses; List<SemesterInfo> get semesters; String get semesterId; BigInt get updateTime;
/// Create a copy of GradeViewData
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GradeViewDataCopyWith<GradeViewData> get copyWith => _$GradeViewDataCopyWithImpl<GradeViewData>(this as GradeViewData, _$identity);

  /// Serializes this GradeViewData to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is GradeViewData&&const DeepCollectionEquality().equals(other.courses, courses)&&const DeepCollectionEquality().equals(other.semesters, semesters)&&(identical(other.semesterId, semesterId) || other.semesterId == semesterId)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(courses),const DeepCollectionEquality().hash(semesters),semesterId,updateTime);

@override
String toString() {
  return 'GradeViewData(courses: $courses, semesters: $semesters, semesterId: $semesterId, updateTime: $updateTime)';
}


}

/// @nodoc
abstract mixin class $GradeViewDataCopyWith<$Res>  {
  factory $GradeViewDataCopyWith(GradeViewData value, $Res Function(GradeViewData) _then) = _$GradeViewDataCopyWithImpl;
@useResult
$Res call({
 List<GradeCourseRecord> courses, List<SemesterInfo> semesters, String semesterId, BigInt updateTime
});




}
/// @nodoc
class _$GradeViewDataCopyWithImpl<$Res>
    implements $GradeViewDataCopyWith<$Res> {
  _$GradeViewDataCopyWithImpl(this._self, this._then);

  final GradeViewData _self;
  final $Res Function(GradeViewData) _then;

/// Create a copy of GradeViewData
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? courses = null,Object? semesters = null,Object? semesterId = null,Object? updateTime = null,}) {
  return _then(_self.copyWith(
courses: null == courses ? _self.courses : courses // ignore: cast_nullable_to_non_nullable
as List<GradeCourseRecord>,semesters: null == semesters ? _self.semesters : semesters // ignore: cast_nullable_to_non_nullable
as List<SemesterInfo>,semesterId: null == semesterId ? _self.semesterId : semesterId // ignore: cast_nullable_to_non_nullable
as String,updateTime: null == updateTime ? _self.updateTime : updateTime // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}

}


/// Adds pattern-matching-related methods to [GradeViewData].
extension GradeViewDataPatterns on GradeViewData {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _GradeViewData value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _GradeViewData() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _GradeViewData value)  $default,){
final _that = this;
switch (_that) {
case _GradeViewData():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _GradeViewData value)?  $default,){
final _that = this;
switch (_that) {
case _GradeViewData() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( List<GradeCourseRecord> courses,  List<SemesterInfo> semesters,  String semesterId,  BigInt updateTime)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _GradeViewData() when $default != null:
return $default(_that.courses,_that.semesters,_that.semesterId,_that.updateTime);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( List<GradeCourseRecord> courses,  List<SemesterInfo> semesters,  String semesterId,  BigInt updateTime)  $default,) {final _that = this;
switch (_that) {
case _GradeViewData():
return $default(_that.courses,_that.semesters,_that.semesterId,_that.updateTime);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( List<GradeCourseRecord> courses,  List<SemesterInfo> semesters,  String semesterId,  BigInt updateTime)?  $default,) {final _that = this;
switch (_that) {
case _GradeViewData() when $default != null:
return $default(_that.courses,_that.semesters,_that.semesterId,_that.updateTime);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _GradeViewData implements GradeViewData {
  const _GradeViewData({required final  List<GradeCourseRecord> courses, required final  List<SemesterInfo> semesters, required this.semesterId, required this.updateTime}): _courses = courses,_semesters = semesters;
  factory _GradeViewData.fromJson(Map<String, dynamic> json) => _$GradeViewDataFromJson(json);

 final  List<GradeCourseRecord> _courses;
@override List<GradeCourseRecord> get courses {
  if (_courses is EqualUnmodifiableListView) return _courses;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_courses);
}

 final  List<SemesterInfo> _semesters;
@override List<SemesterInfo> get semesters {
  if (_semesters is EqualUnmodifiableListView) return _semesters;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_semesters);
}

@override final  String semesterId;
@override final  BigInt updateTime;

/// Create a copy of GradeViewData
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$GradeViewDataCopyWith<_GradeViewData> get copyWith => __$GradeViewDataCopyWithImpl<_GradeViewData>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$GradeViewDataToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _GradeViewData&&const DeepCollectionEquality().equals(other._courses, _courses)&&const DeepCollectionEquality().equals(other._semesters, _semesters)&&(identical(other.semesterId, semesterId) || other.semesterId == semesterId)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(_courses),const DeepCollectionEquality().hash(_semesters),semesterId,updateTime);

@override
String toString() {
  return 'GradeViewData(courses: $courses, semesters: $semesters, semesterId: $semesterId, updateTime: $updateTime)';
}


}

/// @nodoc
abstract mixin class _$GradeViewDataCopyWith<$Res> implements $GradeViewDataCopyWith<$Res> {
  factory _$GradeViewDataCopyWith(_GradeViewData value, $Res Function(_GradeViewData) _then) = __$GradeViewDataCopyWithImpl;
@override @useResult
$Res call({
 List<GradeCourseRecord> courses, List<SemesterInfo> semesters, String semesterId, BigInt updateTime
});




}
/// @nodoc
class __$GradeViewDataCopyWithImpl<$Res>
    implements _$GradeViewDataCopyWith<$Res> {
  __$GradeViewDataCopyWithImpl(this._self, this._then);

  final _GradeViewData _self;
  final $Res Function(_GradeViewData) _then;

/// Create a copy of GradeViewData
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? courses = null,Object? semesters = null,Object? semesterId = null,Object? updateTime = null,}) {
  return _then(_GradeViewData(
courses: null == courses ? _self._courses : courses // ignore: cast_nullable_to_non_nullable
as List<GradeCourseRecord>,semesters: null == semesters ? _self._semesters : semesters // ignore: cast_nullable_to_non_nullable
as List<SemesterInfo>,semesterId: null == semesterId ? _self.semesterId : semesterId // ignore: cast_nullable_to_non_nullable
as String,updateTime: null == updateTime ? _self.updateTime : updateTime // ignore: cast_nullable_to_non_nullable
as BigInt,
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

  /// Serializes this MarksData to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is MarksData&&const DeepCollectionEquality().equals(other.records, records)&&(identical(other.semesterId, semesterId) || other.semesterId == semesterId)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}

@JsonKey(includeFromJson: false, includeToJson: false)
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


/// Adds pattern-matching-related methods to [MarksData].
extension MarksDataPatterns on MarksData {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _MarksData value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _MarksData() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _MarksData value)  $default,){
final _that = this;
switch (_that) {
case _MarksData():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _MarksData value)?  $default,){
final _that = this;
switch (_that) {
case _MarksData() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( List<MarksRecord> records,  String semesterId,  BigInt updateTime)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _MarksData() when $default != null:
return $default(_that.records,_that.semesterId,_that.updateTime);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( List<MarksRecord> records,  String semesterId,  BigInt updateTime)  $default,) {final _that = this;
switch (_that) {
case _MarksData():
return $default(_that.records,_that.semesterId,_that.updateTime);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( List<MarksRecord> records,  String semesterId,  BigInt updateTime)?  $default,) {final _that = this;
switch (_that) {
case _MarksData() when $default != null:
return $default(_that.records,_that.semesterId,_that.updateTime);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _MarksData implements MarksData {
  const _MarksData({required final  List<MarksRecord> records, required this.semesterId, required this.updateTime}): _records = records;
  factory _MarksData.fromJson(Map<String, dynamic> json) => _$MarksDataFromJson(json);

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
Map<String, dynamic> toJson() {
  return _$MarksDataToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _MarksData&&const DeepCollectionEquality().equals(other._records, _records)&&(identical(other.semesterId, semesterId) || other.semesterId == semesterId)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}

@JsonKey(includeFromJson: false, includeToJson: false)
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

  /// Serializes this MarksRecord to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is MarksRecord&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.coursecode, coursecode) || other.coursecode == coursecode)&&(identical(other.coursetitle, coursetitle) || other.coursetitle == coursetitle)&&(identical(other.coursetype, coursetype) || other.coursetype == coursetype)&&(identical(other.faculity, faculity) || other.faculity == faculity)&&(identical(other.slot, slot) || other.slot == slot)&&const DeepCollectionEquality().equals(other.marks, marks));
}

@JsonKey(includeFromJson: false, includeToJson: false)
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


/// Adds pattern-matching-related methods to [MarksRecord].
extension MarksRecordPatterns on MarksRecord {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _MarksRecord value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _MarksRecord() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _MarksRecord value)  $default,){
final _that = this;
switch (_that) {
case _MarksRecord():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _MarksRecord value)?  $default,){
final _that = this;
switch (_that) {
case _MarksRecord() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String serial,  String coursecode,  String coursetitle,  String coursetype,  String faculity,  String slot,  List<MarksRecordEach> marks)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _MarksRecord() when $default != null:
return $default(_that.serial,_that.coursecode,_that.coursetitle,_that.coursetype,_that.faculity,_that.slot,_that.marks);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String serial,  String coursecode,  String coursetitle,  String coursetype,  String faculity,  String slot,  List<MarksRecordEach> marks)  $default,) {final _that = this;
switch (_that) {
case _MarksRecord():
return $default(_that.serial,_that.coursecode,_that.coursetitle,_that.coursetype,_that.faculity,_that.slot,_that.marks);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String serial,  String coursecode,  String coursetitle,  String coursetype,  String faculity,  String slot,  List<MarksRecordEach> marks)?  $default,) {final _that = this;
switch (_that) {
case _MarksRecord() when $default != null:
return $default(_that.serial,_that.coursecode,_that.coursetitle,_that.coursetype,_that.faculity,_that.slot,_that.marks);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _MarksRecord implements MarksRecord {
  const _MarksRecord({required this.serial, required this.coursecode, required this.coursetitle, required this.coursetype, required this.faculity, required this.slot, required final  List<MarksRecordEach> marks}): _marks = marks;
  factory _MarksRecord.fromJson(Map<String, dynamic> json) => _$MarksRecordFromJson(json);

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
Map<String, dynamic> toJson() {
  return _$MarksRecordToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _MarksRecord&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.coursecode, coursecode) || other.coursecode == coursecode)&&(identical(other.coursetitle, coursetitle) || other.coursetitle == coursetitle)&&(identical(other.coursetype, coursetype) || other.coursetype == coursetype)&&(identical(other.faculity, faculity) || other.faculity == faculity)&&(identical(other.slot, slot) || other.slot == slot)&&const DeepCollectionEquality().equals(other._marks, _marks));
}

@JsonKey(includeFromJson: false, includeToJson: false)
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


/// Adds pattern-matching-related methods to [MarksRecordEach].
extension MarksRecordEachPatterns on MarksRecordEach {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _MarksRecordEach value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _MarksRecordEach() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _MarksRecordEach value)  $default,){
final _that = this;
switch (_that) {
case _MarksRecordEach():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _MarksRecordEach value)?  $default,){
final _that = this;
switch (_that) {
case _MarksRecordEach() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String serial,  String markstitle,  String maxmarks,  String weightage,  String status,  String scoredmark,  String weightagemark,  String remark)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _MarksRecordEach() when $default != null:
return $default(_that.serial,_that.markstitle,_that.maxmarks,_that.weightage,_that.status,_that.scoredmark,_that.weightagemark,_that.remark);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String serial,  String markstitle,  String maxmarks,  String weightage,  String status,  String scoredmark,  String weightagemark,  String remark)  $default,) {final _that = this;
switch (_that) {
case _MarksRecordEach():
return $default(_that.serial,_that.markstitle,_that.maxmarks,_that.weightage,_that.status,_that.scoredmark,_that.weightagemark,_that.remark);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String serial,  String markstitle,  String maxmarks,  String weightage,  String status,  String scoredmark,  String weightagemark,  String remark)?  $default,) {final _that = this;
switch (_that) {
case _MarksRecordEach() when $default != null:
return $default(_that.serial,_that.markstitle,_that.maxmarks,_that.weightage,_that.status,_that.scoredmark,_that.weightagemark,_that.remark);case _:
  return null;

}
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

  /// Serializes this PerExamScheduleRecord to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PerExamScheduleRecord&&const DeepCollectionEquality().equals(other.records, records)&&(identical(other.examType, examType) || other.examType == examType));
}

@JsonKey(includeFromJson: false, includeToJson: false)
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


/// Adds pattern-matching-related methods to [PerExamScheduleRecord].
extension PerExamScheduleRecordPatterns on PerExamScheduleRecord {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _PerExamScheduleRecord value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _PerExamScheduleRecord() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _PerExamScheduleRecord value)  $default,){
final _that = this;
switch (_that) {
case _PerExamScheduleRecord():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _PerExamScheduleRecord value)?  $default,){
final _that = this;
switch (_that) {
case _PerExamScheduleRecord() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( List<ExamScheduleRecord> records,  String examType)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _PerExamScheduleRecord() when $default != null:
return $default(_that.records,_that.examType);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( List<ExamScheduleRecord> records,  String examType)  $default,) {final _that = this;
switch (_that) {
case _PerExamScheduleRecord():
return $default(_that.records,_that.examType);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( List<ExamScheduleRecord> records,  String examType)?  $default,) {final _that = this;
switch (_that) {
case _PerExamScheduleRecord() when $default != null:
return $default(_that.records,_that.examType);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _PerExamScheduleRecord implements PerExamScheduleRecord {
  const _PerExamScheduleRecord({required final  List<ExamScheduleRecord> records, required this.examType}): _records = records;
  factory _PerExamScheduleRecord.fromJson(Map<String, dynamic> json) => _$PerExamScheduleRecordFromJson(json);

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
Map<String, dynamic> toJson() {
  return _$PerExamScheduleRecordToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _PerExamScheduleRecord&&const DeepCollectionEquality().equals(other._records, _records)&&(identical(other.examType, examType) || other.examType == examType));
}

@JsonKey(includeFromJson: false, includeToJson: false)
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

  /// Serializes this SemesterData to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SemesterData&&const DeepCollectionEquality().equals(other.semesters, semesters)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}

@JsonKey(includeFromJson: false, includeToJson: false)
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


/// Adds pattern-matching-related methods to [SemesterData].
extension SemesterDataPatterns on SemesterData {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _SemesterData value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _SemesterData() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _SemesterData value)  $default,){
final _that = this;
switch (_that) {
case _SemesterData():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _SemesterData value)?  $default,){
final _that = this;
switch (_that) {
case _SemesterData() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( List<SemesterInfo> semesters,  BigInt updateTime)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _SemesterData() when $default != null:
return $default(_that.semesters,_that.updateTime);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( List<SemesterInfo> semesters,  BigInt updateTime)  $default,) {final _that = this;
switch (_that) {
case _SemesterData():
return $default(_that.semesters,_that.updateTime);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( List<SemesterInfo> semesters,  BigInt updateTime)?  $default,) {final _that = this;
switch (_that) {
case _SemesterData() when $default != null:
return $default(_that.semesters,_that.updateTime);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _SemesterData implements SemesterData {
  const _SemesterData({required final  List<SemesterInfo> semesters, required this.updateTime}): _semesters = semesters;
  factory _SemesterData.fromJson(Map<String, dynamic> json) => _$SemesterDataFromJson(json);

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
Map<String, dynamic> toJson() {
  return _$SemesterDataToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _SemesterData&&const DeepCollectionEquality().equals(other._semesters, _semesters)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}

@JsonKey(includeFromJson: false, includeToJson: false)
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

  /// Serializes this SemesterInfo to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SemesterInfo&&(identical(other.id, id) || other.id == id)&&(identical(other.name, name) || other.name == name));
}

@JsonKey(includeFromJson: false, includeToJson: false)
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


/// Adds pattern-matching-related methods to [SemesterInfo].
extension SemesterInfoPatterns on SemesterInfo {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _SemesterInfo value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _SemesterInfo() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _SemesterInfo value)  $default,){
final _that = this;
switch (_that) {
case _SemesterInfo():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _SemesterInfo value)?  $default,){
final _that = this;
switch (_that) {
case _SemesterInfo() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String id,  String name)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _SemesterInfo() when $default != null:
return $default(_that.id,_that.name);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String id,  String name)  $default,) {final _that = this;
switch (_that) {
case _SemesterInfo():
return $default(_that.id,_that.name);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String id,  String name)?  $default,) {final _that = this;
switch (_that) {
case _SemesterInfo() when $default != null:
return $default(_that.id,_that.name);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _SemesterInfo implements SemesterInfo {
  const _SemesterInfo({required this.id, required this.name});
  factory _SemesterInfo.fromJson(Map<String, dynamic> json) => _$SemesterInfoFromJson(json);

@override final  String id;
@override final  String name;

/// Create a copy of SemesterInfo
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$SemesterInfoCopyWith<_SemesterInfo> get copyWith => __$SemesterInfoCopyWithImpl<_SemesterInfo>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$SemesterInfoToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _SemesterInfo&&(identical(other.id, id) || other.id == id)&&(identical(other.name, name) || other.name == name));
}

@JsonKey(includeFromJson: false, includeToJson: false)
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

  /// Serializes this TimetableData to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TimetableData&&const DeepCollectionEquality().equals(other.slots, slots)&&(identical(other.semesterId, semesterId) || other.semesterId == semesterId)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}

@JsonKey(includeFromJson: false, includeToJson: false)
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


/// Adds pattern-matching-related methods to [TimetableData].
extension TimetableDataPatterns on TimetableData {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _TimetableData value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _TimetableData() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _TimetableData value)  $default,){
final _that = this;
switch (_that) {
case _TimetableData():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _TimetableData value)?  $default,){
final _that = this;
switch (_that) {
case _TimetableData() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( List<TimetableSlot> slots,  String semesterId,  BigInt updateTime)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _TimetableData() when $default != null:
return $default(_that.slots,_that.semesterId,_that.updateTime);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( List<TimetableSlot> slots,  String semesterId,  BigInt updateTime)  $default,) {final _that = this;
switch (_that) {
case _TimetableData():
return $default(_that.slots,_that.semesterId,_that.updateTime);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( List<TimetableSlot> slots,  String semesterId,  BigInt updateTime)?  $default,) {final _that = this;
switch (_that) {
case _TimetableData() when $default != null:
return $default(_that.slots,_that.semesterId,_that.updateTime);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _TimetableData implements TimetableData {
  const _TimetableData({required final  List<TimetableSlot> slots, required this.semesterId, required this.updateTime}): _slots = slots;
  factory _TimetableData.fromJson(Map<String, dynamic> json) => _$TimetableDataFromJson(json);

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
Map<String, dynamic> toJson() {
  return _$TimetableDataToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _TimetableData&&const DeepCollectionEquality().equals(other._slots, _slots)&&(identical(other.semesterId, semesterId) || other.semesterId == semesterId)&&(identical(other.updateTime, updateTime) || other.updateTime == updateTime));
}

@JsonKey(includeFromJson: false, includeToJson: false)
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

 String get serial; String get day; String get slot; String get courseCode; String get courseType; String get roomNo; String get block; String get startTime; String get endTime; String get name; bool get isLab; String get faculty;
/// Create a copy of TimetableSlot
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$TimetableSlotCopyWith<TimetableSlot> get copyWith => _$TimetableSlotCopyWithImpl<TimetableSlot>(this as TimetableSlot, _$identity);

  /// Serializes this TimetableSlot to a JSON map.
  Map<String, dynamic> toJson();


@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TimetableSlot&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.day, day) || other.day == day)&&(identical(other.slot, slot) || other.slot == slot)&&(identical(other.courseCode, courseCode) || other.courseCode == courseCode)&&(identical(other.courseType, courseType) || other.courseType == courseType)&&(identical(other.roomNo, roomNo) || other.roomNo == roomNo)&&(identical(other.block, block) || other.block == block)&&(identical(other.startTime, startTime) || other.startTime == startTime)&&(identical(other.endTime, endTime) || other.endTime == endTime)&&(identical(other.name, name) || other.name == name)&&(identical(other.isLab, isLab) || other.isLab == isLab)&&(identical(other.faculty, faculty) || other.faculty == faculty));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,serial,day,slot,courseCode,courseType,roomNo,block,startTime,endTime,name,isLab,faculty);

@override
String toString() {
  return 'TimetableSlot(serial: $serial, day: $day, slot: $slot, courseCode: $courseCode, courseType: $courseType, roomNo: $roomNo, block: $block, startTime: $startTime, endTime: $endTime, name: $name, isLab: $isLab, faculty: $faculty)';
}


}

/// @nodoc
abstract mixin class $TimetableSlotCopyWith<$Res>  {
  factory $TimetableSlotCopyWith(TimetableSlot value, $Res Function(TimetableSlot) _then) = _$TimetableSlotCopyWithImpl;
@useResult
$Res call({
 String serial, String day, String slot, String courseCode, String courseType, String roomNo, String block, String startTime, String endTime, String name, bool isLab, String faculty
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
@pragma('vm:prefer-inline') @override $Res call({Object? serial = null,Object? day = null,Object? slot = null,Object? courseCode = null,Object? courseType = null,Object? roomNo = null,Object? block = null,Object? startTime = null,Object? endTime = null,Object? name = null,Object? isLab = null,Object? faculty = null,}) {
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
as String,isLab: null == isLab ? _self.isLab : isLab // ignore: cast_nullable_to_non_nullable
as bool,faculty: null == faculty ? _self.faculty : faculty // ignore: cast_nullable_to_non_nullable
as String,
  ));
}

}


/// Adds pattern-matching-related methods to [TimetableSlot].
extension TimetableSlotPatterns on TimetableSlot {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>(TResult Function( _TimetableSlot value)?  $default,{required TResult orElse(),}){
final _that = this;
switch (_that) {
case _TimetableSlot() when $default != null:
return $default(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>(TResult Function( _TimetableSlot value)  $default,){
final _that = this;
switch (_that) {
case _TimetableSlot():
return $default(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>(TResult? Function( _TimetableSlot value)?  $default,){
final _that = this;
switch (_that) {
case _TimetableSlot() when $default != null:
return $default(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>(TResult Function( String serial,  String day,  String slot,  String courseCode,  String courseType,  String roomNo,  String block,  String startTime,  String endTime,  String name,  bool isLab,  String faculty)?  $default,{required TResult orElse(),}) {final _that = this;
switch (_that) {
case _TimetableSlot() when $default != null:
return $default(_that.serial,_that.day,_that.slot,_that.courseCode,_that.courseType,_that.roomNo,_that.block,_that.startTime,_that.endTime,_that.name,_that.isLab,_that.faculty);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>(TResult Function( String serial,  String day,  String slot,  String courseCode,  String courseType,  String roomNo,  String block,  String startTime,  String endTime,  String name,  bool isLab,  String faculty)  $default,) {final _that = this;
switch (_that) {
case _TimetableSlot():
return $default(_that.serial,_that.day,_that.slot,_that.courseCode,_that.courseType,_that.roomNo,_that.block,_that.startTime,_that.endTime,_that.name,_that.isLab,_that.faculty);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>(TResult? Function( String serial,  String day,  String slot,  String courseCode,  String courseType,  String roomNo,  String block,  String startTime,  String endTime,  String name,  bool isLab,  String faculty)?  $default,) {final _that = this;
switch (_that) {
case _TimetableSlot() when $default != null:
return $default(_that.serial,_that.day,_that.slot,_that.courseCode,_that.courseType,_that.roomNo,_that.block,_that.startTime,_that.endTime,_that.name,_that.isLab,_that.faculty);case _:
  return null;

}
}

}

/// @nodoc
@JsonSerializable()

class _TimetableSlot implements TimetableSlot {
  const _TimetableSlot({required this.serial, required this.day, required this.slot, required this.courseCode, required this.courseType, required this.roomNo, required this.block, required this.startTime, required this.endTime, required this.name, required this.isLab, required this.faculty});
  factory _TimetableSlot.fromJson(Map<String, dynamic> json) => _$TimetableSlotFromJson(json);

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
@override final  bool isLab;
@override final  String faculty;

/// Create a copy of TimetableSlot
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$TimetableSlotCopyWith<_TimetableSlot> get copyWith => __$TimetableSlotCopyWithImpl<_TimetableSlot>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$TimetableSlotToJson(this, );
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _TimetableSlot&&(identical(other.serial, serial) || other.serial == serial)&&(identical(other.day, day) || other.day == day)&&(identical(other.slot, slot) || other.slot == slot)&&(identical(other.courseCode, courseCode) || other.courseCode == courseCode)&&(identical(other.courseType, courseType) || other.courseType == courseType)&&(identical(other.roomNo, roomNo) || other.roomNo == roomNo)&&(identical(other.block, block) || other.block == block)&&(identical(other.startTime, startTime) || other.startTime == startTime)&&(identical(other.endTime, endTime) || other.endTime == endTime)&&(identical(other.name, name) || other.name == name)&&(identical(other.isLab, isLab) || other.isLab == isLab)&&(identical(other.faculty, faculty) || other.faculty == faculty));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,serial,day,slot,courseCode,courseType,roomNo,block,startTime,endTime,name,isLab,faculty);

@override
String toString() {
  return 'TimetableSlot(serial: $serial, day: $day, slot: $slot, courseCode: $courseCode, courseType: $courseType, roomNo: $roomNo, block: $block, startTime: $startTime, endTime: $endTime, name: $name, isLab: $isLab, faculty: $faculty)';
}


}

/// @nodoc
abstract mixin class _$TimetableSlotCopyWith<$Res> implements $TimetableSlotCopyWith<$Res> {
  factory _$TimetableSlotCopyWith(_TimetableSlot value, $Res Function(_TimetableSlot) _then) = __$TimetableSlotCopyWithImpl;
@override @useResult
$Res call({
 String serial, String day, String slot, String courseCode, String courseType, String roomNo, String block, String startTime, String endTime, String name, bool isLab, String faculty
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
@override @pragma('vm:prefer-inline') $Res call({Object? serial = null,Object? day = null,Object? slot = null,Object? courseCode = null,Object? courseType = null,Object? roomNo = null,Object? block = null,Object? startTime = null,Object? endTime = null,Object? name = null,Object? isLab = null,Object? faculty = null,}) {
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
as String,isLab: null == isLab ? _self.isLab : isLab // ignore: cast_nullable_to_non_nullable
as bool,faculty: null == faculty ? _self.faculty : faculty // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
