import 'dart:ui';
import 'package:firebase_analytics/firebase_analytics.dart';
import 'package:firebase_crashlytics/firebase_crashlytics.dart';
import 'package:firebase_messaging/firebase_messaging.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/core/router/router.dart';
import 'package:vitapmate/services/firebase_notification.dart';
import 'package:vitapmate/services/update_service.dart';
import 'package:vitapmate/src/frb_generated.dart';
import 'package:firebase_core/firebase_core.dart';
import 'firebase_options.dart';

FirebaseAnalytics analytics = FirebaseAnalytics.instance;
var notifications = NotificationService.instance;
Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await Firebase.initializeApp(options: DefaultFirebaseOptions.currentPlatform);
  FirebaseMessaging.onBackgroundMessage(firebaseMessagingBackgroundHandler);
  await notifications.initialize();
  FlutterError.onError = (errorDetails) {
    FirebaseCrashlytics.instance.recordFlutterFatalError(errorDetails);
  };
  PlatformDispatcher.instance.onError = (error, stack) {
    FirebaseCrashlytics.instance.recordError(error, stack, fatal: true);
    return true;
  };
  await RustLib.init();
  runApp(ProviderScope(child: const MyApp()));
}

class MyApp extends HookConsumerWidget {
  const MyApp({super.key});
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final goRouter = ref.watch(routerProvider);
    useEffect(() {
      Future.microtask(() async {
        await Future.delayed(Duration(milliseconds: 500));
        UpdateService.checkForFlexibleUpdate();
        var value = await FirebaseMessaging.instance.getInitialMessage();
        var newContext = rootNavigatorKey.currentContext;
        if (newContext == null || !newContext.mounted || value == null) return;
        if (value.data["type"] == "chat") {
          GoRouter.of(newContext).pushNamed(Paths.social);
        }
      });
      return null;
    }, []);
    final fTheme = FThemes.zinc.light;
    return MaterialApp.router(
      routeInformationProvider: goRouter.routeInformationProvider,
      routeInformationParser: goRouter.routeInformationParser,
      routerDelegate: goRouter.routerDelegate,
      builder: (context, child) => FTheme(data: fTheme, child: child!),
    );
  }
}
