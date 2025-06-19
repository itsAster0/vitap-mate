import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:flutter/foundation.dart';
part 'vtop_user_entity.freezed.dart';
part 'vtop_user_entity.g.dart';

@freezed
sealed class VtopUserEntity with _$VtopUserEntity {
  factory VtopUserEntity({
    required String? username,
    required String? password,
    required String? semid,
    @Default(false) bool isValid,
  }) = _VtopUserEntity;
  factory VtopUserEntity.fromJson(Map<String, dynamic> json) =>
      _$VtopUserEntityFromJson(json);
}
