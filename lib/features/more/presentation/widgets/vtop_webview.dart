import 'package:vitapmate/features/more/presentation/widgets/vtop_webview/vtop_webview_recovery.dart';
import 'package:flutter/material.dart';
import 'package:flutter_inappwebview/flutter_inappwebview.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/logging/app_logger.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/utils/vtop_session_store.dart';
import 'package:vitapmate/core/utils/vtop_webview_store.dart';
import 'package:vitapmate/core/widgets/app_dialog.dart';
import 'package:vitapmate/features/more/presentation/widgets/vtop_webview/vtop_webview_actions.dart';
import 'package:vitapmate/features/more/presentation/widgets/vtop_webview/vtop_webview_body.dart';
import 'package:vitapmate/features/more/presentation/widgets/vtop_webview/vtop_webview_cookie_service.dart';
import 'package:vitapmate/features/more/presentation/widgets/vtop_webview/vtop_webview_loading.dart';

class VtopWebview extends ConsumerStatefulWidget {
  const VtopWebview({this.initialMenuUrl, super.key});
  final String? initialMenuUrl;

  @override
  ConsumerState<VtopWebview> createState() => _VtopWebviewState();
}

class _VtopWebviewState extends ConsumerState<VtopWebview> {
  final _bodyKey = GlobalKey<VtopWebviewBodyState>();
  VtopWebviewSession? _session;
  String? _owner;
  Object? _error;
  bool _preparing = true;
  bool _recovering = false;
  bool _promptOpen = false;
  bool? _dark;
  int _generation = 0;
  int _loadRevision = 0;
  final _recovery = VtopWebviewRecovery();
  String? _pendingMenu;

  @override
  void initState() {
    super.initState();
    _pendingMenu = widget.initialMenuUrl;
    Future.microtask(_prepare);
  }

  bool _current(int generation) => mounted && generation == _generation;

  Future<void> _prepare({bool force = false}) async {
    final generation = ++_generation;
    if (mounted) {
      setState(() {
        _preparing = true;
        _error = null;
      });
    }
    final timer = Stopwatch()..start();
    try {
      await ref.read(settingsProvider.future);
      final user = await ref.read(vtopUserProvider.future);
      if (!_current(generation)) return;
      final username = user.username;
      if (username == null) throw StateError('A VTOP account is required.');
      final client = await ref
          .read(vClientProvider.notifier)
          .ensureLogin(force: force);
      if (!_current(generation)) return;
      final snapshot = createPersistedVtopSessionSnapshot(client: client);
      // ensureLogin already persists the current snapshot.
      if (ref.read(vtopUserProvider).isLoading ||
          ref.read(vtopUserProvider).value?.username != username) {
        return;
      }
      vtopWebviewStore.acquire(username);
      final storeGeneration = vtopWebviewStore.generation;
      bool current() =>
          _current(generation) &&
          storeGeneration == vtopWebviewStore.generation &&
          !ref.read(vtopUserProvider).isLoading &&
          ref.read(vtopUserProvider).value?.username == username;
      final cookieTimer = Stopwatch()..start();
      final session = await vtopWebviewStore.serialize(
        () => loadVtopWebviewSession(
          snapshot: snapshot,
          baseUrl: WebUri('https://vtop.vitap.ac.in'),
          isCurrent: current,
        ),
      );
      if (!current()) return;
      if (session == null) throw StateError('Could not prepare VTOP cookies.');
      AppLogger.instance.info(
        'vtop.webview',
        'prepareMs=${timer.elapsedMilliseconds} cookieSyncMs=${cookieTimer.elapsedMilliseconds}',
      );
      setState(() {
        _owner = username;
        _session = session;
        _loadRevision++;
        _preparing = false;
      });
    } catch (error) {
      if (!_current(generation)) return;
      setState(() {
        _error = error;
        _preparing = false;
      });
    }
  }

  Future<void> _forceLogin() async {
    if (_recovering) return;
    _recovering = true;
    try {
      await _prepare(force: true);
    } finally {
      _recovering = false;
    }
  }

  Future<void> _loginRedirect() async {
    if (_recovering || _promptOpen || !mounted) return;
    if (_recovery.beginAutomaticAttempt()) {
      await _forceLogin();
      return;
    }
    _promptOpen = true;
    try {
      final retry = await showFDialog<bool>(
        context: context,
        builder: (context, style, animation) => AppDialog(
          animation: animation,
          title: const Text('Login expired'),
          body: const Text(
            'VTOP could not restore your session. Try signing in again?',
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
      if (retry == true && mounted) await _forceLogin();
    } finally {
      _promptOpen = false;
    }
  }

  Future<void> _savePreference(
    VtopViewPreference preference,
    bool value,
  ) async {
    try {
      await preference.setValue(value);
    } catch (_) {
      if (mounted) {
        dispToast(
          context,
          'Setting not saved',
          'Your selection applies now, but could not be saved for next time.',
        );
      }
    }
  }

  @override
  void dispose() {
    _generation++;
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final user = ref.watch(vtopUserProvider);
    ref.listen(vtopUserProvider, (previous, next) {
      if (next.hasValue &&
          !next.isLoading &&
          (previous?.isLoading == true ||
              previous?.value?.username != next.value?.username)) {
        _session = null;
        _recovery.authenticatedPageReady();
        _prepare();
      }
    });
    final compact = ref.watch(vtopCompactModeProvider);
    final desktop = ref.watch(vtopDesktopModeProvider);
    final theme = ref.watch(themeProvider);
    _dark ??=
        theme == ThemeMode.dark ||
        (theme == ThemeMode.system &&
            MediaQuery.platformBrightnessOf(context) == Brightness.dark);
    final ready =
        !user.isLoading &&
        _session != null &&
        _owner == user.value?.username &&
        !_preparing &&
        _error == null;
    return FScaffold(
      childPad: false,
      header: FHeader.nested(
        title: const Text('VTOP'),
        prefixes: [
          FHeaderAction.back(onPress: () => GoRouter.of(context).pop()),
        ],
        suffixes: ready
            ? [
                VtopWebviewThemeAction(
                  isDarkMode: _dark!,
                  onToggle: () => setState(() => _dark = !_dark!),
                ),
                VtopWebviewActionsMenu(
                  isCompactMode: compact,
                  isDesktopMode: desktop,
                  onGoTo: (url) {
                    _pendingMenu = url;
                    _bodyKey.currentState?.openMenu(url);
                  },
                  onToggleCompactMode: () => _savePreference(
                    ref.read(vtopCompactModeProvider.notifier),
                    !compact,
                  ),
                  onToggleDesktopMode: () => _savePreference(
                    ref.read(vtopDesktopModeProvider.notifier),
                    !desktop,
                  ),
                  onForceLogin: _forceLogin,
                ),
              ]
            : [],
      ),
      child: ready
          ? VtopWebviewBody(
              key: _bodyKey,
              initialUrl: WebUri('https://vtop.vitap.ac.in/vtop/content?'),
              revision: _loadRevision,
              isCompactMode: compact,
              isDesktopMode: desktop,
              isDarkMode: _dark!,
              session: _session!,
              initialMenuUrl: _pendingMenu,
              onMenuOpened: () => _pendingMenu = null,
              onLoginRedirect: _loginRedirect,
              onAuthenticatedPage: _recovery.authenticatedPageReady,
            )
          : VtopWebviewLoading(
              error: _error,
              onRetry: () => _prepare(),
              reconnecting: _recovering,
            ),
    );
  }
}
