import 'package:vitapmate/vtop/backend/vtop_backend.dart';
import 'package:vitapmate/vtop/backend/vtop_query.dart';

/// Records what is asked for and answers with [answer].
class RecordingBackend implements VtopBackend {
  RecordingBackend(this.answer);

  /// The value for a key; throw from it to fail the call.
  final Object Function(String key) answer;
  final fetched = <VtopQuery<Object?>>[];
  final ran = <VtopAction<Object?>>[];

  @override
  String get name => 'recording';

  @override
  Future<T> fetch<T>(VtopQuery<T> query) async {
    fetched.add(query);
    return answer(query.key) as T;
  }

  @override
  Future<T> run<T>(VtopAction<T> action) async {
    ran.add(action);
    return answer(action.key) as T;
  }
}
