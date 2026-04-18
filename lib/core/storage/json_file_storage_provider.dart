import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/storage/json_file_storage.dart';

part 'json_file_storage_provider.g.dart';

@Riverpod(keepAlive: true)
Future<JsonFileStorage> jsonFileStorage(Ref ref) async {
  final username = await ref.watch(
    vtopUserProvider.selectAsync((val) => val.username),
  );
  return JsonFileStorage(username: username ?? 'NO_USERNAME');
}
