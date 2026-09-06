import 'dart:developer';

import 'package:vitapmate/core/utils/vtop_webview_store.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/utils/users/vtop_users_utils.dart';
import 'package:vitapmate/core/utils/entity/vtop_user_entity.dart';

part 'vtop_user_provider.g.dart';

@Riverpod(keepAlive: true)
class VtopUser extends _$VtopUser {
  @override
  Future<VtopUserEntity> build() async {
    log("VtopUser build start");
    var user = await ref
        .read(vtopusersutilsProvider.notifier)
        .vtopUserDefault();
    log("VtopUser build sucessfull $user");
    await vtopWebviewStore.clearIfDifferent(user?.username);
    if (user == null) {
      return const VtopUserEntity.unconfigured();
    }
    return user;
  }
}
