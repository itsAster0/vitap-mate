// dart format width=80
// coverage:ignore-file
// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'vtop_errors.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$VtopError {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is VtopError);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'VtopError()';
}


}

/// @nodoc
class $VtopErrorCopyWith<$Res>  {
$VtopErrorCopyWith(VtopError _, $Res Function(VtopError) __);
}


/// @nodoc


class VtopError_NetworkError extends VtopError {
  const VtopError_NetworkError(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is VtopError_NetworkError);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'VtopError.networkError()';
}


}




/// @nodoc


class VtopError_VtopServerError extends VtopError {
  const VtopError_VtopServerError(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is VtopError_VtopServerError);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'VtopError.vtopServerError()';
}


}




/// @nodoc


class VtopError_AuthenticationFailed extends VtopError {
  const VtopError_AuthenticationFailed(this.field0): super._();
  

 final  String field0;

/// Create a copy of VtopError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$VtopError_AuthenticationFailedCopyWith<VtopError_AuthenticationFailed> get copyWith => _$VtopError_AuthenticationFailedCopyWithImpl<VtopError_AuthenticationFailed>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is VtopError_AuthenticationFailed&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'VtopError.authenticationFailed(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $VtopError_AuthenticationFailedCopyWith<$Res> implements $VtopErrorCopyWith<$Res> {
  factory $VtopError_AuthenticationFailedCopyWith(VtopError_AuthenticationFailed value, $Res Function(VtopError_AuthenticationFailed) _then) = _$VtopError_AuthenticationFailedCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$VtopError_AuthenticationFailedCopyWithImpl<$Res>
    implements $VtopError_AuthenticationFailedCopyWith<$Res> {
  _$VtopError_AuthenticationFailedCopyWithImpl(this._self, this._then);

  final VtopError_AuthenticationFailed _self;
  final $Res Function(VtopError_AuthenticationFailed) _then;

/// Create a copy of VtopError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(VtopError_AuthenticationFailed(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class VtopError_RegistrationParsingError extends VtopError {
  const VtopError_RegistrationParsingError(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is VtopError_RegistrationParsingError);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'VtopError.registrationParsingError()';
}


}




/// @nodoc


class VtopError_InvalidCredentials extends VtopError {
  const VtopError_InvalidCredentials(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is VtopError_InvalidCredentials);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'VtopError.invalidCredentials()';
}


}




/// @nodoc


class VtopError_SessionExpired extends VtopError {
  const VtopError_SessionExpired(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is VtopError_SessionExpired);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'VtopError.sessionExpired()';
}


}




/// @nodoc


class VtopError_ParseError extends VtopError {
  const VtopError_ParseError(this.field0): super._();
  

 final  String field0;

/// Create a copy of VtopError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$VtopError_ParseErrorCopyWith<VtopError_ParseError> get copyWith => _$VtopError_ParseErrorCopyWithImpl<VtopError_ParseError>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is VtopError_ParseError&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'VtopError.parseError(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $VtopError_ParseErrorCopyWith<$Res> implements $VtopErrorCopyWith<$Res> {
  factory $VtopError_ParseErrorCopyWith(VtopError_ParseError value, $Res Function(VtopError_ParseError) _then) = _$VtopError_ParseErrorCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$VtopError_ParseErrorCopyWithImpl<$Res>
    implements $VtopError_ParseErrorCopyWith<$Res> {
  _$VtopError_ParseErrorCopyWithImpl(this._self, this._then);

  final VtopError_ParseError _self;
  final $Res Function(VtopError_ParseError) _then;

/// Create a copy of VtopError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(VtopError_ParseError(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class VtopError_ConfigurationError extends VtopError {
  const VtopError_ConfigurationError(this.field0): super._();
  

 final  String field0;

/// Create a copy of VtopError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$VtopError_ConfigurationErrorCopyWith<VtopError_ConfigurationError> get copyWith => _$VtopError_ConfigurationErrorCopyWithImpl<VtopError_ConfigurationError>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is VtopError_ConfigurationError&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'VtopError.configurationError(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $VtopError_ConfigurationErrorCopyWith<$Res> implements $VtopErrorCopyWith<$Res> {
  factory $VtopError_ConfigurationErrorCopyWith(VtopError_ConfigurationError value, $Res Function(VtopError_ConfigurationError) _then) = _$VtopError_ConfigurationErrorCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$VtopError_ConfigurationErrorCopyWithImpl<$Res>
    implements $VtopError_ConfigurationErrorCopyWith<$Res> {
  _$VtopError_ConfigurationErrorCopyWithImpl(this._self, this._then);

  final VtopError_ConfigurationError _self;
  final $Res Function(VtopError_ConfigurationError) _then;

/// Create a copy of VtopError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(VtopError_ConfigurationError(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class VtopError_CaptchaRequired extends VtopError {
  const VtopError_CaptchaRequired(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is VtopError_CaptchaRequired);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'VtopError.captchaRequired()';
}


}




/// @nodoc


class VtopError_InvalidResponse extends VtopError {
  const VtopError_InvalidResponse(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is VtopError_InvalidResponse);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'VtopError.invalidResponse()';
}


}




// dart format on
