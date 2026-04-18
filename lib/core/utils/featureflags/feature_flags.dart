import 'package:flagsmith/flagsmith.dart';
import 'package:flutter/foundation.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';

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

@Riverpod(keepAlive: true)
class FeatureFlagsController extends _$FeatureFlagsController {
  @override
  Future<FeatureFlagPodController> build() async {
    final user = await ref.watch(vtopUserProvider.future);
    final info = await PackageInfo.fromPlatform();

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
      storageType: StorageType.inMemory,
      caches: true,
    );
    final flagsmithClient = await FlagsmithClient.init(
      config: config,
      apiKey: apiKey,
      seeds: <Flag>[],
    );

    final username = user.username?.trim();
    final appVersionCode = _versionStringToNumber(info.version);
    final parsedBuildNumber = int.tryParse(info.buildNumber);
    final buildNumberTraitValue = parsedBuildNumber ?? info.buildNumber.trim();
    final traits = <Trait>[
      Trait(key: "appVersionCode", value: appVersionCode),
      Trait(key: "buildNumber", value: buildNumberTraitValue),
    ];
    final identity = Identity(
      identifier: username?.toUpperCase() ?? "uninitialized_user".toUpperCase(),
    );

    await flagsmithClient.getFeatureFlags(
      user: identity,
      traits: traits,
      reload: true,
    );

    return FeatureFlagPodController(
      flagsmithClient,
      identity: identity,
      traits: traits,
    );
  }
}

int _versionStringToNumber(String version) {
  final parts = version.split('.');
  final major = int.tryParse(parts[0]) ?? 0;
  final minor = parts.length > 1 ? int.tryParse(parts[1]) ?? 0 : 0;
  final patch = parts.length > 2 ? int.tryParse(parts[2]) ?? 0 : 0;
  return major * 1000000 + minor * 1000 + patch;
}
