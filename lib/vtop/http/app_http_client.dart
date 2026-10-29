import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:http/http.dart' as http;
import 'package:vitapmate/observability/log.dart';

/// One client per isolate, never closed, so connections are reused. Code
/// without a Ref (background isolates, ad hoc services) uses this too.
final http.Client sharedAppHttpClient = _FlowIdClient(http.Client());

/// Sends the current [flow] id as `x-request-id`, so the server's log lines
/// for a request can be found from the phone's.
class _FlowIdClient extends http.BaseClient {
  _FlowIdClient(this._inner);

  final http.Client _inner;

  @override
  Future<http.StreamedResponse> send(http.BaseRequest request) {
    if (currentFlow case final id?) request.headers['x-request-id'] = id;
    return _inner.send(request);
  }

  @override
  void close() => _inner.close();
}

/// [sharedAppHttpClient], overridable in tests.
final appHttpClientProvider = Provider<http.Client>(
  (ref) => sharedAppHttpClient,
);
