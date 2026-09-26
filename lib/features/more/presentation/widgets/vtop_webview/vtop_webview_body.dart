import 'dart:async';
import 'dart:collection';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_inappwebview/flutter_inappwebview.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/core/logging/app_logger.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/utils/vtop_webview_pages.dart';
import 'package:vitapmate/core/utils/vtop_webview_store.dart';
import 'package:vitapmate/features/more/presentation/widgets/vtop_webview/vtop_webview_cookie_service.dart';
import 'package:vitapmate/features/more/presentation/widgets/vtop_webview/vtop_webview_scripts.dart';

bool _isOutingDownload(Iterable<String?> values) => values.any((value) {
  final path = value?.toLowerCase() ?? '';
  return path.contains('studentweekendouting') ||
      path.contains('studentgeneralouting');
});

bool _isDownloadAction(String value) =>
    value.toLowerCase().contains('/download');

class VtopWebviewBody extends StatefulWidget {
  const VtopWebviewBody({
    required this.initialUrl,
    required this.revision,
    required this.isCompactMode,
    required this.isDesktopMode,
    required this.isDarkMode,
    required this.session,
    required this.onLoginRedirect,
    required this.onAuthenticatedPage,
    required this.onMenuOpened,
    required this.onPageChanged,
    required this.onViewLost,
    this.initialMenuUrl,
    super.key,
  });
  final WebUri initialUrl;
  final int revision;
  final bool isCompactMode;
  final bool isDesktopMode;
  final bool isDarkMode;

  /// Null while the login is still being prepared; the view warms up meanwhile.
  final VtopWebviewSession? session;
  final String? initialMenuUrl;
  final VoidCallback onLoginRedirect;
  final VoidCallback onAuthenticatedPage;
  final VoidCallback onMenuOpened;

  /// The page now shown, or null for VTOP's Home; used for the header title
  /// and the recent pages list.
  final ValueChanged<VtopPage?> onPageChanged;

  /// The page's process died; this view can no longer be used.
  final VoidCallback onViewLost;

  @override
  State<VtopWebviewBody> createState() => VtopWebviewBodyState();
}

class VtopWebviewBodyState extends State<VtopWebviewBody> {
  InAppWebViewController? _controller;
  late final int _storeGeneration;
  bool _pageLoading = true;
  bool _hasContent = false;
  bool _slow = false;
  // Progress ticks rebuild only the bar, not the web view's stack.
  final _progress = ValueNotifier<int>(0);
  int _requests = 0;
  String? _error;
  String? _document;
  String? _retiredDocument;
  bool _acceptActivity = false;
  final _instanceId = DateTime.now().microsecondsSinceEpoch;
  int _homeRevision = 0;
  String? _menuRequest;
  String? _pendingMenu;
  String? _currentMenu;
  int _menuSequence = 0;
  int _navigation = 0;
  Timer? _slowTimer;
  Timer? _menuTimer;
  final _opening = Stopwatch()..start();
  final _pageTimer = Stopwatch();
  bool _loggedFirstPage = false;
  WebUri? _navigationUrl;
  bool _reattached = false;
  // The page being reopened by Back; its click must not be recorded again.
  String? _restoring;
  Future<void> _styleWrites = Future.value();

  bool get _valid => mounted && _storeGeneration == vtopWebviewStore.generation;
  bool get _busy => _pageLoading || _requests > 0 || _menuRequest != null;

  String get _preferences => vtopPreferencesScript(
    dark: widget.isDarkMode,
    compact: widget.isCompactMode,
    desktop: widget.isDesktopMode,
  );
  UserScript get _styleScript => UserScript(
    groupName: 'vtop-preferences',
    source: _preferences,
    injectionTime: UserScriptInjectionTime.AT_DOCUMENT_START,
    forMainFrameOnly: true,
  );

  @override
  void initState() {
    super.initState();
    _storeGeneration = vtopWebviewStore.generation;
    _pendingMenu = widget.initialMenuUrl;
    _currentMenu = widget.initialMenuUrl;
    _watchSlowLoad();
  }

  void _watchSlowLoad() {
    if (!_busy) {
      _slowTimer?.cancel();
      _slowTimer = null;
      _slow = false;
    } else {
      _slowTimer ??= Timer(const Duration(seconds: 15), () {
        if (_valid && _busy) setState(() => _slow = true);
      });
    }
  }

  Future<void> _updateStyles() {
    final source = _preferences;
    final script = _styleScript;
    final controller = _controller;
    final update = _styleWrites.then((_) async {
      if (!_valid || controller == null) return;
      await controller.removeUserScriptsByGroupName(
        groupName: 'vtop-preferences',
      );
      await controller.addUserScript(userScript: script);
      if (!_valid) return;
      await controller.evaluateJavascript(source: source);
    });
    _styleWrites = update.catchError((Object _) {
      if (mounted && _valid) {
        dispToast(
          context,
          'View could not update',
          'Try changing the view setting again.',
        );
      }
    });
    return _styleWrites;
  }

  @override
  void didUpdateWidget(covariant VtopWebviewBody oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.isCompactMode != widget.isCompactMode ||
        oldWidget.isDesktopMode != widget.isDesktopMode ||
        oldWidget.isDarkMode != widget.isDarkMode) {
      unawaited(_updateStyles());
    }
    // Deferred: opening reports the page to the parent, which is building.
    if (oldWidget.revision != widget.revision) {
      unawaited(Future.microtask(_open));
    }
  }

  /// A reattached page is reused once if it was opened with these cookies;
  /// any later session change starts again from Home.
  Future<void> _open() async {
    final controller = _controller;
    final session = widget.session;
    if (!_valid || controller == null || session == null) return;
    final reuse =
        _reattached && vtopWebviewStore.keptCookies == session.cookieHeader;
    _reattached = false;
    if (!reuse) return _home();
    try {
      await _resume(controller);
    } catch (_) {
      if (_valid) await _home();
    }
  }

  Future<void> _resume(InAppWebViewController controller) async {
    final url = await controller.getUrl();
    if (!_valid) return;
    if (url?.scheme != 'https' ||
        url?.host != 'vtop.vitap.ac.in' ||
        url!.path.startsWith('/vtop/login')) {
      return _home();
    }
    final document = await controller.evaluateJavascript(
      source: 'window.__mateActivity',
    );
    if (!_valid) return;
    _document = document is String ? document : null;
    _acceptActivity = true;
    _navigationUrl = url;
    if (_pendingMenu != null) vtopWebviewStore.history.clear();
    _showCurrentPage();
    setState(() {
      _hasContent = true;
      _pageLoading = false;
      _requests = 0;
      _error = null;
      _watchSlowLoad();
    });
    AppLogger.instance.info(
      'vtop.webview',
      'reattachedMs=${_opening.elapsedMilliseconds}',
    );
    await _updateStyles();
    if (_valid) await _ready(controller);
  }

  Future<void> _home() async {
    final controller = _controller;
    final session = widget.session;
    if (!_valid || controller == null || session == null) return;
    _reattached = false;
    final homeRevision = ++_homeRevision;
    _acceptActivity = false;
    _document = null;
    _currentMenu = null;
    _menuRequest = null;
    _restoring = null;
    // Opened straight to a page, Back leaves VTOP instead of visiting Home.
    vtopWebviewStore.history
      ..clear()
      ..addAll([if (_pendingMenu == null) VtopPage.home]);
    vtopWebviewStore.pages = null;
    widget.onPageChanged(null);
    _menuTimer?.cancel();
    _progress.value = 0;
    setState(() {
      _error = null;
      _pageLoading = true;
      _requests = 0;
    });
    _watchSlowLoad();
    try {
      final previousDocument = await controller.evaluateJavascript(
        source: 'window.__mateActivity',
      );
      if (!_valid || homeRevision != _homeRevision) return;
      _retiredDocument = previousDocument is String ? previousDocument : null;
      await _updateStyles();
      if (!_valid || homeRevision != _homeRevision) return;
      await controller.loadUrl(urlRequest: URLRequest(url: widget.initialUrl));
      vtopWebviewStore.keptCookies = session.cookieHeader;
    } catch (_) {
      _fail('Could not open VTOP.');
    }
  }

  void _fail(String message) {
    if (!_valid) return;
    setState(() {
      _error = message;
      _pageLoading = false;
      _requests = 0;
      _menuRequest = null;
      _watchSlowLoad();
    });
    _menuTimer?.cancel();
  }

  Future<void> openMenu(String url) async {
    if (!_valid || _controller == null) return;
    _pendingMenu = null;
    _currentMenu = url;
    final request = '$_instanceId:${++_menuSequence}';
    _menuTimer?.cancel();
    setState(() {
      _menuRequest = request;
      _error = null;
      _watchSlowLoad();
    });
    // Native fallback also handles a missing/unavailable JavaScript bridge.
    _menuTimer = Timer(const Duration(seconds: 11), () {
      if (_valid && _menuRequest == request) {
        _fail('This VTOP menu is unavailable. Try opening Home.');
      }
    });
    try {
      await _controller!.evaluateJavascript(
        source: vtopWaitForMenuScript(url, request),
      );
    } catch (_) {
      if (_menuRequest == request) _fail('Could not open this VTOP menu.');
    }
  }

  Future<void> _ready(InAppWebViewController controller) async {
    final navigation = _navigation;
    final authenticated = await controller.evaluateJavascript(
      source:
          "location.origin === 'https://vtop.vitap.ac.in' && !location.pathname.startsWith('/vtop/login') && !!document.querySelector('a[data-url]')",
    );
    if (!_valid || navigation != _navigation) return;
    if (authenticated == true) widget.onAuthenticatedPage();
    final target = _pendingMenu;
    if (target != null) await openMenu(target);
  }

  Future<void> _activity(
    InAppWebViewController controller,
    List<dynamic> args,
  ) async {
    if (!_valid || !_acceptActivity || args.isEmpty || args.first is! Map) {
      return;
    }
    final event = args.first as Map;
    final id = event['id'];
    if (id is! String || id == _retiredDocument) return;
    if (_document != id) {
      final navigation = _navigation;
      final current = await controller.evaluateJavascript(
        source: 'window.__mateActivity',
      );
      if (!_valid || navigation != _navigation || current != id) return;
      _document = id;
    }
    if (!_valid) return;
    if (event['type'] == 'activity') {
      final requests = (event['active'] as num?)?.toInt() ?? 0;
      final changed = (requests > 0) != (_requests > 0);
      _requests = requests;
      // Only whether VTOP is busy is shown, not how many requests are open.
      if (changed) setState(_watchSlowLoad);
    } else if (event['type'] == 'ready') {
      await _ready(controller);
    } else if (event['type'] == 'timing') {
      // Log aggregate numbers only, never URLs, resource names or response bodies.
      final values = [
        'duration',
        'resources',
        'transferBytes',
        'zeroTransfer',
      ].map((key) => '$key=${event[key] is num ? event[key] : 0}').join(' ');
      AppLogger.instance.info('vtop.webview', 'resourceTiming $values');
    }
  }

  VtopPage? get _page {
    final history = vtopWebviewStore.history;
    return history.isEmpty || history.last.isHome ? null : history.last;
  }

  void _showCurrentPage() => widget.onPageChanged(_page);

  void _visited(String url, String title) {
    if (_isDownloadAction(url)) return;
    _currentMenu = url;
    final restoring = _restoring;
    _restoring = null;
    if (restoring != null && restoring == url) return;
    final page = url.isEmpty
        ? VtopPage.home
        : VtopPage(
            url: url,
            title: title.isEmpty ? 'VTOP' : title,
            section:
                vtopWebviewStore.pages
                    ?.where((page) => page.url == url)
                    .firstOrNull
                    ?.section ??
                '',
          );
    final history = vtopWebviewStore.history;
    if (page.isHome) {
      // Home is the root; nothing before it is worth returning to.
      history
        ..clear()
        ..add(page);
    } else if (history.isNotEmpty && history.last.url == url) {
      history.last = page;
    } else {
      history.add(page);
      if (history.length > 30) history.removeAt(0);
    }
    _showCurrentPage();
  }

  /// Closes a VTOP overlay or returns to the previous page. False means the
  /// screen itself should close.
  Future<bool> handleBack() async {
    final controller = _controller;
    if (!_valid || controller == null) return false;
    try {
      final closed = await controller.evaluateJavascript(
        source: vtopCloseOverlayScript,
      );
      if (closed == true) return true;
    } catch (_) {
      // A page without VTOP's scripts falls back to page history.
    }
    final history = vtopWebviewStore.history;
    if (!_valid || history.length < 2) return false;
    history.removeLast();
    final target = history.last;
    _showCurrentPage();
    if (target.isHome) {
      _currentMenu = null;
      var opened = false;
      try {
        opened =
            await controller.evaluateJavascript(source: vtopHomeScript) == true;
      } catch (_) {}
      if (!opened) await _home();
    } else {
      _restoring = target.url;
      await openMenu(target.url);
    }
    return true;
  }

  /// The pages in VTOP's sidebar, empty until an authenticated page is open.
  Future<List<VtopPage>> pages() async {
    final cached = vtopWebviewStore.pages;
    if (cached != null && cached.isNotEmpty) return cached;
    final controller = _controller;
    if (!_valid || controller == null) return const [];
    try {
      final result = await controller.evaluateJavascript(
        source: vtopPagesScript,
      );
      final pages = result is List
          ? result.map(VtopPage.fromJson).whereType<VtopPage>().toList()
          : <VtopPage>[];
      if (_valid && pages.isNotEmpty) vtopWebviewStore.pages = pages;
      return pages;
    } catch (_) {
      return const [];
    }
  }

  /// VTOP's Home, loaded in place when possible.
  Future<void> openHome() async {
    final controller = _controller;
    if (!_valid || controller == null) return;
    try {
      if (await controller.evaluateJavascript(source: vtopHomeScript) == true) {
        _visited('', '');
        return;
      }
    } catch (_) {}
    await _home();
  }

  @override
  void dispose() {
    _slowTimer?.cancel();
    _menuTimer?.cancel();
    _progress.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    return SafeArea(
      top: false,
      child: Stack(
        fit: StackFit.expand,
        children: [
          InAppWebView(
            keepAlive: vtopWebviewStore.keepAlive,
            initialSettings: InAppWebViewSettings(
              cacheEnabled: true,
              cacheMode: CacheMode.LOAD_DEFAULT,
              isInspectable: kDebugMode,
              useOnDownloadStart: true,
              useShouldOverrideUrlLoading: true,
              transparentBackground: false,
              underPageBackgroundColor: widget.isDarkMode
                  ? colors.background
                  : Colors.white,
            ),
            initialUserScripts: UnmodifiableListView([
              _styleScript,
              UserScript(
                groupName: 'vtop-activity',
                source: vtopActivityScript,
                injectionTime: UserScriptInjectionTime.AT_DOCUMENT_START,
                forMainFrameOnly: true,
              ),
            ]),
            onWebViewCreated: (controller) async {
              _controller = controller;
              if (kDebugMode &&
                  defaultTargetPlatform == TargetPlatform.android) {
                await InAppWebViewController.setWebContentsDebuggingEnabled(
                  true,
                );
                if (!_valid) return;
              }
              controller.addJavaScriptHandler(
                handlerName: 'vtopActivity',
                callback: (args) =>
                    _activity(controller, args).catchError((Object _) {}),
              );
              controller.addJavaScriptHandler(
                handlerName: 'vtopMenuChanged',
                callback: (args) {
                  if (_valid && args.isNotEmpty && args.first is String) {
                    _visited(
                      args.first as String,
                      args.length > 1 && args[1] is String ? args[1] : '',
                    );
                  }
                },
              );
              controller.addJavaScriptHandler(
                handlerName: 'vtopMenuResult',
                callback: (args) {
                  if (!_valid || args.length != 2 || args[0] != _menuRequest) {
                    return;
                  }
                  _menuTimer?.cancel();
                  setState(() {
                    _menuRequest = null;
                    _watchSlowLoad();
                  });
                  if (args[1] == true) widget.onMenuOpened();
                  if (args[1] != true) {
                    _fail('This VTOP menu is unavailable. Try opening Home.');
                  }
                },
              );
              // Handlers are re-registered above because a reattached view
              // still holds callbacks bound to the previous, disposed state.
              _reattached = vtopWebviewStore.keptCookies != null;
              await _open();
            },
            shouldOverrideUrlLoading: (controller, action) async {
              final uri = action.request.url;
              return uri?.scheme == 'https' && uri?.host == 'vtop.vitap.ac.in'
                  ? NavigationActionPolicy.ALLOW
                  : NavigationActionPolicy.CANCEL;
            },
            onDownloadStartRequest: (controller, request) async {
              if (!_valid) return;
              try {
                final cookieHeader = await loadLatestVtopCookieHeader(
                  request.url,
                  fallbackSession: widget.session,
                );
                final referer = (await controller.getUrl())?.toString();
                String? currentMenu;
                try {
                  final value = await controller.evaluateJavascript(
                    source: 'window.__mateCurrentMenu || ""',
                  );
                  if (value is String) currentMenu = value;
                } catch (_) {
                  // The download still uses the original native path.
                }
                if (currentMenu != null &&
                    currentMenu.isNotEmpty &&
                    !_isDownloadAction(currentMenu)) {
                  _currentMenu = currentMenu;
                }
                final saveToDocs = _isOutingDownload([
                  _currentMenu,
                  currentMenu,
                  request.url.toString(),
                  referer,
                ]);
                if (!_valid) return;
                await downloadFile(
                  request.url.toString(),
                  cookieHeader,
                  contentDisposition: request.contentDisposition,
                  mimeType: request.mimeType,
                  suggestedFilename: request.suggestedFilename,
                  userAgent: request.userAgent,
                  referer: referer,
                  saveToDocs: saveToDocs,
                );
              } on DocsCopyException {
                if (context.mounted) {
                  dispToast(
                    context,
                    'Saved to Downloads',
                    'Could not add this file to Docs.',
                  );
                }
              } catch (_) {
                if (context.mounted && _valid) {
                  dispToast(
                    context,
                    'Download failed',
                    'Please try the download again.',
                  );
                }
              } finally {
                if (_valid) {
                  setState(() {
                    _pageLoading = false;
                    _watchSlowLoad();
                  });
                }
              }
            },
            onLoadStart: (controller, url) {
              if (!_valid) return;
              _navigation++;
              _retiredDocument = _document ?? _retiredDocument;
              _acceptActivity = true;
              _navigationUrl = url;
              _pageTimer
                ..reset()
                ..start();
              _slowTimer?.cancel();
              _slowTimer = null;
              _slow = false;
              _document = null;
              _progress.value = 0;
              setState(() {
                _pageLoading = true;
                _requests = 0;
                _error = null;
                _watchSlowLoad();
              });
            },
            onUpdateVisitedHistory: (controller, url, isReload) {
              if (_valid && url?.host == 'vtop.vitap.ac.in') {
                _navigationUrl = url;
              }
            },
            onProgressChanged: (controller, progress) {
              if (_valid) _progress.value = progress;
            },
            onPageCommitVisible: (controller, url) {
              if (_valid) setState(() => _hasContent = true);
            },
            onLoadStop: (controller, url) async {
              if (!_valid) return;
              final navigation = _navigation;
              final currentUrl = await controller.getUrl();
              if (!_valid || navigation != _navigation || currentUrl != url) {
                return;
              }
              setState(() {
                _pageLoading = false;
                _hasContent = true;
                _watchSlowLoad();
              });
              if (url?.host != 'vtop.vitap.ac.in') return;
              if (url!.path.startsWith('/vtop/login')) {
                widget.onLoginRedirect();
                return;
              }
              AppLogger.instance.info(
                'vtop.webview',
                'pageLoadMs=${_pageTimer.elapsedMilliseconds}'
                    '${_loggedFirstPage ? '' : ' firstPageReadyMs=${_opening.elapsedMilliseconds}'}',
              );
              _loggedFirstPage = true;
              // The document-start preferences script already styled this page.
              try {
                await _ready(controller);
              } catch (_) {
                _fail('Could not finish preparing this VTOP page.');
              }
            },
            onReceivedError: (controller, request, error) {
              if (request.isForMainFrame != true || !_valid) return;
              if (_navigationUrl != null && request.url != _navigationUrl) {
                return;
              }
              _fail(
                error.type == WebResourceErrorType.CANCELLED
                    ? 'Loading was cancelled. Open Home to continue.'
                    : 'VTOP could not load. Check your connection and try Home.',
              );
            },
            onReceivedHttpError: (controller, request, response) {
              if (request.isForMainFrame == true &&
                  (_navigationUrl == null || request.url == _navigationUrl)) {
                _fail('VTOP returned an error. Try Home again.');
              }
            },
            // Handling these keeps Android from killing the whole app along
            // with the web page's process.
            onRenderProcessGone: (controller, detail) {
              AppLogger.instance.warning(
                'vtop.webview',
                'renderProcessGone didCrash=${detail.didCrash}',
              );
              if (_valid) widget.onViewLost();
            },
            onWebContentProcessDidTerminate: (_) {
              AppLogger.instance.warning(
                'vtop.webview',
                'webContentProcessTerminated',
              );
              if (_valid) widget.onViewLost();
            },
          ),
          if (!_hasContent && _error == null)
            ColoredBox(
              color: colors.background,
              child: const Center(
                child: FCircularProgress(semanticsLabel: 'Opening VTOP'),
              ),
            ),
          if (_busy)
            Positioned(
              top: 0,
              left: 0,
              right: 0,
              child: ValueListenableBuilder(
                valueListenable: _progress,
                builder: (context, progress, _) =>
                    _pageLoading && progress > 0 && progress < 100
                    ? FDeterminateProgress(
                        value: progress / 100,
                        semanticsLabel: 'Loading VTOP page',
                      )
                    : const FProgress(semanticsLabel: 'Loading VTOP'),
              ),
            ),
          if (_error != null || _slow)
            Positioned(
              top: 12,
              left: 12,
              right: 12,
              child: DecoratedBox(
                decoration: BoxDecoration(
                  color: colors.background,
                  border: Border.all(color: colors.border),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Padding(
                  padding: const EdgeInsets.all(16),
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Text(
                        _error ??
                            'VTOP is taking longer than usual. You can keep waiting.',
                        style: context.theme.typography.body.sm,
                        textAlign: TextAlign.center,
                      ),
                      const SizedBox(height: 12),
                      FButton(
                        variant: FButtonVariant.outline,
                        onPress: _home,
                        child: const Text('Open Home'),
                      ),
                    ],
                  ),
                ),
              ),
            ),
        ],
      ),
    );
  }
}
