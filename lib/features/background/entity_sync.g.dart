// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'entity_sync.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

_SyncState _$SyncStateFromJson(Map<String, dynamic> json) =>
    _SyncState(freq: Duration(microseconds: (json['freq'] as num).toInt()));

Map<String, dynamic> _$SyncStateToJson(_SyncState instance) =>
    <String, dynamic>{'freq': instance.freq.inMicroseconds};
