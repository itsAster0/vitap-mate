import 'dart:convert';

import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/features/background/entity_sync.dart';
import 'package:workmanager/workmanager.dart';
part 'controller.g.dart';

@Riverpod(keepAlive: true)
class BackgroundSync extends _$BackgroundSync {
  final _keySync = "background_sync_key_v2";
  @override
  Future<SyncState> build() async {
    final prefs = await SharedPreferences.getInstance();
    final syncjsonstr = prefs.getString(_keySync);
    if (syncjsonstr == null) {
      final freq = Duration(hours: 12);
      final k = SyncState(freq: freq);

      _save(k);
      return k;
    }
    return SyncState.fromJson(jsonDecode(syncjsonstr));
  }

  void _save(SyncState k) async {
    final prefs = await SharedPreferences.getInstance();

    await prefs.setString(_keySync, jsonEncode(k.toJson()));
    _configure(k.freq);
  }

  void updateFreq(Duration freq) async {
    final sync = await future;
    final newSync = sync.copyWith(freq: freq);
    state = AsyncData(newSync);

    _save(newSync);
  }

  void _configure(Duration freqk) async {
    final freq = freqk == Duration(seconds: 0) ? null : freqk;
    final valToken = "sync_vtop";
    if (freq != null) {
      Workmanager().registerPeriodicTask(
        valToken,
        valToken,
        existingWorkPolicy: ExistingPeriodicWorkPolicy.update,
        frequency: freq,
        constraints: Constraints(networkType: NetworkType.connected),
      );
    } else {
      Workmanager().cancelByUniqueName(valToken);
    }
  }
}
