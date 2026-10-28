import 'package:vitapmate/vtop/backend/vtop_backend.dart';
import 'package:vitapmate/vtop/backend/vtop_query.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';

/// Fetches and parses in the bundled Rust library.
class LocalVtopBackend implements VtopBackend {
  LocalVtopBackend(this._client);

  final Future<VtopClient> Function() _client;

  @override
  String get name => 'local';

  @override
  Future<T> fetch<T>(VtopQuery<T> query) async => query.local(await _client());

  @override
  Future<T> run<T>(VtopAction<T> action) async => action.local(await _client());
}
