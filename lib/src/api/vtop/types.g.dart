// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'types.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

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
