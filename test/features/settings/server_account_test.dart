import 'package:flutter_test/flutter_test.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:shared_preferences_platform_interface/in_memory_shared_preferences_async.dart';
import 'package:shared_preferences_platform_interface/shared_preferences_async_platform_interface.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/vtop/vtop_bridge_account_service.dart';
import 'package:vitapmate/features/settings/presentation/providers/server_account.dart';

BridgeAccount accountWith({
  bool linkedHere = true,
  bool saved = false,
  List<BridgePhone> phones = const [],
}) => BridgeAccount(
  registrationNumber: '22BCE0001',
  linked: linkedHere,
  savedCredentials: saved,
  phoneLinked: linkedHere,
  thisPhone: linkedHere,
  keys: const [],
  phones: phones,
  semesterId: 'S',
);

class _Api implements ServerAccountApi {
  _Api(this.current);

  BridgeAccount current;
  final calls = <String>[];

  /// What the next [account] answers after a link with credentials.
  BridgeAccount? afterLink;

  @override
  Future<BridgeAccount> account() async => current;

  @override
  Future<void> link({required bool withCredentials}) async {
    calls.add('link:$withCredentials');
    current = afterLink ?? accountWith(saved: withCredentials);
  }

  @override
  Future<BridgeKey> createKey(String label, {bool outings = false}) async {
    calls.add('createKey:$label:$outings');
    return const BridgeKey(id: 'k', key: 'vtm_x', mcpUrl: 'https://e/mcp');
  }

  @override
  Future<void> setKeyOutings(String id, bool outings) async =>
      calls.add('outings:$id:$outings');

  @override
  Future<void> revokeKey(String id) async => calls.add('revoke:$id');

  @override
  Future<void> removePhone(BridgePhone phone) async =>
      calls.add('remove:${phone.id}');

  /// Answers like the network: after a pause.
  bool slow = false;

  @override
  Future<void> forgetCredentials() async {
    if (slow) await Future<void>.delayed(const Duration(milliseconds: 20));
    calls.add('forget');
    current = accountWith();
  }

  @override
  Future<void> updateSettings(BridgeSettings settings) async =>
      calls.add('settings');

  @override
  Future<void> deleteAccount() async {
    calls.add('delete');
    current = accountWith(linkedHere: false);
  }
}

void main() {
  Future<(ProviderContainer, _Api)> rig(
    BridgeAccount start, {
    Map<String, Object> prefs = const {},
  }) async {
    SharedPreferencesAsyncPlatform.instance =
        InMemorySharedPreferencesAsync.withData(prefs);
    final api = _Api(start);
    final container = ProviderContainer(
      overrides: [serverAccountApiProvider.overrideWithValue(api)],
    );
    addTearDown(container.dispose);
    container.listen(serverAccountProvider, (_, _) {});
    await container.read(settingsProvider.future);
    await container.read(serverAccountProvider.future);
    return (container, api);
  }

  ServerAccount controller(ProviderContainer c) =>
      c.read(serverAccountProvider.notifier);

  Future<bool?> pref(ProviderContainer c, String key) async =>
      (await c.read(settingsProvider.future)).getBool(key);

  test('an action finishes even when no page watches the account', () async {
    SharedPreferencesAsyncPlatform.instance =
        InMemorySharedPreferencesAsync.withData({});
    final api = _Api(accountWith(saved: true))..slow = true;
    final container = ProviderContainer(
      overrides: [serverAccountApiProvider.overrideWithValue(api)],
    );
    addTearDown(container.dispose);
    await container.read(settingsProvider.future);
    // The mode page only reads the notifier; nothing listens.
    await controller(container).setServerSignIn(false);
    expect(api.calls, ['forget']);
    expect(await pref(container, bridgeServerSignInSettingKey), isFalse);
  });

  test('turning server sign-in on links with the password', () async {
    final (c, api) = await rig(accountWith());
    await controller(c).setServerSignIn(true);
    expect(api.calls, ['link:true']);
    expect(await pref(c, bridgeServerSignInSettingKey), isTrue);
    expect(c.read(serverAccountProvider).value!.savedCredentials, isTrue);
  });

  test('a server that kept no password is an error', () async {
    final (c, api) = await rig(accountWith());
    api.afterLink = accountWith();
    await expectLater(
      controller(c).setServerSignIn(true),
      throwsA(isA<StateError>()),
    );
    expect(await pref(c, bridgeServerSignInSettingKey), isNot(isTrue));
  });

  test('turning it off forgets the password', () async {
    final (c, api) = await rig(
      accountWith(saved: true),
      prefs: {bridgeServerSignInSettingKey: true},
    );
    await controller(c).setServerSignIn(false);
    expect(api.calls, ['forget']);
    expect(await pref(c, bridgeServerSignInSettingKey), isFalse);
  });

  test('a key on an unlinked phone links it first', () async {
    final (c, api) = await rig(accountWith(linkedHere: false));
    await controller(c).createKey('laptop');
    expect(api.calls, ['link:false', 'createKey:laptop:false']);
    expect(await pref(c, bridgeLinkedSettingKey), isTrue);
  });

  test('outing permission is sent on create and when changed', () async {
    final (c, api) = await rig(accountWith());
    await controller(c).createKey('agent', outings: true);
    await controller(c).setKeyOutings('k', false);
    expect(api.calls, ['createKey:agent:true', 'outings:k:false']);
  });

  test('removing this phone unlinks it here', () async {
    final me = BridgePhone(
      id: 'p1',
      label: null,
      addedAt: DateTime.utc(2026),
      seenAt: DateTime.utc(2026),
      thisPhone: true,
    );
    final (c, api) = await rig(
      accountWith(phones: [me]),
      prefs: {bridgeLinkedSettingKey: true},
    );
    await controller(c).removePhone(me);
    expect(api.calls, ['remove:p1']);
    expect(await pref(c, bridgeLinkedSettingKey), isFalse);
  });

  test('deleting the data unlinks and forgets server sign-in', () async {
    final (c, api) = await rig(
      accountWith(saved: true),
      prefs: {bridgeLinkedSettingKey: true, bridgeServerSignInSettingKey: true},
    );
    await controller(c).deleteData();
    expect(api.calls, ['delete']);
    expect(await pref(c, bridgeLinkedSettingKey), isFalse);
    expect(await pref(c, bridgeServerSignInSettingKey), isFalse);
  });
}
