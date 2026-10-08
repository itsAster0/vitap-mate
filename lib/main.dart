import 'dart:async';
import 'package:vitapmate/core/utils/bridge_otp_prompt.dart';
import 'package:firebase_messaging/firebase_messaging.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/core/router/router.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/utils/fcm_cookie_bridge_service.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/features/docs/data/download_to_docs.dart';
import 'package:vitapmate/core/widgets/screen_refresh.dart';
import 'package:vitapmate/core/widgets/vtop_otp_overlay.dart';
import 'package:vitapmate/features/background/controller.dart';
import 'package:vitapmate/features/background/sync.dart';
import 'package:vitapmate/services/update_service.dart';
import 'package:vitapmate/src/frb_generated.dart';
import 'package:workmanager/workmanager.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  // Independent, so start both rather than waiting on one then the other.
  final (firebaseReady, _) = await (ensureFirebaseReady(), RustLib.init()).wait;
  if (firebaseReady) {
    FirebaseMessaging.onBackgroundMessage(vtopCookieBridgeBackgroundHandler);
  }
  Workmanager().initialize(callbackDispatcher);

  fileDownloaderConfig();

  runApp(const ProviderScope(child: MyApp()));
}

class MyApp extends HookConsumerWidget {
  const MyApp({super.key});
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final goRouter = ref.watch(routerProvider);
    // Agents (MCP) default to the semester picked here.
    ref.listen(vtopUserProvider.select((user) => user.value?.semid), (
      _,
      semid,
    ) {
      if (semid != null) unawaited(syncBridgeSemester(semid));
    });
    useEffect(() {
      // A tap on the OTP notification, or a login left waiting for its OTP:
      // finish it with the app's OTP prompt.
      final container = ProviderScope.containerOf(context, listen: false);
      void answer() => unawaited(answerPendingBridgeOtpInApp(container));
      bridgeOtpTapSignal.addListener(answer);
      answer();
      return () => bridgeOtpTapSignal.removeListener(answer);
    }, const []);
    useEffect(() {
      Future(() async {
        startVtopCookieBridgeListener();
        ref.read(backgroundSyncProvider);
        try {
          await resumePendingDocsDownloads();
        } catch (_) {
          // A pending local copy can be retried on the next launch.
        }
        await Future.delayed(Duration(milliseconds: 500));
        UpdateService.checkForFlexibleUpdate();
      });
      return null;
    }, []);

    final themeMode = ref.watch(themeProvider);

    return MaterialApp.router(
      theme: _materialLight,
      darkTheme: _materialDark,
      themeMode: themeMode,
      routeInformationProvider: goRouter.routeInformationProvider,
      routeInformationParser: goRouter.routeInformationParser,
      routerDelegate: goRouter.routerDelegate,
      // Follow the brightness Material resolved, so "System" tracks the
      // phone's setting live.
      builder: (context, child) => FTheme(
        data: Theme.of(context).brightness == Brightness.dark
            ? appDarkTheme
            : appLightTheme,
        child: FToaster(
          child: VtopOtpOverlay(
            child: Stack(children: [child!, const ScreenRefreshButton()]),
          ),
        ),
      ),
    );
  }
}

final _materialLight = appLightTheme.toApproximateMaterialTheme();
final _materialDark = appDarkTheme.toApproximateMaterialTheme();
