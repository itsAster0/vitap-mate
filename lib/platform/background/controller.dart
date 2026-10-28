import 'dart:convert';

import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/platform/background/sync.dart';
import 'package:workmanager/workmanager.dart';

/// How often the periodic background sync runs; zero turns it off.
typedef SyncState = ({Duration freq});

final backgroundSyncProvider = AsyncNotifierProvider<BackgroundSync, SyncState>(
  BackgroundSync.new,
);

class BackgroundSync extends AsyncNotifier<SyncState> {
  static const _keySync = "background_sync_key_v3";

  @override
  Future<SyncState> build() async {
    final prefs = await SharedPreferences.getInstance();
    if (prefs.getString(_keySync) case final json?) {
      final micros = (jsonDecode(json) as Map<String, dynamic>)['freq'] as num;
      return (freq: Duration(microseconds: micros.toInt()));
    }
    const initial = (freq: Duration.zero);
    _save(initial);
    return initial;
  }

  void _save(SyncState sync) async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(
      _keySync,
      jsonEncode({'freq': sync.freq.inMicroseconds}),
    );
    if (sync.freq == Duration.zero) {
      Workmanager().cancelByUniqueName(backgroundSyncTask);
    } else {
      Workmanager().registerPeriodicTask(
        backgroundSyncTask,
        backgroundSyncTask,
        existingWorkPolicy: .update,
        frequency: sync.freq,
        constraints: Constraints(networkType: .connected),
      );
    }
  }

  void updateFreq(Duration freq) async {
    await future;
    final sync = (freq: freq);
    state = AsyncData(sync);
    _save(sync);
  }
}
