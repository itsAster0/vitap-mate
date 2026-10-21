import 'package:vitapmate/core/vtop_backend/vtop_query.dart';

/// Where VTOP data comes from. Data sources fetch through this and do not
/// know how the page is fetched ([LocalVtopBackend]: the phone's client,
/// to VTOP or through the server's proxy). Pages are described once, in
/// `vtop_queries.dart`; no backend knows their names.
abstract interface class VtopBackend {
  /// Short name for logs.
  String get name;

  Future<T> fetch<T>(VtopQuery<T> query);

  /// Changes VTOP. Never retried except after an expired session.
  Future<T> run<T>(VtopAction<T> action);
}
