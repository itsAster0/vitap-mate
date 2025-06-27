import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:pocketbase/pocketbase.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:shared_preferences/shared_preferences.dart';
part 'pocketbase.g.dart';

@riverpod
Future<PocketBase> pb(Ref ref) async {
  final prefs = await SharedPreferences.getInstance();
  final store = AsyncAuthStore(
    save: (String data) async => prefs.setString('pb_auth', data),
    initial: prefs.getString('pb_auth'),
  );
  var pb = PocketBase("https://api.va.synaptic.gg", authStore: store);
  try {
    if (pb.authStore.isValid) {
      var k = await pb.collection('users').authRefresh();
    }
  } catch (e) {
    if (e.toString().toLowerCase().contains(
      "The request requires valid record authorization token".toLowerCase(),
    )){
       pb.authStore.clear();
    }
     
  }
  return pb;
}
