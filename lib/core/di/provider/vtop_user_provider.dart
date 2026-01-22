import 'dart:developer';

import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/utils/users/vtop_users_utils.dart';
import 'package:vitapmate/core/utils/entity/vtop_user_entity.dart';

part 'vtop_user_provider.g.dart';

@Riverpod(keepAlive: true)
class VtopUser extends _$VtopUser {
  @override
  Future<VtopUserEntity> build() async {
    log("VtopUser build start");
    var user =
        await ref.read(vtopusersutilsProvider.notifier).vtopUserDefault();
    log("VtopUser build sucessfull $user");

    return user!;
  }
}
