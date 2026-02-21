import 'dart:convert';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:vitapmate/core/utils/entity/vtop_user_entity.dart';

part 'vtop_users_utils.g.dart';

const _defaultUserKey = 'defaultUser';

@riverpod
class Vtopusersutils extends _$Vtopusersutils {
  late FlutterSecureStorage _storage;

  @override
  FlutterSecureStorage build() {
    _storage = const FlutterSecureStorage();
    return _storage;
  }

  Future<VtopUserEntity?> vtopUserDefault() async {
    final defaultUsername = await _storage.read(key: _defaultUserKey);
    if (defaultUsername == null) return null;

    final rawJson = await _storage.read(key: "username_$defaultUsername");
    if (rawJson == null) return null;

    final userJson = jsonDecode(rawJson);
    return VtopUserEntity.fromJson(userJson);
  }

  Future<void> vtopUserSave(VtopUserEntity user) async {
    final jsonString = jsonEncode(user.toJson());
    await _storage.write(key: "username_${user.username!}", value: jsonString);
  }

  Future<void> vtopSetDefault(String username) async {
    await _storage.write(key: _defaultUserKey, value: username);
  }

  Future<void> vtopUserInitialData(VtopUserEntity user) async {
    await vtopSetDefault(user.username!);
    await vtopUserSave(user);
  }

  Future<(List<VtopUserEntity>, String?)> getAllUsers() async {
    var all = await _storage.readAll();
    String? defaultUser;
    List<VtopUserEntity> users = [];
    for (final i in all.entries) {
      try {
        if (i.key.startsWith("username_")) {
          final userJson = jsonDecode(i.value);
          users.add(VtopUserEntity.fromJson(userJson));
        } else if (i.key == _defaultUserKey) {
          defaultUser = i.value;
        }
      } catch (e) {
        continue;
      }
    }
    return (users, defaultUser);
  }

  Future<(String?, String?)> getWifiDetails() async {
    var username = await _storage.read(key: "wifi_username");
    var password = await _storage.read(key: "wifi_password");
    return (username, password);
  }

  Future<void> saveWifiDetails((String, String) data) async {
    await _storage.write(key: "wifi_username", value: data.$1);
    await _storage.write(key: "wifi_password", value: data.$2);
  }

  Future<void> vtopUserDelete(String username) async {
    await _storage.delete(key: "username_$username");
    final defaultUsername = await _storage.read(key: _defaultUserKey);
    if (defaultUsername != username) return;

    final all = await getAllUsers();
    if (all.$1.isEmpty) {
      await _storage.delete(key: _defaultUserKey);
      return;
    }

    final next = all.$1.first.username;
    if (next != null && next.isNotEmpty) {
      await vtopSetDefault(next);
    } else {
      await _storage.delete(key: _defaultUserKey);
    }
  }

  // Future<void> clearAllUsers() async {
  //   await _storage.deleteAll();
  // }
}

@riverpod
Future<(List<VtopUserEntity>, String?)> allUsersProvider(Ref ref) async {
  return ref.watch(vtopusersutilsProvider.notifier).getAllUsers();
}
