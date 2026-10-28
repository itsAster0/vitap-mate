import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/vtop/vtop_user_provider.dart';
import 'package:vitapmate/platform/storage/json_file_storage.dart';

final jsonFileStorageProvider = FutureProvider<JsonFileStorage>((ref) async {
  final username = await ref.watch(
    vtopUserProvider.selectAsync((val) => val.username),
  );
  return JsonFileStorage(username: username ?? 'NO_USERNAME');
});
