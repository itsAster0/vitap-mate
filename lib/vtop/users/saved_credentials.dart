import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/vtop/vtop_user_provider.dart';
import 'package:vitapmate/vtop/entity/vtop_user_entity.dart';
import 'package:vitapmate/vtop/users/vtop_users_utils.dart';

/// VTOP turned the saved password down: marks the saved user so the app
/// asks for it again and background refreshes stop. True when it changed
/// anything (a user already marked, or none, is left alone).
Future<bool> rejectSavedCredentials({
  required Future<VtopUserEntity?> Function() load,
  required Future<void> Function(VtopUserEntity user) save,
}) async {
  final user = await load();
  if (user is! ConfiguredVtopUser) return false;
  await save(user.rejectCredentials());
  return true;
}

/// [rejectSavedCredentials] on [container]'s storage, refreshing its user.
Future<void> rejectSavedCredentialsIn(ProviderContainer container) async {
  final users = container.read(vtopusersutilsProvider.notifier);
  if (await rejectSavedCredentials(
    load: users.vtopUserDefault,
    save: users.vtopUserSave,
  )) {
    container.invalidate(vtopUserProvider);
  }
}
