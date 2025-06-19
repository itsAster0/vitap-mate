// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'vtop_user_entity.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

_VtopUserEntity _$VtopUserEntityFromJson(Map<String, dynamic> json) =>
    _VtopUserEntity(
      username: json['username'] as String?,
      password: json['password'] as String?,
      semid: json['semid'] as String?,
      isValid: json['isValid'] as bool? ?? false,
    );

Map<String, dynamic> _$VtopUserEntityToJson(_VtopUserEntity instance) =>
    <String, dynamic>{
      'username': instance.username,
      'password': instance.password,
      'semid': instance.semid,
      'isValid': instance.isValid,
    };
