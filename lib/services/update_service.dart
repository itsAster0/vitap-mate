import 'dart:developer';
import 'package:in_app_update/in_app_update.dart';

class UpdateService {
  static Future<void> checkForFlexibleUpdate() async {
    try {
      final updateInfo = await InAppUpdate.checkForUpdate();

      if (updateInfo.updateAvailability == UpdateAvailability.updateAvailable) {
        final pri = updateInfo.updatePriority;
        if (pri >= 5) {
          await InAppUpdate.performImmediateUpdate();
        } else if (pri >= 3 && updateInfo.flexibleUpdateAllowed) {
          await InAppUpdate.startFlexibleUpdate();
          await InAppUpdate.completeFlexibleUpdate();
        }
      }
    } catch (e) {
      log("Flexible update check failed: $e");
    }
  }
}
