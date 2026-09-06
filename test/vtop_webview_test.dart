import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_inappwebview/flutter_inappwebview.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/vtop_webview_store.dart';
import 'package:vitapmate/features/more/presentation/widgets/vtop_webview/vtop_webview_cookie_service.dart';
import 'package:vitapmate/features/more/presentation/widgets/vtop_webview/vtop_webview_loading.dart';
import 'package:vitapmate/features/more/presentation/widgets/vtop_webview/vtop_webview_recovery.dart';
import 'package:vitapmate/features/more/presentation/widgets/vtop_webview/vtop_webview_scripts.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

// Mutable storage test double.
// ignore: must_be_immutable
class TestPreferences implements SharedPreferencesWithCache {
  final values = <String, bool>{};
  final writes = <bool>[];
  Completer<void>? gate;
  bool failNext = false;
  @override
  bool? getBool(String key) => values[key];
  @override
  Future<void> setBool(String key, bool value) async {
    writes.add(value);
    await gate?.future;
    if (failNext) {
      failNext = false;
      throw StateError('disk unavailable');
    }
    values[key] = value;
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class TestCookies implements CookieManager {
  List<Cookie> cookies = [];
  final writes = <String>[];
  final deletions = <String>[];
  bool rejectWrite = false;
  VoidCallback? afterRead;
  @override
  Future<List<Cookie>> getCookies({
    required WebUri url,
    InAppWebViewController? iosBelow11WebViewController,
    InAppWebViewController? webViewController,
  }) async {
    expect(url.host, 'vtop.vitap.ac.in');
    expect(url.path, '/vtop/');
    afterRead?.call();
    return cookies;
  }

  @override
  Future<bool> setCookie({
    required WebUri url,
    required String name,
    required String value,
    String path = '/',
    String? domain,
    int? expiresDate,
    int? maxAge,
    bool? isSecure,
    bool? isHttpOnly,
    HTTPCookieSameSitePolicy? sameSite,
    InAppWebViewController? iosBelow11WebViewController,
    InAppWebViewController? webViewController,
  }) async {
    expect(url.host, 'vtop.vitap.ac.in');
    expect(path, '/vtop');
    expect(isSecure, isTrue);
    writes.add('$name=$value');
    return !rejectWrite;
  }

  @override
  Future<bool> deleteCookie({
    required WebUri url,
    required String name,
    String path = '/',
    String? domain,
    InAppWebViewController? iosBelow11WebViewController,
    InAppWebViewController? webViewController,
  }) async {
    expect(url.host, 'vtop.vitap.ac.in');
    deletions.add(name);
    return true;
  }

  // Global deletion deliberately has no implementation, so any use fails.
  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  Future<ProviderContainer> container(TestPreferences prefs) async {
    final result = ProviderContainer(
      overrides: [settingsProvider.overrideWith((ref) async => prefs)],
    );
    addTearDown(result.dispose);
    await result.read(settingsProvider.future);
    return result;
  }

  test(
    'view preferences retain defaults and survive a fresh provider container',
    () async {
      final prefs = TestPreferences();
      final first = await container(prefs);
      expect(first.read(vtopCompactModeProvider), isTrue);
      expect(first.read(vtopDesktopModeProvider), isFalse);
      await first.read(vtopCompactModeProvider.notifier).setValue(false);
      await first.read(vtopDesktopModeProvider.notifier).setValue(true);
      final reopened = await container(prefs);
      expect(reopened.read(vtopCompactModeProvider), isFalse);
      expect(reopened.read(vtopDesktopModeProvider), isTrue);
    },
  );

  test('rapid toggles update immediately and write serially', () async {
    final prefs = TestPreferences()..gate = Completer<void>();
    final ref = await container(prefs);
    final notifier = ref.read(vtopDesktopModeProvider.notifier);
    final first = notifier.setValue(true);
    final second = notifier.setValue(false);
    final third = notifier.setValue(true);
    expect(ref.read(vtopDesktopModeProvider), isTrue);
    await Future<void>.delayed(Duration.zero);
    expect(prefs.writes, [true]);
    prefs.gate!.complete();
    await Future.wait([first, second, third]);
    expect(prefs.writes, [true, false, true]);
    expect(prefs.values[vtopDesktopModeSettingKey], isTrue);
  });

  test('write failures are surfaced and do not block future saves', () async {
    final prefs = TestPreferences()..failNext = true;
    final ref = await container(prefs);
    final notifier = ref.read(vtopCompactModeProvider.notifier);
    await expectLater(notifier.setValue(false), throwsStateError);
    expect(ref.read(vtopCompactModeProvider), isFalse);
    await notifier.setValue(true);
    expect(prefs.values[vtopCompactModeSettingKey], isTrue);
  });

  Future<VtopWebviewSession?> sync(
    TestCookies cookies, {
    bool Function()? current,
  }) => loadVtopWebviewSession(
    snapshot: PersistedVtopSession(
      username: 'TEST',
      savedAtEpochMs: BigInt.zero,
      cookies: 'JSESSIONID=new; token=a=b',
    ),
    baseUrl: WebUri('https://vtop.vitap.ac.in'),
    manager: cookies,
    isCurrent: current,
  );

  test(
    'cookie synchronization skips unchanged values and removes obsolete cookies',
    () async {
      final cookies = TestCookies()
        ..cookies = [
          Cookie(name: 'JSESSIONID', value: 'new', path: '/vtop'),
          Cookie(
            name: 'obsolete',
            value: 'old',
            path: '/vtop',
            domain: '.vitap.ac.in',
          ),
        ];
      final session = await sync(cookies);
      expect(cookies.writes, ['token=a=b']);
      expect(cookies.deletions, ['obsolete']);
      expect(session!.cookieHeader, 'JSESSIONID=new; token=a=b');
    },
  );

  test(
    'changed cookies replace prior values and failed native writes surface',
    () async {
      final cookies = TestCookies()
        ..cookies = [Cookie(name: 'JSESSIONID', value: 'old')];
      await sync(cookies);
      expect(cookies.deletions, hasLength(9));
      expect(cookies.deletions, everyElement('JSESSIONID'));
      expect(cookies.writes, ['JSESSIONID=new', 'token=a=b']);
      await expectLater(
        sync(TestCookies()..rejectWrite = true),
        throwsStateError,
      );
    },
  );

  test(
    'stale preparation cannot mutate cookies after asynchronous reads',
    () async {
      var current = true;
      final cookies = TestCookies()..afterRead = () => current = false;
      expect(await sync(cookies, current: () => current), isNull);
      expect(cookies.writes, isEmpty);
      expect(cookies.deletions, isEmpty);
    },
  );

  test(
    'cookie state is limited to one account and cleared on logout',
    () async {
      var cleared = 0;
      final store = VtopWebviewStore(
        clearCookies: () async {
          cleared++;
        },
      );
      store.acquire('alice');
      store.acquire('ALICE');
      await store.clearIfDifferent('ALICE');
      expect(cleared, 0);
      expect(() => store.acquire('bob'), throwsStateError);
      await store.clearIfDifferent('bob');
      expect(cleared, 1);
      store.acquire('bob');
      await store.clear(username: 'alice');
      store.acquire('bob');
      await store.clearIfDifferent(null);
      expect(cleared, 2);
    },
  );

  test(
    'account cleanup waits for in-flight cookie work and invalidates callbacks immediately',
    () async {
      final gate = Completer<void>();
      final events = <String>[];
      final store = VtopWebviewStore(
        clearCookies: () async {
          events.add('clear');
        },
      );
      store.acquire('alice');
      final generation = store.generation;
      final write = store.serialize(() async {
        events.add('write');
        await gate.future;
      });
      await Future<void>.delayed(Duration.zero);
      final clear = store.clear();
      expect(store.generation, greaterThan(generation));
      expect(events, ['write']);
      gate.complete();
      await Future.wait([write, clear]);
      expect(events, ['write', 'clear']);
    },
  );

  test('login recovery cannot loop until an authenticated page succeeds', () {
    final recovery = VtopWebviewRecovery();
    expect(recovery.beginAutomaticAttempt(), isTrue);
    for (var i = 0; i < 5; i++) {
      expect(recovery.beginAutomaticAttempt(), isFalse);
    }
    recovery.authenticatedPageReady();
    expect(recovery.beginAutomaticAttempt(), isTrue);
  });

  testWidgets('ForUI preparation and retry fit large text in both themes', (
    tester,
  ) async {
    for (final theme in [
      FTheme.neutral.light.touch,
      FTheme.neutral.dark.touch,
    ]) {
      var retries = 0;
      Widget app({Object? error}) => MaterialApp(
        builder: (_, child) => FTheme(
          data: theme,
          child: MediaQuery(
            data: const MediaQueryData(
              size: Size(400, 800),
              textScaler: TextScaler.linear(1.8),
            ),
            child: child!,
          ),
        ),
        home: FScaffold(
          child: VtopWebviewLoading(error: error, onRetry: () => retries++),
        ),
      );
      await tester.pumpWidget(app());
      expect(find.byType(FCircularProgress), findsOneWidget);
      expect(find.byType(CircularProgressIndicator), findsNothing);
      expect(tester.takeException(), isNull);
      await tester.pump(const Duration(seconds: 16));
      expect(find.textContaining('taking longer than usual'), findsOneWidget);
      await tester.pumpWidget(app(error: StateError('failure')));
      await tester.tap(find.text('Retry'));
      await tester.pumpAndSettle();
      expect(retries, 1);
      expect(tester.takeException(), isNull);
    }
  });

  test(
    'JavaScript preserves requests, handles cancellation, styles and bounded shortcuts',
    () async {
      final process = await Process.start('node', [
        'test/support/vtop_webview_scripts_test.cjs',
      ]);
      process.stdin.write(
        jsonEncode({
          'activity': vtopActivityScript,
          'stylesOn': vtopPreferencesScript(
            dark: true,
            compact: true,
            desktop: true,
          ),
          'stylesOff': vtopPreferencesScript(
            dark: false,
            compact: false,
            desktop: false,
          ),
          'menu': vtopWaitForMenuScript('academics/test', 'request-1'),
          'quotedMenu': vtopWaitForMenuScript('a"\\\n]', 'request-2'),
        }),
      );
      await process.stdin.close();
      final output = process.stdout.transform(utf8.decoder).join();
      final errors = process.stderr.transform(utf8.decoder).join();
      expect(
        await process.exitCode,
        0,
        reason: '${await output}\n${await errors}',
      );
    },
  );
}
