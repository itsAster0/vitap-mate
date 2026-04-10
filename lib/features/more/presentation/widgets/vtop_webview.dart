import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:flutter_inappwebview/flutter_inappwebview.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/features/more/presentation/widgets/vtop_webview/vtop_webview_actions.dart';
import 'package:vitapmate/features/more/presentation/widgets/vtop_webview/vtop_webview_body.dart';
import 'package:vitapmate/features/more/presentation/widgets/vtop_webview/vtop_webview_cookie_service.dart';
import 'package:vitapmate/features/more/presentation/widgets/vtop_webview/vtop_webview_loading.dart';
import 'package:vitapmate/features/more/presentation/widgets/vtop_webview/vtop_webview_scripts.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';

class VtopWebview extends HookConsumerWidget {
  const VtopWebview({super.key});

  static final _baseUrl = WebUri('https://vtop.vitap.ac.in');
  static final _initialUrl = WebUri('https://vtop.vitap.ac.in/vtop/content?');

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isPreparingSession = useState(true);
    final loading = useState(false);
    final cookie = useState<VtopWebviewCookie?>(null);
    final webController = useState<InAppWebViewController?>(null);
    final showHeading = useState('VTOP');
    final isCompactMode = useState(true);
    final isDesktopMode = useState(false);
    final isDarkMode = useState(ref.read(themeControllerProvider).index == 2);
    final setupError = useState<Object?>(null);

    Future<bool> prepareSession({bool force = false}) async {
      isPreparingSession.value = true;
      setupError.value = null;

      try {
        await ref.read(vClientProvider.notifier).tryLogin(force: force);
        final client = await ref.read(vClientProvider.future);
        if (!await fetchIsAuth(client: client)) {
          throw Exception('Unable to authenticate with VTOP.');
        }

        final preparedCookie = await loadVtopWebviewCookie(
          ref: ref,
          baseUrl: _baseUrl,
        );
        if (preparedCookie == null) {
          throw Exception('Unable to prepare VTOP session.');
        }

        cookie.value = preparedCookie;
        isPreparingSession.value = false;
        return true;
      } catch (error) {
        setupError.value = error;
        if (context.mounted) {
          disCommonToast(context, error);
        }
        return false;
      }
    }

    Future<void> reloadInitialPage() async {
      final controller = webController.value;
      if (controller == null) return;

      await controller.loadUrl(urlRequest: URLRequest(url: _initialUrl));
    }

    Future<void> forceLogin() async {
      loading.value = true;
      try {
        final didPrepareSession = await prepareSession(force: true);
        if (didPrepareSession) {
          await reloadInitialPage();
        }
      } catch (error) {
        if (context.mounted) {
          disCommonToast(context, error);
        }
      } finally {
        loading.value = false;
      }
    }

    void goTo(String url) {
      webController.value?.clickVtopMenuLink(url);
    }

    useEffect(() {
      Future(prepareSession);
      return null;
    }, const []);

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

    if (isPreparingSession.value) {
      return VtopWebviewLoading(
        error: setupError.value,
        onRetry: prepareSession,
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
            onGoTo: goTo,
            onToggleCompactMode:
                () => isCompactMode.value = !isCompactMode.value,
            onToggleDesktopMode:
                () => isDesktopMode.value = !isDesktopMode.value,
            onForceLogin: forceLogin,
          ),
        ],
      ),
      child: VtopWebviewBody(
        initialUrl: _initialUrl,
        isCompactMode: isCompactMode.value,
        isDarkMode: isDarkMode.value,
        loading: loading.value,
        cookie: cookie.value,
        onLoadingChanged: (value) => loading.value = value,
        onWebViewCreated: (controller) {
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
