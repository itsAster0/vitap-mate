import 'dart:async';

import 'package:vitapmate/features/vtop_browser/vtop_webview/vtop_webview_recovery.dart';
import 'package:flutter/material.dart';
import 'package:flutter_inappwebview/flutter_inappwebview.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/vtop/clinet_provider.dart';
import 'package:vitapmate/vtop/vtop_user_provider.dart';
import 'package:vitapmate/observability/log.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/app/theme_provider.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/vtop/vtop_session_store.dart';
import 'package:vitapmate/platform/webview/vtop_webview_pages.dart';
import 'package:vitapmate/platform/webview/vtop_webview_store.dart';
import 'package:vitapmate/ui/app_dialog.dart';
import 'package:vitapmate/features/vtop_browser/vtop_webview/vtop_webview_body.dart';
import 'package:vitapmate/features/vtop_browser/vtop_webview/vtop_webview_cookie_service.dart';
import 'package:vitapmate/features/vtop_browser/vtop_webview/vtop_webview_loading.dart';
import 'package:vitapmate/features/vtop_browser/vtop_webview/vtop_webview_search.dart';

class VtopWebview extends ConsumerStatefulWidget {
  const VtopWebview({this.initialMenuUrl, super.key});
  final String? initialMenuUrl;

  @override
  ConsumerState<VtopWebview> createState() => _VtopWebviewState();
}

class _VtopWebviewState extends ConsumerState<VtopWebview> {
  var _bodyKey = GlobalKey<VtopWebviewBodyState>();
  int? _bodyGeneration;
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
  VtopPage? _page;
  DateTime? _lastViewLoss;

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
      Log.info(
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
    if (_recovery.beginAutomaticAttempt()) return _forceLogin();
    _promptOpen = true;
    try {
      final retry = await confirm(
        context,
        title: 'Login expired',
        body: 'VTOP could not restore your session. Try signing in again?',
        action: 'Try again',
      );
      if (retry && mounted) await _forceLogin();
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

  void _pageChanged(VtopPage? page) {
    if (!mounted) return;
    if (page?.url != _page?.url || page?.title != _page?.title) {
      setState(() => _page = page);
    }
    if (page != null && _owner != null) {
      unawaited(ref.read(vtopRecentPagesProvider.notifier).add(page));
    }
  }

  /// Replaces a crashed view with a fresh one on the same page. A second
  /// crash soon after stops and asks, so a page that keeps crashing
  /// cannot loop.
  void _viewLost() {
    if (!mounted) return;
    final now = DateTime.now();
    final repeated =
        _lastViewLoss != null &&
        now.difference(_lastViewLoss!) < const Duration(seconds: 30);
    _lastViewLoss = now;
    vtopWebviewStore.replaceKeptView();
    setState(() {
      _pendingMenu = _page?.url;
      _page = null;
      _bodyKey = GlobalKey<VtopWebviewBodyState>();
      if (repeated) _error = StateError('VTOP stopped responding.');
    });
    if (!repeated) {
      dispToast(
        context,
        'VTOP reopened',
        'The page stopped responding, so it was loaded again.',
      );
    }
  }

  void _openMenu(String url) {
    _pendingMenu = url;
    _bodyKey.currentState?.openMenu(url);
  }

  Future<void> _search() async {
    final body = _bodyKey.currentState;
    if (body == null || _owner == null) return;
    await showVtopPageSearch(
      context,
      pages: body.pages(),
      recent: ref.read(vtopRecentPagesProvider),
      onSelect: (page) => _openMenu(page.url),
    );
  }

  Future<void> _back(bool ready) async {
    final handled =
        ready && (await _bodyKey.currentState?.handleBack() ?? false);
    if (!handled && mounted) context.pop();
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
    final username = user.isLoading ? null : user.value?.username;
    final ready =
        username != null &&
        _session != null &&
        _owner == username &&
        !_preparing &&
        _error == null;
    // A cleared store invalidates the old view; start a fresh one.
    if (_bodyGeneration != vtopWebviewStore.generation) {
      _bodyGeneration = vtopWebviewStore.generation;
      _bodyKey = GlobalKey<VtopWebviewBodyState>();
    }
    final loading = VtopWebviewLoading(
      error: _error,
      onRetry: () => _prepare(),
      reconnecting: _recovering,
    );
    // System Back steps through VTOP first; the header arrow always leaves.
    return PopScope(
      canPop: false,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop) _back(ready);
      },
      child: FScaffold(
        childPad: false,
        header: FHeader.nested(
          title: Text(_page?.title ?? 'VTOP', maxLines: 1, overflow: .ellipsis),
          prefixes: [FHeaderAction.back(onPress: () => context.pop())],
          suffixes: [
            if (ready) ...[
              FHeaderAction(
                icon: const Icon(FLucideIcons.search),
                semanticsLabel: 'Search VTOP pages',
                onPress: _search,
              ),
              FPopoverMenu(
                autofocus: true,
                menuAnchor: .topRight,
                childAnchor: .bottomRight,
                menu: [
                  FItemGroup(
                    children: [
                      FItem(
                        prefix: const Icon(FLucideIcons.search),
                        title: const Text('Search pages'),
                        onPress: _search,
                      ),
                      FItem(
                        prefix: const Icon(FLucideIcons.house),
                        title: const Text('VTOP Home'),
                        onPress: () => _bodyKey.currentState?.openHome(),
                      ),
                    ],
                  ),
                  FItemGroup(
                    children: [
                      for (final (value, label, onPress) in [
                        (
                          _dark!,
                          'Dark Pages',
                          () => setState(() => _dark = !_dark!),
                        ),
                        (
                          compact,
                          'Compact View',
                          () => _savePreference(
                            ref.read(vtopCompactModeProvider.notifier),
                            !compact,
                          ),
                        ),
                        (
                          desktop,
                          'Desktop Mode',
                          () => _savePreference(
                            ref.read(vtopDesktopModeProvider.notifier),
                            !desktop,
                          ),
                        ),
                      ])
                        FItem(
                          prefix: FCheckbox(value: value),
                          title: Text(label),
                          onPress: onPress,
                        ),
                    ],
                  ),
                  FItemGroup(
                    children: [
                      FItem(
                        prefix: const Icon(FLucideIcons.logIn),
                        title: const Text('Force Login'),
                        onPress: _forceLogin,
                      ),
                    ],
                  ),
                ],
                builder: (_, controller, _) => FHeaderAction(
                  icon: const Icon(FLucideIcons.ellipsis),
                  onPress: controller.toggle,
                ),
              ),
            ],
          ],
        ),
        // The web view starts while login is prepared, so its engine start-up
        // overlaps the network work; the loading screen covers it until then.
        child: username == null
            ? loading
            : Stack(
                fit: .expand,
                children: [
                  VtopWebviewBody(
                    key: _bodyKey,
                    initialUrl: WebUri(
                      'https://vtop.vitap.ac.in/vtop/content?',
                    ),
                    revision: _loadRevision,
                    isCompactMode: compact,
                    isDesktopMode: desktop,
                    isDarkMode: _dark!,
                    session: ready ? _session : null,
                    initialMenuUrl: _pendingMenu,
                    onMenuOpened: () => _pendingMenu = null,
                    onPageChanged: _pageChanged,
                    onViewLost: _viewLost,
                    onLoginRedirect: _loginRedirect,
                    onAuthenticatedPage: _recovery.authenticatedPageReady,
                  ),
                  if (!ready) loading,
                ],
              ),
      ),
    );
  }
}
