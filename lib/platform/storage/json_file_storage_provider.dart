import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/vtop/vtop_user_provider.dart';
import 'package:vitapmate/platform/storage/json_file_storage.dart';

part 'json_file_storage_provider.g.dart';

@Riverpod(keepAlive: true)
Future<JsonFileStorage> jsonFileStorage(Ref ref) async {
  final username = await ref.watch(
    vtopUserProvider.selectAsync((val) => val.username),
  );
  return JsonFileStorage(username: username ?? 'NO_USERNAME');
}
