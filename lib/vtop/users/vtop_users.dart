import 'dart:convert';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/vtop/entity/vtop_user_entity.dart';

part 'vtop_users.g.dart';

const _singleUserKey = 'vtopUser';
const _defaultUserKey = 'defaultUser';

@riverpod
class VtopUsers extends _$VtopUsers {
  static const _storage = FlutterSecureStorage();

  @override
  FlutterSecureStorage build() => _storage;

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
    await _storage.write(key: _singleUserKey, value: jsonEncode(user.toJson()));
    await _cleanupLegacyUserKeys();
  }

  Future<void> vtopUserInitialData(VtopUserEntity user) async {
    if (user is! StoredVtopUser) {
      throw StateError('Cannot persist an unconfigured VTOP user.');
    }
    await _cleanupLegacyUserKeys();
    await vtopUserSave(user);
  }

  Future<void> vtopUserDelete(String username) async {
    final existing = await vtopUserDefault();
    if (existing?.username == username) {
      await _storage.delete(key: _singleUserKey);
    }
    await _cleanupLegacyUserKeys();
  }

  /// Reads the account saved by versions that kept several users.
  Future<VtopUserEntity?> _migrateLegacyUsersToSingleUser() async {
    final all = await _storage.readAll();
    final raw =
        switch (all[_defaultUserKey]) {
          final name? => all['username_$name'],
          null => null,
        } ??
        all.entries
            .where((entry) => entry.key.startsWith('username_'))
            .firstOrNull
            ?.value ??
        '';
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
      if (key == _defaultUserKey || key.startsWith('username_')) {
        await _storage.delete(key: key);
      }
    }
  }
}
