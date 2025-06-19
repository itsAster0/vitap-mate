// dart format width=80
// coverage:ignore-file
// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'vtop_user_entity.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;

/// @nodoc
mixin _$VtopUserEntity implements DiagnosticableTreeMixin {

 String? get username; String? get password; String? get semid; bool get isValid;
/// Create a copy of VtopUserEntity
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$VtopUserEntityCopyWith<VtopUserEntity> get copyWith => _$VtopUserEntityCopyWithImpl<VtopUserEntity>(this as VtopUserEntity, _$identity);

  /// Serializes this VtopUserEntity to a JSON map.
  Map<String, dynamic> toJson();

@override
void debugFillProperties(DiagnosticPropertiesBuilder properties) {
  properties
    ..add(DiagnosticsProperty('type', 'VtopUserEntity'))
    ..add(DiagnosticsProperty('username', username))..add(DiagnosticsProperty('password', password))..add(DiagnosticsProperty('semid', semid))..add(DiagnosticsProperty('isValid', isValid));
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is VtopUserEntity&&(identical(other.username, username) || other.username == username)&&(identical(other.password, password) || other.password == password)&&(identical(other.semid, semid) || other.semid == semid)&&(identical(other.isValid, isValid) || other.isValid == isValid));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,username,password,semid,isValid);

@override
String toString({ DiagnosticLevel minLevel = DiagnosticLevel.info }) {
  return 'VtopUserEntity(username: $username, password: $password, semid: $semid, isValid: $isValid)';
}


}

/// @nodoc
abstract mixin class $VtopUserEntityCopyWith<$Res>  {
  factory $VtopUserEntityCopyWith(VtopUserEntity value, $Res Function(VtopUserEntity) _then) = _$VtopUserEntityCopyWithImpl;
@useResult
$Res call({
 String? username, String? password, String? semid, bool isValid
});




}
/// @nodoc
class _$VtopUserEntityCopyWithImpl<$Res>
    implements $VtopUserEntityCopyWith<$Res> {
  _$VtopUserEntityCopyWithImpl(this._self, this._then);

  final VtopUserEntity _self;
  final $Res Function(VtopUserEntity) _then;

/// Create a copy of VtopUserEntity
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? username = freezed,Object? password = freezed,Object? semid = freezed,Object? isValid = null,}) {
  return _then(_self.copyWith(
username: freezed == username ? _self.username : username // ignore: cast_nullable_to_non_nullable
as String?,password: freezed == password ? _self.password : password // ignore: cast_nullable_to_non_nullable
as String?,semid: freezed == semid ? _self.semid : semid // ignore: cast_nullable_to_non_nullable
as String?,isValid: null == isValid ? _self.isValid : isValid // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}

}


/// @nodoc
@JsonSerializable()

class _VtopUserEntity with DiagnosticableTreeMixin implements VtopUserEntity {
   _VtopUserEntity({required this.username, required this.password, required this.semid, this.isValid = false});
  factory _VtopUserEntity.fromJson(Map<String, dynamic> json) => _$VtopUserEntityFromJson(json);

@override final  String? username;
@override final  String? password;
@override final  String? semid;
@override@JsonKey() final  bool isValid;

/// Create a copy of VtopUserEntity
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$VtopUserEntityCopyWith<_VtopUserEntity> get copyWith => __$VtopUserEntityCopyWithImpl<_VtopUserEntity>(this, _$identity);

@override
Map<String, dynamic> toJson() {
  return _$VtopUserEntityToJson(this, );
}
@override
void debugFillProperties(DiagnosticPropertiesBuilder properties) {
  properties
    ..add(DiagnosticsProperty('type', 'VtopUserEntity'))
    ..add(DiagnosticsProperty('username', username))..add(DiagnosticsProperty('password', password))..add(DiagnosticsProperty('semid', semid))..add(DiagnosticsProperty('isValid', isValid));
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _VtopUserEntity&&(identical(other.username, username) || other.username == username)&&(identical(other.password, password) || other.password == password)&&(identical(other.semid, semid) || other.semid == semid)&&(identical(other.isValid, isValid) || other.isValid == isValid));
}

@JsonKey(includeFromJson: false, includeToJson: false)
@override
int get hashCode => Object.hash(runtimeType,username,password,semid,isValid);

@override
String toString({ DiagnosticLevel minLevel = DiagnosticLevel.info }) {
  return 'VtopUserEntity(username: $username, password: $password, semid: $semid, isValid: $isValid)';
}


}

/// @nodoc
abstract mixin class _$VtopUserEntityCopyWith<$Res> implements $VtopUserEntityCopyWith<$Res> {
  factory _$VtopUserEntityCopyWith(_VtopUserEntity value, $Res Function(_VtopUserEntity) _then) = __$VtopUserEntityCopyWithImpl;
@override @useResult
$Res call({
 String? username, String? password, String? semid, bool isValid
});




}
/// @nodoc
class __$VtopUserEntityCopyWithImpl<$Res>
    implements _$VtopUserEntityCopyWith<$Res> {
  __$VtopUserEntityCopyWithImpl(this._self, this._then);

  final _VtopUserEntity _self;
  final $Res Function(_VtopUserEntity) _then;

/// Create a copy of VtopUserEntity
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? username = freezed,Object? password = freezed,Object? semid = freezed,Object? isValid = null,}) {
  return _then(_VtopUserEntity(
username: freezed == username ? _self.username : username // ignore: cast_nullable_to_non_nullable
as String?,password: freezed == password ? _self.password : password // ignore: cast_nullable_to_non_nullable
as String?,semid: freezed == semid ? _self.semid : semid // ignore: cast_nullable_to_non_nullable
as String?,isValid: null == isValid ? _self.isValid : isValid // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

// dart format on
