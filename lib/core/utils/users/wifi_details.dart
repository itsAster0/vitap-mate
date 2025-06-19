import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/utils/users/vtop_users_utils.dart';
part 'wifi_details.g.dart';

@riverpod
Future<(String?, String?)> wifiDetails(Ref ref) async {
  return ref.watch(vtopusersutilsProvider.notifier).getWifiDetails();
}
