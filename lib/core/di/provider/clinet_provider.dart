import 'dart:async';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/core/services/service_layer.dart';
import 'package:vitapmate/core/utils/entity/vtop_user_entity.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';
import 'package:vitapmate/core/utils/users/vtop_users_utils.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';
part 'clinet_provider.g.dart';

@Riverpod(keepAlive: true)
class VClient extends _$VClient {
  @override
  Future<VtopClient> build() async {
    final services = await ref.watch(appServicesProvider.future);
    final account = await services.authRepository.loadActiveAccount();
    final password = await services.authRepository.requirePassword();
    if (account == null) {
      throw StateError('No active VTOP account is available.');
    }
    return services.sessionCoordinator.ensureAuthenticated(
      account: account,
      password: password,
    );
  }

  void replaceVClinet(VtopClient vclinet) {
    state = AsyncData(vclinet);
  }

  Future<void> tryLogin({bool force = false}) async {
    final services = await ref.read(appServicesProvider.future);
    final account = await services.authRepository.loadActiveAccount();
    if (account == null) {
      throw StateError('No active VTOP account is available.');
    }
    final password = await services.authRepository.requirePassword();
    VtopUserEntity user = await ref.watch(vtopUserProvider.future);
    if (!user.isValid) throw VtopError.invalidCredentials();
    var gb = await ref.read(gbProvider.future);
    var feature = gb.feature("try-login");
    if (!feature.on || !feature.value) {
      throw FeatureDisabledException("Login is Disabled");
    }

    try {
      final client = await services.sessionCoordinator.ensureAuthenticated(
        account: account,
        password: password,
        force: force,
      );
      state = AsyncData(client);
    } catch (e) {
      if (e == VtopError.invalidCredentials()) {
        await ref
            .read(vtopusersutilsProvider.notifier)
            .vtopUserSave(user.copyWith(isValid: false));
        ref.invalidate(vtopUserProvider);
      }
      if (e is VtopError && e is! VtopError_NetworkError) {
        await services.preferenceStore.writeCookie(null);
        services.sessionCoordinator.clearClient();
      }

      rethrow;
    }
  }
}
