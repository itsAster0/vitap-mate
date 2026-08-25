import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';

void main() {
  test('all feature flags are enabled locally', () async {
    const flags = FeatureFlagPodController();

    expect(await flags.has('unknown-feature'), isTrue);
    expect(await flags.isEnabled('unknown-feature'), isTrue);
    expect(await flags.value('unknown-feature'), isTrue);
    await flags.refresh();
  });
}
