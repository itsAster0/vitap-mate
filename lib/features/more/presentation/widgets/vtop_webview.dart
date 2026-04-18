import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:flutter_inappwebview/flutter_inappwebview.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/utils/vtop_session_store.dart';
import 'package:vitapmate/features/more/presentation/widgets/vtop_webview/vtop_webview_actions.dart';
import 'package:vitapmate/features/more/presentation/widgets/vtop_webview/vtop_webview_body.dart';
import 'package:vitapmate/features/more/presentation/widgets/vtop_webview/vtop_webview_cookie_service.dart';
import 'package:vitapmate/features/more/presentation/widgets/vtop_webview/vtop_webview_loading.dart';
import 'package:vitapmate/features/more/presentation/widgets/vtop_webview/vtop_webview_scripts.dart';

class VtopWebview extends HookConsumerWidget {
  const VtopWebview({this.initialMenuUrl, super.key});
  final String? initialMenuUrl;

  static final _baseUrl = WebUri('https://vtop.vitap.ac.in');
  static final _initialUrl = WebUri('https://vtop.vitap.ac.in/vtop/content?');

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isPreparingSession = useState(true);
    final loading = useState(false);
    final webviewSession = useState<VtopWebviewSession?>(null);
    final webController = useState<InAppWebViewController?>(null);
    final showHeading = useState('VTOP');
    final isCompactMode = useState(true);
    final isDesktopMode = useState(false);
    final themeMode = ref.watch(themeProvider);
    final isDarkMode = useState(
      themeMode == ThemeMode.dark ||
          (themeMode == ThemeMode.system &&
              MediaQuery.platformBrightnessOf(context) == Brightness.dark),
    );

    final setupError = useState<Object?>(null);
    final isLoginRedirectPromptOpen = useRef(false);
    final pendingInitialMenuUrl = useState(initialMenuUrl);
    final forceLoginCounter = useRef(0);
    final redirectForceAttempts = useRef(0);

    Future<bool> prepareSession({int force = 0}) async {
      isPreparingSession.value = true;
      setupError.value = null;

      try {
        final client = await ref.read(vClientProvider.future);
        await ref.read(vClientProvider.notifier).ensureLogin(force: force > 0);
        final user = await ref.read(vtopUserProvider.future);
        final snapshot =
            (await loadStoredVtopSession(user.username!))?.snapshot ??
            createPersistedVtopSessionSnapshot(
              client: client,
              ttl: vtopSessionReuseTtl,
            );
        if (snapshot.cookies?.isEmpty ?? true) {
          throw Exception('Could not prepare VTOP session for webview.');
        }
        await saveStoredVtopSession(snapshot);
        final preparedSession = await loadVtopWebviewSession(
          snapshot: snapshot,
          baseUrl: _baseUrl,
        );
        if (preparedSession == null) {
          throw Exception('Could not prepare VTOP session for webview.');
        }

        webviewSession.value = preparedSession;
        if (force > 0) {
          redirectForceAttempts.value = 0;
        }
        return true;
      } catch (error, _) {
        setupError.value = error;

        if (context.mounted) {
          disCommonToast(context, error);
        }
        return false;
      } finally {
        isPreparingSession.value = false;
      }
    }

    useEffect(() {
      Future.microtask(() async {
        await prepareSession();
      });
      return null;
    }, const []);

    Future<void> forceLogin() async {
      loading.value = true;
      forceLoginCounter.value += 1;
      try {
        final force = forceLoginCounter.value.clamp(1, 2);
        final didPrepare = await prepareSession(force: force);
        if (!didPrepare) return;
      } catch (error) {
        if (context.mounted) {
          disCommonToast(context, error);
        }
      } finally {
        loading.value = false;
      }
    }

    Future<void> promptForceLogin() async {
      if (isLoginRedirectPromptOpen.value || loading.value) return;
      if (redirectForceAttempts.value < 1) {
        redirectForceAttempts.value += 1;
        await forceLogin();
        return;
      }
      isLoginRedirectPromptOpen.value = true;

      try {
        final shouldLoginAgain = await showFDialog<bool>(
          context: context,
          builder: (context, style, animation) => FDialog(
            animation: animation,
            direction: Axis.horizontal,
            title: const Text('Login expired'),
            body: const Text(
              'VTOP sent you back to the login page. Try logging in again?',
            ),
            actions: [
              FButton(
                variant: FButtonVariant.outline,
                onPress: () => Navigator.of(context).pop(false),
                child: const Text('Cancel'),
              ),
              FButton(
                onPress: () => Navigator.of(context).pop(true),
                child: const Text('Try again'),
              ),
            ],
          ),
        );

        if (shouldLoginAgain == true && context.mounted) {
          await forceLogin();
        }
      } finally {
        isLoginRedirectPromptOpen.value = false;
      }
    }

    Future<bool> goTo(String url) async {
      final result = await webController.value?.clickVtopMenuLink(url);
      return result == true;
    }

    useEffect(() {
      webController.value?.setVtopDarkMode(isDarkMode.value);
      return null;
    }, [isDarkMode.value]);

    useEffect(() {
      webController.value?.setVtopDesktopMode(isDesktopMode.value);
      return null;
    }, [isDesktopMode.value]);

    useEffect(() {
      if (isCompactMode.value) {
        webController.value?.setVtopCompactSpacing(padding: 1);
      } else {
        webController.value?.resetVtopSpacing();
      }
      return null;
    }, [isCompactMode.value]);

    if (isPreparingSession.value || webviewSession.value == null) {
      return VtopWebviewLoading(
        error: setupError.value,
        onRetry: () => prepareSession(),
      );
    }

    return FScaffold(
      childPad: false,
      header: FHeader.nested(
        title: Text(
          showHeading.value,
          style: TextStyle(fontSize: context.theme.typography.sm.fontSize),
        ),
        prefixes: [
          FHeaderAction.back(onPress: () => GoRouter.of(context).pop()),
        ],
        suffixes: [
          VtopWebviewThemeAction(
            isDarkMode: isDarkMode.value,
            onToggle: () => isDarkMode.value = !isDarkMode.value,
          ),
          const SizedBox(width: 10),
          VtopWebviewActionsMenu(
            isCompactMode: isCompactMode.value,
            isDesktopMode: isDesktopMode.value,
            onGoTo: (url) {
              pendingInitialMenuUrl.value = null;
              goTo(url);
            },
            onToggleCompactMode: () =>
                isCompactMode.value = !isCompactMode.value,
            onToggleDesktopMode: () =>
                isDesktopMode.value = !isDesktopMode.value,
            onForceLogin: forceLogin,
          ),
        ],
      ),
      child: VtopWebviewBody(
        initialUrl: _initialUrl,
        isCompactMode: isCompactMode.value,
        isDarkMode: isDarkMode.value,
        loading: loading.value,
        session: webviewSession.value,
        onLoadingChanged: (value) => loading.value = value,
        onLoginRedirect: promptForceLogin,
        onPageReady: (controller) async {
          final target = pendingInitialMenuUrl.value;
          if (target == null) return;

          final didOpen = await goTo(target);
          if (didOpen == true) {
            pendingInitialMenuUrl.value = null;
          }
        },
        onWebViewCreated: (controller) async {
          webController.value = controller;
          controller.setVtopDarkMode(isDarkMode.value);
          controller.setVtopDesktopMode(isDesktopMode.value);
          if (isCompactMode.value) {
            controller.setVtopCompactSpacing(padding: 1);
          }
        },
      ),
    );
  }
}
