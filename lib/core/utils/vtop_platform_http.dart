import 'dart:async';
import 'dart:io';
import 'dart:typed_data';

import 'package:cronet_http/cronet_http.dart';
import 'package:cupertino_http/cupertino_http.dart';
import 'package:http/http.dart' as http;
import 'package:vitapmate/core/logging/app_logger.dart';
import 'package:vitapmate/core/providers/vtop_proxy.dart';
import 'package:vitapmate/core/utils/app_http_client.dart';
import 'package:vitapmate/src/api/platform_http.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';

const _log = SourceLogger('client.http');

/// A VTOP client for the saved proxy setting. Through the proxy, requests
/// go out on the phone's own HTTP stack (HTTP/3 when the server offers it,
/// HTTP/2 when QUIC is blocked); straight to VTOP, on the Rust client, which
/// carries the intermediate certificate VTOP sometimes leaves out.
Future<VtopClient> createVtopClient({
  required String username,
  required String password,
  PersistedVtopSession? persistedSession,
}) async {
  final baseUrl = await currentVtopBaseUrl();
  final platform = _vtopPlatformClient;
  if (baseUrl == null || platform == null) {
    _log.info(
      'vtop client: reqwest, ${baseUrl == null ? 'direct' : 'via $baseUrl'}',
    );
    return getVtopClient(
      username: username,
      password: password,
      persistedSession: persistedSession,
      baseUrl: baseUrl,
    );
  }
  _log.info('vtop client: platform (${appHttpClientName()}) via $baseUrl');
  return getVtopClientViaPlatform(
    username: username,
    password: password,
    persistedSession: persistedSession,
    baseUrl: baseUrl,
    send: (request) => sendPlatformHttp(platform, request),
  );
}

/// The client for vtop-core's requests, one per isolate. It must not follow
/// redirects or keep cookies: vtop-core does both. Cronet keeps no cookies,
/// so Android shares the app's client (and its HTTP/3 connection to the
/// server); NSURLSession does unless told not to. Null where there is no
/// platform stack, and the Rust client is used.
final http.Client? _vtopPlatformClient = Platform.isAndroid
    ? sharedAppHttpClient
    : Platform.isIOS
    ? CupertinoClient.fromSessionConfiguration(
        URLSessionConfiguration.ephemeralSessionConfiguration()
          ..httpShouldSetCookies = false
          ..httpCookieAcceptPolicy =
              NSHTTPCookieAcceptPolicy.NSHTTPCookieAcceptPolicyNever,
      )
    : null;

/// Sends [request] on [client] exactly as given. Never throws: a failure
/// comes back as [PlatformHttpResponse.errorKind], since vtop-core waits on
/// the answer.
Future<PlatformHttpResponse> sendPlatformHttp(
  http.Client client,
  PlatformHttpRequest request,
) async {
  final url = Uri.parse(request.url);
  final watch = Stopwatch()..start();
  // Fires on timeout, so the request is cancelled instead of left running.
  final abort = Completer<void>();
  try {
    final outgoing =
        http.AbortableRequest(request.method, url, abortTrigger: abort.future)
          ..followRedirects = false
          ..bodyBytes = request.body;
    for (final (name, value) in request.headers) {
      final previous = outgoing.headers[name];
      outgoing.headers[name] = previous == null ? value : '$previous, $value';
    }
    String? protocol;
    final response = await client
        .send(outgoing)
        .then((streamed) {
          if (streamed is CronetStreamedResponse) {
            protocol = streamed.negotiatedProtocol;
          }
          return http.Response.fromStream(streamed);
        })
        .timeout(Duration(milliseconds: request.timeoutMs));
    // The path only: query strings are not logged.
    _log.info(
      'platform ${request.method} ${url.path} -> ${response.statusCode}'
      '${protocol == null ? '' : ' $protocol'} (${watch.elapsedMilliseconds}ms)',
    );
    return PlatformHttpResponse(
      status: response.statusCode,
      url: request.url,
      headers: _headerLines(response),
      body: response.bodyBytes,
    );
  } on TimeoutException catch (error) {
    abort.complete();
    _log.warning('platform ${request.method} ${url.path} timed out');
    return _failure(PlatformHttpErrorKind.timeout, request, error);
  } catch (error) {
    _log.warning('platform ${request.method} ${url.path} failed: $error');
    // The request may have reached VTOP, so it is not retried.
    return _failure(PlatformHttpErrorKind.other, request, error);
  }
}

/// One entry per header line. `headers` joins repeated `Set-Cookie` lines
/// with commas, which cookie dates also contain, so those are split back.
List<(String, String)> _headerLines(http.Response response) => [
  for (final MapEntry(:key, :value) in response.headers.entries)
    if (key == 'set-cookie')
      for (final cookie in response.headersSplitValues[key] ?? [value])
        (key, cookie)
    else
      (key, value),
];

PlatformHttpResponse _failure(
  PlatformHttpErrorKind kind,
  PlatformHttpRequest request,
  Object error,
) => PlatformHttpResponse(
  status: 0,
  url: request.url,
  headers: const [],
  body: Uint8List(0),
  errorKind: kind,
  error: '${request.method} ${request.url}: $error',
);
