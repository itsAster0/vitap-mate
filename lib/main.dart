import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/router/router.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/widgets/vtop_otp_overlay.dart';
import 'package:vitapmate/features/background/controller.dart';
import 'package:vitapmate/features/background/sync.dart';

import 'package:vitapmate/services/update_service.dart';
import 'package:vitapmate/src/frb_generated.dart';
import 'package:workmanager/workmanager.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  Workmanager().initialize(callbackDispatcher);

  await RustLib.init();
  fileDownloaderConfig();
  runApp(ProviderScope(child: const MyApp()));
}

class MyApp extends HookConsumerWidget {
  const MyApp({super.key});
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final goRouter = ref.watch(routerProvider);
    useEffect(() {
      Future(() async {
        ref.read(backgroundSyncProvider);
        await Future.delayed(Duration(milliseconds: 500));

        UpdateService.checkForFlexibleUpdate();
      });
      return null;
    }, []);

    final fTheme = ref.watch(fThemeProvider);

    return MaterialApp.router(
      routeInformationProvider: goRouter.routeInformationProvider,
      routeInformationParser: goRouter.routeInformationParser,
      routerDelegate: goRouter.routerDelegate,
      builder: (context, child) => FTheme(
        data: fTheme,
        child: FToaster(child: VtopOtpOverlay(child: child!)),
      ),
    );
  }
}
