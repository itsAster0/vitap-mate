import 'package:vitapmate/vtop/backend/vtop_backend.dart';
import 'package:vitapmate/vtop/backend/vtop_query.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';

/// Signs in again and retries once when a fetch fails with
/// [VtopError.sessionExpired].
///
/// The app's "logged in" flag can be stale: VTOP may drop a session the
/// device still believes in. Without this, a refresh would stop at
/// "Your saved session expired" instead of recovering.
///
/// When several fetches fail at once they share one re-login, and a fetch
/// that started before a re-login finished just retries on the new session.
///
/// A rejected password ([VtopError.invalidCredentials]), from the server's
/// sign-in or the phone's, goes to [onCredentialsRejected] and is never
/// retried.
class ReloginVtopBackend implements VtopBackend {
  ReloginVtopBackend(
    this._inner, {
    required Future<void> Function() relogin,
    Future<void> Function()? onCredentialsRejected,
  }) : _relogin = relogin,
       _onCredentialsRejected = onCredentialsRejected;

  final VtopBackend _inner;
  final Future<void> Function() _relogin;
  final Future<void> Function()? _onCredentialsRejected;

  /// The backend this one retries for.
  VtopBackend get inner => _inner;

  /// Bumped after every successful re-login.
  int _generation = 0;
  Future<void>? _inFlight;

  @override
  String get name => _inner.name;

  /// Runs [fetch], signing in again and retrying once on an expired session.
  Future<T> retrying<T>(Future<T> Function() fetch) => _guard(fetch);

  Future<T> _guard<T>(Future<T> Function() fetch) async {
    try {
      return await _retrying(fetch);
    } on VtopError_InvalidCredentials {
      await _onCredentialsRejected?.call();
      rethrow;
    }
  }

  Future<T> _retrying<T>(Future<T> Function() fetch) async {
    final startedAt = _generation;
    try {
      return await fetch();
    } on VtopError_SessionExpired {
      if (_generation == startedAt) {
        await (_inFlight ??= _signInAgain());
      }
      return fetch();
    }
  }

  Future<void> _signInAgain() async {
    try {
      await _relogin();
      _generation++;
    } finally {
      _inFlight = null;
    }
  }

  @override
  Future<T> fetch<T>(VtopQuery<T> query) => _guard(() => _inner.fetch(query));

  /// Safe to send again: an expired session means VTOP refused it before
  /// running it.
  @override
  Future<T> run<T>(VtopAction<T> action) => _guard(() => _inner.run(action));
}
