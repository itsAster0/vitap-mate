import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'feature_flags.g.dart';

class FeatureFlagPodController {
  const FeatureFlagPodController();

  Future<void> refresh({bool reload = true}) async {}

  Future<bool> has(String key) async => true;

  Future<bool> isEnabled(String key) async => true;

  Future<dynamic> value(String key) async => true;
}

@Riverpod(keepAlive: true)
class FeatureFlagsController extends _$FeatureFlagsController {
  @override
  Future<FeatureFlagPodController> build() async =>
      const FeatureFlagPodController();
}
