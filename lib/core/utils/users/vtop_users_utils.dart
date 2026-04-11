import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:vitapmate/core/models/account.dart';
import 'package:vitapmate/core/services/storage.dart';
import 'package:vitapmate/core/utils/entity/vtop_user_entity.dart';

part 'vtop_users_utils.g.dart';

@riverpod
class Vtopusersutils extends _$Vtopusersutils {
  late FlutterSecureStorage _storage;
  late SecureAccountStore _accountStore;

  @override
  FlutterSecureStorage build() {
    _storage = const FlutterSecureStorage();
    _accountStore = SecureAccountStore(_storage);
    return _storage;
  }

  Future<VtopUserEntity?> vtopUserDefault() async {
    final account = await _accountStore.readActiveAccount();
    final password = await _accountStore.readPassword();
    if (account == null || password == null) return null;
    return VtopUserEntity(
      username: account.identity.registrationNumber,
      password: password,
      semid: account.identity.selectedSemesterId,
      isValid: account.isValid,
    );
  }

  Future<void> vtopUserSave(VtopUserEntity user) async {
    final existing = await _accountStore.readActiveAccount();
    await _accountStore.writeActiveAccount(
      account: ActiveAccount(
        identity: StudentIdentity(
          email: existing?.identity.email,
          registrationNumber: user.username ?? '',
          selectedSemesterId: user.semid ?? '',
        ),
        isValid: user.isValid,
      ),
      password: user.password ?? '',
    );
  }

  Future<void> vtopSetDefault(String username) async {}

  Future<void> vtopUserInitialData(VtopUserEntity user) async {
    await vtopUserSave(user);
  }

  Future<(List<VtopUserEntity>, String?)> getAllUsers() async {
    final user = await vtopUserDefault();
    return (
      user == null ? <VtopUserEntity>[] : <VtopUserEntity>[user],
      user?.username,
    );
  }

  Future<void> vtopUserDelete(String username) async {
    final user = await vtopUserDefault();
    if (user?.username == username) {
      await _accountStore.clear();
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
