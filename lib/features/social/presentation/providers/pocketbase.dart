import 'dart:developer';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:pocketbase/pocketbase.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/main.dart';
part 'pocketbase.g.dart';

const pbUrl = "https://api.va.kryxen.dev/";
// const pbUrl = "http://10.0.2.2:8090/";
@riverpod
Future<PocketBase> pb(Ref ref) async {
  final prefs = await SharedPreferences.getInstance();
  final store = AsyncAuthStore(
    save: (String data) async => prefs.setString('pb_auth', data),
    initial: prefs.getString('pb_auth'),
  );

  var pb = PocketBase(pbUrl, authStore: store, reuseHTTPClient: true);

  try {
    if (pb.authStore.isValid) {
      var k = await pb.collection('users').authRefresh();
    }
  } catch (e) {
    if (e.toString().toLowerCase().contains(
      "The request requires valid record authorization token".toLowerCase(),
    )) {
      pb.authStore.clear();
    }
  }
  ref.onDispose(() {
    pb.close();
  });

  return pb;
}

Future<void> pbSetNotificationToken(PocketBase pb, String? token) async {
  if (pb.authStore.isValid && token != null) {
    final userId = pb.authStore.record!.id;

    try {
      await pb
          .collection('notifications_token')
          .create(body: {'token': token, 'user': userId});
    } catch (e) {
      try {
        final existing = await pb
            .collection('notifications_token')
            .getFirstListItem('user="$userId"');
        if (existing.getStringValue("token") == token) {
          return;
        }

        await pb
            .collection('notifications_token')
            .update(existing.id, body: {'token': token});
      } catch (inner) {
        log('Failed to update notification token: $inner');
      }
    }
  }
}

Future<void> pbSetNtotification({String? tokenNew}) async {
  final pref = SharedPreferencesAsync();
  final prefs = await SharedPreferences.getInstance();
  final store = AsyncAuthStore(
    save: (String data) async => prefs.setString('pb_auth', data),
    initial: prefs.getString('pb_auth'),
  );
  var pb = PocketBase(pbUrl, authStore: store);
  var token = tokenNew ?? await notifications.getToken();
  await pbSetNotificationToken(pb, token);
}
