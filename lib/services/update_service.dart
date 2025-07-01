import 'dart:developer';
import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:in_app_update/in_app_update.dart';
import 'package:vitapmate/core/router/router.dart';

class UpdateService {
  static Future<void> checkForFlexibleUpdate() async {
    try {
      final updateInfo = await InAppUpdate.checkForUpdate();
      final context = rootNavigatorKey.currentContext;
      if (context == null || !context.mounted) return;
      if (updateInfo.updateAvailability == UpdateAvailability.updateAvailable &&
          updateInfo.flexibleUpdateAllowed) {
        await InAppUpdate.startFlexibleUpdate();
        if (context.mounted) {
          showFToast(
            context: context,
            alignment: FToastAlignment.bottomCenter,
            title: const Text('Update Ready'),
            description: const Text(
              "A new update has been downloaded. Please restart the app to complete the installation.",
            ),
            suffixBuilder:
                (context, entry, _) => IntrinsicHeight(
                  child: FButton(
                    style: context.theme.buttonStyles.primary.copyWith(
                      contentStyle: context
                          .theme
                          .buttonStyles
                          .primary
                          .contentStyle
                          .copyWith(
                            padding: const EdgeInsets.symmetric(
                              horizontal: 12,
                              vertical: 7.5,
                            ),
                            textStyle: FWidgetStateMap.all(
                              context.theme.typography.xs.copyWith(
                                color: context.theme.colors.primaryForeground,
                              ),
                            ),
                          ),
                    ),
                    onPress: entry.dismiss,
                    child: const Text('Restart'),
                  ),
                ),
          );
        }
      }
    } catch (e) {
      log("Flexible update check failed: $e");
    }
  }
}
