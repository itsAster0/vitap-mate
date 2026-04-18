import 'dart:convert';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:vitapmate/core/utils/entity/vtop_user_entity.dart';

part 'vtop_users_utils.g.dart';

const _singleUserKey = 'vtopUser';
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
    final singleRaw = await _storage.read(key: _singleUserKey);
    if (singleRaw != null && singleRaw.isNotEmpty) {
      try {
        return VtopUserEntity.fromJson(jsonDecode(singleRaw));
      } catch (_) {
        await _storage.delete(key: _singleUserKey);
      }
    }

    return _migrateLegacyUsersToSingleUser();
  }

  Future<void> vtopUserSave(VtopUserEntity user) async {
    final jsonString = jsonEncode(user.toJson());
    await _storage.write(key: _singleUserKey, value: jsonString);
    await _cleanupLegacyUserKeys();
  }

  Future<void> vtopSetDefault(String username) async {
    // Deprecated in single-user mode; kept for backward compatibility.
    await _cleanupLegacyUserKeys();
  }

  Future<void> vtopUserInitialData(VtopUserEntity user) async {
    await vtopSetDefault(user.username!);
    await vtopUserSave(user);
  }

  Future<(List<VtopUserEntity>, String?)> getAllUsers() async {
    final user = await vtopUserDefault();
    if (user == null) return (<VtopUserEntity>[], null);
    return ([user], user.username);
  }

  Future<void> vtopUserDelete(String username) async {
    final existing = await vtopUserDefault();
    if (existing?.username == username) {
      await _storage.delete(key: _singleUserKey);
    }
    await _cleanupLegacyUserKeys();
  }

  Future<VtopUserEntity?> _migrateLegacyUsersToSingleUser() async {
    final all = await _storage.readAll();
    final defaultUsername = all[_defaultUserKey];

    String? raw;
    if (defaultUsername != null) {
      raw = all["username_$defaultUsername"];
    }
    raw ??= all.entries
        .firstWhere(
          (entry) => entry.key.startsWith("username_"),
          orElse: () => const MapEntry('', ''),
        )
        .value;
    if (raw.isEmpty) {
      await _cleanupLegacyUserKeys();
      return null;
    }

    try {
      final user = VtopUserEntity.fromJson(jsonDecode(raw));
      await _storage.write(
        key: _singleUserKey,
        value: jsonEncode(user.toJson()),
      );
      await _cleanupLegacyUserKeys();
      return user;
    } catch (_) {
      await _cleanupLegacyUserKeys();
      return null;
    }
  }

  Future<void> _cleanupLegacyUserKeys() async {
    final all = await _storage.readAll();
    for (final key in all.keys) {
      if (key == _defaultUserKey || key.startsWith("username_")) {
        await _storage.delete(key: key);
      }
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
