import 'package:vitapmate/observability/log.dart';
import 'package:vitapmate/platform/webview/vtop_webview_store.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/src/api/native_logs.dart';
import 'package:vitapmate/vtop/users/vtop_users.dart';
import 'package:vitapmate/vtop/entity/vtop_user_entity.dart';

part 'vtop_user_provider.g.dart';

@Riverpod(keepAlive: true)
class VtopUser extends _$VtopUser {
  @override
  Future<VtopUserEntity> build() async {
    final user = await ref.read(vtopUsersProvider.notifier).vtopUserDefault();
    await vtopWebviewStore.clearIfDifferent(user?.username);
    Log.regno = (user?.registrationNumber ?? user?.username)?.toUpperCase();
    nativeLogsRegno(regno: Log.regno);
    if (user == null) {
      Log.info('vtop.user', 'no saved account');
      return const VtopUserEntity.unconfigured();
    }
    Log.info('vtop.user', 'account loaded');
    return user;
  }
}
