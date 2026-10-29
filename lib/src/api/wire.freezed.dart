// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'wire.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$WireFetchError {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WireFetchError);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WireFetchError()';
}


}

/// @nodoc
class $WireFetchErrorCopyWith<$Res>  {
$WireFetchErrorCopyWith(WireFetchError _, $Res Function(WireFetchError) __);
}


/// Adds pattern-matching-related methods to [WireFetchError].
extension WireFetchErrorPatterns on WireFetchError {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( WireFetchError_Unreachable value)?  unreachable,TResult Function( WireFetchError_Timeout value)?  timeout,TResult Function( WireFetchError_WrongKey value)?  wrongKey,TResult Function( WireFetchError_Unsupported value)?  unsupported,TResult Function( WireFetchError_Failed value)?  failed,required TResult orElse(),}){
final _that = this;
switch (_that) {
case WireFetchError_Unreachable() when unreachable != null:
return unreachable(_that);case WireFetchError_Timeout() when timeout != null:
return timeout(_that);case WireFetchError_WrongKey() when wrongKey != null:
return wrongKey(_that);case WireFetchError_Unsupported() when unsupported != null:
return unsupported(_that);case WireFetchError_Failed() when failed != null:
return failed(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( WireFetchError_Unreachable value)  unreachable,required TResult Function( WireFetchError_Timeout value)  timeout,required TResult Function( WireFetchError_WrongKey value)  wrongKey,required TResult Function( WireFetchError_Unsupported value)  unsupported,required TResult Function( WireFetchError_Failed value)  failed,}){
final _that = this;
switch (_that) {
case WireFetchError_Unreachable():
return unreachable(_that);case WireFetchError_Timeout():
return timeout(_that);case WireFetchError_WrongKey():
return wrongKey(_that);case WireFetchError_Unsupported():
return unsupported(_that);case WireFetchError_Failed():
return failed(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( WireFetchError_Unreachable value)?  unreachable,TResult? Function( WireFetchError_Timeout value)?  timeout,TResult? Function( WireFetchError_WrongKey value)?  wrongKey,TResult? Function( WireFetchError_Unsupported value)?  unsupported,TResult? Function( WireFetchError_Failed value)?  failed,}){
final _that = this;
switch (_that) {
case WireFetchError_Unreachable() when unreachable != null:
return unreachable(_that);case WireFetchError_Timeout() when timeout != null:
return timeout(_that);case WireFetchError_WrongKey() when wrongKey != null:
return wrongKey(_that);case WireFetchError_Unsupported() when unsupported != null:
return unsupported(_that);case WireFetchError_Failed() when failed != null:
return failed(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  unreachable,TResult Function()?  timeout,TResult Function()?  wrongKey,TResult Function( String field0)?  unsupported,TResult Function( String field0)?  failed,required TResult orElse(),}) {final _that = this;
switch (_that) {
case WireFetchError_Unreachable() when unreachable != null:
return unreachable();case WireFetchError_Timeout() when timeout != null:
return timeout();case WireFetchError_WrongKey() when wrongKey != null:
return wrongKey();case WireFetchError_Unsupported() when unsupported != null:
return unsupported(_that.field0);case WireFetchError_Failed() when failed != null:
return failed(_that.field0);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  unreachable,required TResult Function()  timeout,required TResult Function()  wrongKey,required TResult Function( String field0)  unsupported,required TResult Function( String field0)  failed,}) {final _that = this;
switch (_that) {
case WireFetchError_Unreachable():
return unreachable();case WireFetchError_Timeout():
return timeout();case WireFetchError_WrongKey():
return wrongKey();case WireFetchError_Unsupported():
return unsupported(_that.field0);case WireFetchError_Failed():
return failed(_that.field0);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  unreachable,TResult? Function()?  timeout,TResult? Function()?  wrongKey,TResult? Function( String field0)?  unsupported,TResult? Function( String field0)?  failed,}) {final _that = this;
switch (_that) {
case WireFetchError_Unreachable() when unreachable != null:
return unreachable();case WireFetchError_Timeout() when timeout != null:
return timeout();case WireFetchError_WrongKey() when wrongKey != null:
return wrongKey();case WireFetchError_Unsupported() when unsupported != null:
return unsupported(_that.field0);case WireFetchError_Failed() when failed != null:
return failed(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class WireFetchError_Unreachable extends WireFetchError {
  const WireFetchError_Unreachable(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WireFetchError_Unreachable);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WireFetchError.unreachable()';
}


}




/// @nodoc


class WireFetchError_Timeout extends WireFetchError {
  const WireFetchError_Timeout(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WireFetchError_Timeout);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WireFetchError.timeout()';
}


}




/// @nodoc


class WireFetchError_WrongKey extends WireFetchError {
  const WireFetchError_WrongKey(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WireFetchError_WrongKey);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WireFetchError.wrongKey()';
}


}




/// @nodoc


class WireFetchError_Unsupported extends WireFetchError {
  const WireFetchError_Unsupported(this.field0): super._();
  

 final  String field0;

/// Create a copy of WireFetchError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WireFetchError_UnsupportedCopyWith<WireFetchError_Unsupported> get copyWith => _$WireFetchError_UnsupportedCopyWithImpl<WireFetchError_Unsupported>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WireFetchError_Unsupported&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'WireFetchError.unsupported(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $WireFetchError_UnsupportedCopyWith<$Res> implements $WireFetchErrorCopyWith<$Res> {
  factory $WireFetchError_UnsupportedCopyWith(WireFetchError_Unsupported value, $Res Function(WireFetchError_Unsupported) _then) = _$WireFetchError_UnsupportedCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$WireFetchError_UnsupportedCopyWithImpl<$Res>
    implements $WireFetchError_UnsupportedCopyWith<$Res> {
  _$WireFetchError_UnsupportedCopyWithImpl(this._self, this._then);

  final WireFetchError_Unsupported _self;
  final $Res Function(WireFetchError_Unsupported) _then;

/// Create a copy of WireFetchError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(WireFetchError_Unsupported(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WireFetchError_Failed extends WireFetchError {
  const WireFetchError_Failed(this.field0): super._();
  

 final  String field0;

/// Create a copy of WireFetchError
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WireFetchError_FailedCopyWith<WireFetchError_Failed> get copyWith => _$WireFetchError_FailedCopyWithImpl<WireFetchError_Failed>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WireFetchError_Failed&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'WireFetchError.failed(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $WireFetchError_FailedCopyWith<$Res> implements $WireFetchErrorCopyWith<$Res> {
  factory $WireFetchError_FailedCopyWith(WireFetchError_Failed value, $Res Function(WireFetchError_Failed) _then) = _$WireFetchError_FailedCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$WireFetchError_FailedCopyWithImpl<$Res>
    implements $WireFetchError_FailedCopyWith<$Res> {
  _$WireFetchError_FailedCopyWithImpl(this._self, this._then);

  final WireFetchError_Failed _self;
  final $Res Function(WireFetchError_Failed) _then;

/// Create a copy of WireFetchError
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(WireFetchError_Failed(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

// dart format on
