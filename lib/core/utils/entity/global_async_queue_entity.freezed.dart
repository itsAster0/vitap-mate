// dart format width=80
// coverage:ignore-file
// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'global_async_queue_entity.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$GlobalAsyncQueueEntity implements DiagnosticableTreeMixin {

 Map<String, Future> get running;
/// Create a copy of GlobalAsyncQueueEntity
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$GlobalAsyncQueueEntityCopyWith<GlobalAsyncQueueEntity> get copyWith => _$GlobalAsyncQueueEntityCopyWithImpl<GlobalAsyncQueueEntity>(this as GlobalAsyncQueueEntity, _$identity);


@override
void debugFillProperties(DiagnosticPropertiesBuilder properties) {
  properties
    ..add(DiagnosticsProperty('type', 'GlobalAsyncQueueEntity'))
    ..add(DiagnosticsProperty('running', running));
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is GlobalAsyncQueueEntity&&const DeepCollectionEquality().equals(other.running, running));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(running));

@override
String toString({ DiagnosticLevel minLevel = DiagnosticLevel.info }) {
  return 'GlobalAsyncQueueEntity(running: $running)';
}


}

/// @nodoc
abstract mixin class $GlobalAsyncQueueEntityCopyWith<$Res>  {
  factory $GlobalAsyncQueueEntityCopyWith(GlobalAsyncQueueEntity value, $Res Function(GlobalAsyncQueueEntity) _then) = _$GlobalAsyncQueueEntityCopyWithImpl;
@useResult
$Res call({
 Map<String, Future> running
});




}
/// @nodoc
class _$GlobalAsyncQueueEntityCopyWithImpl<$Res>
    implements $GlobalAsyncQueueEntityCopyWith<$Res> {
  _$GlobalAsyncQueueEntityCopyWithImpl(this._self, this._then);

  final GlobalAsyncQueueEntity _self;
  final $Res Function(GlobalAsyncQueueEntity) _then;

/// Create a copy of GlobalAsyncQueueEntity
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? running = null,}) {
  return _then(_self.copyWith(
running: null == running ? _self.running : running // ignore: cast_nullable_to_non_nullable
as Map<String, Future>,
  ));
}

}


/// @nodoc


class _GlobalAsyncQueueEntity with DiagnosticableTreeMixin implements GlobalAsyncQueueEntity {
  const _GlobalAsyncQueueEntity({final  Map<String, Future> running = const {}}): _running = running;
  

 final  Map<String, Future> _running;
@override@JsonKey() Map<String, Future> get running {
  if (_running is EqualUnmodifiableMapView) return _running;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableMapView(_running);
}


/// Create a copy of GlobalAsyncQueueEntity
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
_$GlobalAsyncQueueEntityCopyWith<_GlobalAsyncQueueEntity> get copyWith => __$GlobalAsyncQueueEntityCopyWithImpl<_GlobalAsyncQueueEntity>(this, _$identity);


@override
void debugFillProperties(DiagnosticPropertiesBuilder properties) {
  properties
    ..add(DiagnosticsProperty('type', 'GlobalAsyncQueueEntity'))
    ..add(DiagnosticsProperty('running', running));
}

@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is _GlobalAsyncQueueEntity&&const DeepCollectionEquality().equals(other._running, _running));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(_running));

@override
String toString({ DiagnosticLevel minLevel = DiagnosticLevel.info }) {
  return 'GlobalAsyncQueueEntity(running: $running)';
}


}

/// @nodoc
abstract mixin class _$GlobalAsyncQueueEntityCopyWith<$Res> implements $GlobalAsyncQueueEntityCopyWith<$Res> {
  factory _$GlobalAsyncQueueEntityCopyWith(_GlobalAsyncQueueEntity value, $Res Function(_GlobalAsyncQueueEntity) _then) = __$GlobalAsyncQueueEntityCopyWithImpl;
@override @useResult
$Res call({
 Map<String, Future> running
});




}
/// @nodoc
class __$GlobalAsyncQueueEntityCopyWithImpl<$Res>
    implements _$GlobalAsyncQueueEntityCopyWith<$Res> {
  __$GlobalAsyncQueueEntityCopyWithImpl(this._self, this._then);

  final _GlobalAsyncQueueEntity _self;
  final $Res Function(_GlobalAsyncQueueEntity) _then;

/// Create a copy of GlobalAsyncQueueEntity
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? running = null,}) {
  return _then(_GlobalAsyncQueueEntity(
running: null == running ? _self._running : running // ignore: cast_nullable_to_non_nullable
as Map<String, Future>,
  ));
}


}

// dart format on
