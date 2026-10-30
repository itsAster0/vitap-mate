import 'dart:convert';
import 'dart:io';

import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:http/http.dart' as http;
import 'package:http/io_client.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/observability/log.dart';
import 'package:vitapmate/vtop/http/traffic_inspection.dart';

/// One client per isolate, never closed, so connections are reused. Code
/// without a Ref (background isolates, ad hoc services) uses this too.
/// Built on first use, after [TrafficInspection.init].
final http.Client sharedAppHttpClient = _FlowIdClient(_innerClient());

http.Client _innerClient() {
  final inspection = TrafficInspection.current;
  if (inspection == null) return http.Client();
  final context = SecurityContext(withTrustedRoots: true);
  for (final pem in inspection.caPems) {
    context.setTrustedCertificatesBytes(utf8.encode(pem));
  }
  final client = HttpClient(context: context);
  if (inspection.proxy case final proxy?) {
    client.findProxy = (_) => 'PROXY $proxy';
  }
  return IOClient(client);
}

/// Sends the current [flow] id as `x-request-id`, so the server's log lines
/// for a request can be found from the phone's, and logs each request:
/// method, host and path (never the query), status, size, time to the
/// answer, and why it was sent (the flow and the app code that sent it).
class _FlowIdClient extends http.BaseClient {
  _FlowIdClient(this._inner);

  final http.Client _inner;

  @override
  Future<http.StreamedResponse> send(http.BaseRequest request) async {
    // Before any await, while the caller is still on the stack.
    final (:why, :data) = _reason(StackTrace.current);
    if (currentFlow case final id?) request.headers['x-request-id'] = id;
    final what = '${request.method} ${urlForLog(request.url)}';
    Log.write(.debug, 'http', '$what sent for $why', data);
    final watch = Stopwatch()..start();
    try {
      final response = await _inner.send(request);
      final size = switch (response.contentLength) {
        final bytes? => ' ${bytes}B',
        null => '',
      };
      Log.write(
        response.statusCode < 400 ? .info : .warn,
        'http',
        '$what ${response.statusCode}$size ${watch.elapsedMilliseconds}ms '
            'for $why',
        data,
      );
      return response;
    } catch (error) {
      Log.write(
        .warn,
        'http',
        '$what failed ${watch.elapsedMilliseconds}ms for $why: $error',
        data,
      );
      rethrow;
    }
  }

  @override
  void close() => _inner.close();
}

/// Logs a network call made by a plugin's own native code (Firebase, Play,
/// AppAuth, downloads, the browser), which [sharedAppHttpClient] never
/// sees: what, why, how long, and how it ended.
Future<T> loggedNetCall<T>(String what, Future<T> Function() call) async {
  final (:why, :data) = _reason(StackTrace.current);
  final watch = Stopwatch()..start();
  try {
    final result = await call();
    Log.write(
      .info,
      'net',
      '$what done ${watch.elapsedMilliseconds}ms for $why',
      data,
    );
    return result;
  } catch (error) {
    Log.write(
      .warn,
      'net',
      '$what failed ${watch.elapsedMilliseconds}ms for $why: $error',
      data,
    );
    rethrow;
  }
}

/// Host and path of [url], never its query (it may carry tokens).
String urlForLog(Uri? url) => url == null ? '?' : '${url.host}${url.path}';

/// Why a request is sent: the [flow] it runs in and the app code that
/// sent it, found in [stack].
({String why, Map<String, Object?> data}) _reason(StackTrace stack) {
  final by = _caller(stack);
  return (
    why: [?currentFlowName, by].join(' via '),
    data: {'why': ?currentFlowName, 'by': by},
  );
}

final _frame = RegExp(r'^#\d+\s+(.+?) \(package:vitapmate/(?:.*/)?([^/]+)\)');

/// The first frame of the app's own code in [stack], e.g.
/// `EdgeEventChannel.connect (edge_events.dart:160:12)`.
String _caller(StackTrace stack) {
  for (final line in '$stack'.split('\n')) {
    if (_frame.firstMatch(line) case final match?
        when !match[2]!.startsWith('app_http_client.dart') &&
            !match[2]!.startsWith('log.dart')) {
      return '${match[1]} (${match[2]})';
    }
  }
  return 'unknown caller';
}

/// [sharedAppHttpClient], overridable in tests.
final appHttpClientProvider = Provider<http.Client>(
  (ref) => sharedAppHttpClient,
);

/// Developer → Connect At Launch. On (the default), launch syncs the push
/// token and checks for an update, and the edge event stream opens after a
/// minute in front. Off, each happens only when needed: the stream while
/// something holds it (e.g. a sign-in waiting for its forwarded OTP), the
/// push token when Firebase changes it, the update check from Settings.
abstract final class LaunchNetwork {
  static const settingKey = 'settings_connect_at_launch';
  static var atLaunch = true;

  /// Reads the setting; call once at launch.
  static Future<void> init() async {
    try {
      atLaunch = await SharedPreferencesAsync().getBool(settingKey) ?? true;
    } catch (_) {}
    if (!atLaunch) Log.info('http', 'lazy: nothing connects at launch');
  }
}
