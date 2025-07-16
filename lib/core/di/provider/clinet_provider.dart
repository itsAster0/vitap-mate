import 'dart:async';
import 'dart:developer';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/core/utils/entity/vtop_user_entity.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';
import 'package:vitapmate/core/utils/users/vtop_users_utils.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';
part 'clinet_provider.g.dart';

@Riverpod(keepAlive: true)
class VClient extends _$VClient {
  @override
  Future<VtopClient> build() async {
    String? username = await ref.watch(
      vtopUserProvider.selectAsync((user) => user.username),
    );
    String? password = await ref.watch(
      vtopUserProvider.selectAsync((user) => user.password),
    );
    log("Vtop client build sucessfull");
    // Future.microtask(() async {
    //   try {
    //     await tryLogin();
    //   } catch (e) {
    //     log("$e");
    //   }
    // });
    return getVtopClient(username: username!, password: password!);
  }

  void replaceVClinet(VtopClient vclinet) {
    state = AsyncData(vclinet);
  }

  Future<void> tryLogin() async {
    VtopClient client = await future;
    VtopUserEntity user = await ref.watch(vtopUserProvider.future);
    if (await fetchIsAuth(client: client)) return;
    if (!user.isValid) throw VtopError.invalidCredentials();
    var gb = await ref.read(gbProvider.future);
    var feature = gb.feature("try-login");
    if (!feature.on || !feature.value) {
      throw FeatureDisabledException("Login is Disabled");
    }
    log("login try");
    try {
      await ref
          .read(globalAsyncQueueProvider.notifier)
          .run(
            "vtop_login_${user.username}",
            () => vtopClientLogin(client: client),
          );
      //state = AsyncData(client);
    } catch (e) {
      if (e == VtopError.invalidCredentials()) {
        log("password change");
        await ref
            .read(vtopusersutilsProvider.notifier)
            .vtopUserSave(user.copyWith(isValid: false));
        ref.invalidate(vtopUserProvider);
      }
      rethrow;
    }
  }
}
