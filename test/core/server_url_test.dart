import 'package:flutter_test/flutter_test.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shared_preferences_platform_interface/in_memory_shared_preferences_async.dart';
import 'package:shared_preferences_platform_interface/shared_preferences_async_platform_interface.dart';
import 'package:vitapmate/vtop/route/server_url.dart';
import 'package:vitapmate/app/app_urls.dart';

void main() {
  setUp(() {
    SharedPreferencesAsyncPlatform.instance =
        InMemorySharedPreferencesAsync.empty();
  });

  group('normalizeServerUrl', () {
    test('trims and drops a trailing slash', () {
      expect(normalizeServerUrl('  https://h.example/ '), 'https://h.example');
      expect(
        normalizeServerUrl('https://h.example:8443'),
        'https://h.example:8443',
      );
    });

    test('refuses a path, query or fragment', () {
      expect(normalizeServerUrl('https://h.example/v1'), isNull);
      expect(normalizeServerUrl('https://h.example?a=1'), isNull);
      expect(normalizeServerUrl('https://h.example#x'), isNull);
    });

    test('plain http only for local and private addresses', () {
      expect(
        normalizeServerUrl('http://localhost:8787'),
        'http://localhost:8787',
      );
      expect(
        normalizeServerUrl('http://127.0.0.1:8787'),
        'http://127.0.0.1:8787',
      );
      expect(normalizeServerUrl('http://192.168.1.4'), 'http://192.168.1.4');
      expect(
        normalizeServerUrl('http://10.0.2.2:8787'),
        'http://10.0.2.2:8787',
      );
      expect(normalizeServerUrl('http://172.20.0.1'), 'http://172.20.0.1');
      expect(normalizeServerUrl('http://172.40.0.1'), isNull);
      expect(normalizeServerUrl('http://example.com'), isNull);
    });

    test('refuses other schemes and junk', () {
      expect(normalizeServerUrl('ftp://h.example'), isNull);
      expect(normalizeServerUrl('h.example'), isNull);
      expect(normalizeServerUrl(''), isNull);
      expect(normalizeServerUrl('https://'), isNull);
    });
  });

  group('probeServer', () {
    http.Client answering(int status, String body) =>
        MockClient((request) async {
          expect(request.url.path, '/healthz');
          return http.Response(body, status);
        });

    test('accepts the edge', () async {
      expect(
        await probeServer(
          'https://h.example',
          answering(200, '{"service":"vtop-edge","status":"ok"}'),
        ),
        isTrue,
      );
    });

    test('refuses another service, an error or junk', () async {
      expect(
        await probeServer(
          'https://h.example',
          answering(200, '{"service":"other"}'),
        ),
        isFalse,
      );
      expect(
        await probeServer('https://h.example', answering(500, '{}')),
        isFalse,
      );
      expect(
        await probeServer('https://h.example', answering(200, 'hello')),
        isFalse,
      );
      expect(
        await probeServer(
          'https://h.example',
          MockClient((_) async => throw http.ClientException('down')),
        ),
        isFalse,
      );
    });
  });

  group('server URL setting', () {
    test('is the built-in default until one is saved', () async {
      expect(defaultServerUrl, 'https://vmate-api.aster0.dev');
      expect(await currentServerUrl(), defaultServerUrl);
      final container = ProviderContainer();
      addTearDown(container.dispose);
      await container.read(serverUrlProvider.notifier).ready;
      expect(container.read(serverUrlProvider), defaultServerUrl);
      expect(container.read(serverUrlProvider.notifier).isDefault, isTrue);
    });

    test('a saved URL is used, and reset returns to the default', () async {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final notifier = container.read(serverUrlProvider.notifier);
      await notifier.ready;
      await notifier.set('https://other.example');
      expect(container.read(serverUrlProvider), 'https://other.example');
      expect(await currentServerUrl(), 'https://other.example');
      expect(notifier.isDefault, isFalse);
      await notifier.reset();
      expect(container.read(serverUrlProvider), defaultServerUrl);
      expect(await currentServerUrl(), defaultServerUrl);
    });

    test('an invalid URL is refused and the old one kept', () async {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final notifier = container.read(serverUrlProvider.notifier);
      await notifier.ready;
      await expectLater(
        notifier.set('https://h.example/v1'),
        throwsArgumentError,
      );
      expect(container.read(serverUrlProvider), defaultServerUrl);
    });
  });
}
