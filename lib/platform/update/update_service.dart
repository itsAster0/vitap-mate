import 'package:vitapmate/observability/log.dart';
import 'package:vitapmate/vtop/http/app_http_client.dart';
import 'package:in_app_update/in_app_update.dart';

class UpdateService {
  static Future<void> checkForFlexibleUpdate() async {
    try {
      final updateInfo = await loggedNetCall(
        'Play update checkForUpdate',
        InAppUpdate.checkForUpdate,
      );

      if (updateInfo.updateAvailability == UpdateAvailability.updateAvailable) {
        final pri = updateInfo.updatePriority;
        if (pri >= 5) {
          await loggedNetCall(
            'Play update performImmediateUpdate',
            InAppUpdate.performImmediateUpdate,
          );
        } else if (pri >= 3 && updateInfo.flexibleUpdateAllowed) {
          await loggedNetCall(
            'Play update startFlexibleUpdate',
            InAppUpdate.startFlexibleUpdate,
          );
          await loggedNetCall(
            'Play update completeFlexibleUpdate',
            InAppUpdate.completeFlexibleUpdate,
          );
        }
      }
    } catch (e) {
      Log.info('platform.update', "Flexible update check failed: $e");
    }
  }
}
