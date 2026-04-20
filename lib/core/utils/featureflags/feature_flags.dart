import 'dart:async';

import 'package:flagsmith/flagsmith.dart';
import 'package:flutter/foundation.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/logging/app_logger.dart';

part 'feature_flags.g.dart';

class FeatureFlagPodController {
  const FeatureFlagPodController(
    this._client, {
    this.identity,
    this.traits = const <Trait>[],
  });

  final FlagsmithClient _client;
  final Identity? identity;
  final List<Trait> traits;

  FlagsmithClient get sdk => _client;

  Future<void> refresh({bool reload = true}) async {
    await _client.getFeatureFlags(
      user: identity,
      traits: traits,
      reload: reload,
    );
  }

  Future<bool> has(String key) async {
    return _client.hasFeatureFlag(key, user: identity);
  }

  Future<bool> isEnabled(String key) async {
    final exists = await has(key);
    if (!exists) return false;
    return _client.isFeatureFlagEnabled(key, user: identity);
  }

  Future<dynamic> value(String key) async {
    return _client.getFeatureFlagValue(key, user: identity);
  }
}

class PrefixedFlagsmithStore extends CoreStorage {
  PrefixedFlagsmithStore({this.prefix = 'flagsmith_cache_'});

  final String prefix;
  SharedPreferences? _prefs;

  String _scopedKey(String key) => '$prefix$key';

  @override
  Future<void> init() async {
    _prefs ??= await SharedPreferences.getInstance();
  }

  @override
  Future<bool> clear() async {
    await init();
    final keys = _prefs!.getKeys().where((key) => key.startsWith(prefix));
    var changed = false;
    for (final key in keys.toList()) {
      changed = await _prefs!.remove(key) || changed;
    }
    return changed;
  }

  @override
  Future<bool> create(String key, String item) async {
    await init();
    final scoped = _scopedKey(key);
    if (_prefs!.containsKey(scoped)) return false;
    return _prefs!.setString(scoped, item);
  }

  @override
  Future<bool> delete(String key) async {
    await init();
    return _prefs!.remove(_scopedKey(key));
  }

  @override
  Future<List<String?>> getAll() async {
    await init();
    final values = <String?>[];
    for (final key in _prefs!.getKeys()) {
      if (!key.startsWith(prefix)) continue;
      values.add(_prefs!.getString(key));
    }
    return values.whereType<String>().toList();
  }

  @override
  Future<String?> read(String key) async {
    await init();
    return _prefs!.getString(_scopedKey(key));
  }

  @override
  Future<bool> seed(List<MapEntry<String, String>> items) async {
    await init();
    final existing = await getAll();
    if (existing.isNotEmpty || items.isEmpty) return false;
    var changed = false;
    for (final item in items) {
      changed = await create(item.key, item.value) || changed;
    }
    return changed;
  }

  @override
  Future<bool> update(String key, String item) async {
    await init();
    return _prefs!.setString(_scopedKey(key), item);
  }
}

@Riverpod(keepAlive: true)
class FeatureFlagsController extends _$FeatureFlagsController {
  @override
  Future<FeatureFlagPodController> build() async {
    final userFuture = ref.watch(vtopUserProvider.future);
    final infoFuture = PackageInfo.fromPlatform();
    final user = await userFuture;
    final packageInfo = await infoFuture;

    const devApiKey = String.fromEnvironment('FLAGSMITH_ENV_API_KEY_DEV');
    const prodApiKey = String.fromEnvironment('FLAGSMITH_ENV_API_KEY_PROD');
    const envBaseUri = String.fromEnvironment('FLAGSMITH_BASE_URI');
    final apiKey = kDebugMode ? devApiKey : prodApiKey;
    if (apiKey.isEmpty || envBaseUri.isEmpty) {
      throw StateError(
        'Missing Flagsmith API key or base URI. Provide FLAGSMITH_ENV_API_KEY_DEV/PROD and FLAGSMITH_BASE_URI via --dart-define or --dart-define-from-file.',
      );
    }
    final config = FlagsmithConfig(
      baseURI: envBaseUri,
      storageType: StorageType.custom,
      caches: true,
    );
    final flagsmithClient = await FlagsmithClient.init(
      config: config,
      apiKey: apiKey,
      storage: PrefixedFlagsmithStore(),
      seeds: <Flag>[],
    );

    final username = user.username?.trim();
    final parsedBuildNumber = int.tryParse(packageInfo.buildNumber);
    final buildNumberTraitValue =
        parsedBuildNumber ?? packageInfo.buildNumber.trim();
    final traits = <Trait>[
      Trait(key: "appVersion", value: packageInfo.version.trim()),
      Trait(key: "buildNumber", value: buildNumberTraitValue),
    ];
    final identity = Identity(
      identifier: username?.toUpperCase() ?? "uninitialized_user".toUpperCase(),
    );

    await flagsmithClient.getFeatureFlags(
      user: identity,
      traits: traits,
      reload: false,
    );

    final controller = FeatureFlagPodController(
      flagsmithClient,
      identity: identity,
      traits: traits,
    );

    unawaited(_refreshInBackground(controller));

    return controller;
  }

  Future<void> _refreshInBackground(FeatureFlagPodController controller) async {
    try {
      AppLogger.instance.info(
        'client.feature_flags',
        'refresh.start background',
      );
      await controller.refresh(reload: true);
      AppLogger.instance.info(
        'client.feature_flags',
        'refresh.success background',
      );
    } catch (error) {
      AppLogger.instance.warning(
        'client.feature_flags',
        'refresh.error background: $error',
      );
    }
  }
}
