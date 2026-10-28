import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/vtop/edge/edge_device.dart';
import 'package:vitapmate/vtop/edge/server_switch.dart';
import 'package:vitapmate/vtop/email_otp/mail_forwarding_service.dart';
import 'package:vitapmate/vtop/vtop_bridge_account_service.dart';

class _Devices implements EdgeDeviceStore {
  EdgeDevice? value = const EdgeDevice(id: 'd', secret: 's', fcmToken: 't');

  @override
  Future<EdgeDevice?> read() async => value;

  @override
  Future<void> write(EdgeDevice device) async => value = device;

  @override
  Future<void> delete() async => value = null;
}

class _Forwarding implements MailForwardingStore {
  MailForwarding? value = const MailForwarding(
    id: 'm',
    address: 'vtop-otp-m@h.example',
    secret: 's',
    privateKey: 'k',
  );

  @override
  Future<MailForwarding?> read() async => value;

  @override
  Future<void> write(MailForwarding forwarding) async => value = forwarding;

  @override
  Future<void> delete() async => value = null;
}

class _Rig {
  final devices = _Devices();
  final forwarding = _Forwarding();
  final secret = MemoryBridgeAppSecretStore();
  final keys = MemoryBridgeKeyStash();
  final saved = <String?>[];
  final events = <String>[];
  bool linked = true;
  bool onDevice = false;

  ServerSwitch get serverSwitch => ServerSwitch(
    devices: devices,
    forwarding: forwarding,
    appSecret: secret,
    keys: keys,
    saveUrl: (url) async => saved.add(url),
    setLinked: (value) async => linked = value,
    onDeviceMode: () async => onDevice,
    pause: () async => events.add('pause'),
    resume: () async => events.add('resume'),
  );

  Future<void> seed() async {
    await secret.write('app-secret');
    await keys.save(const BridgeKey(id: 'k', key: 'vtm_x', mcpUrl: 'u'));
  }
}

void main() {
  test('switching forgets the old install and registers again', () async {
    final rig = _Rig();
    await rig.seed();
    await rig.serverSwitch.switchTo('https://other.example');
    expect(rig.devices.value, isNull);
    expect(rig.forwarding.value, isNull);
    expect(await rig.secret.read(), isNull);
    expect(await rig.keys.readAll(), isEmpty);
    expect(rig.linked, isFalse);
    expect(rig.saved, ['https://other.example']);
    expect(rig.events, ['pause', 'resume']);
  });

  test('switching back to the old URL registers again', () async {
    final rig = _Rig();
    await rig.serverSwitch.switchTo('https://other.example');
    rig.devices.value = const EdgeDevice(id: 'd2', secret: 's', fcmToken: 't');
    await rig.serverSwitch.switchTo(null);
    expect(rig.devices.value, isNull);
    expect(rig.saved, ['https://other.example', null]);
    expect(rig.events, ['pause', 'resume', 'pause', 'resume']);
  });

  test('on-device mode switches the URL without registering', () async {
    final rig = _Rig()..onDevice = true;
    await rig.serverSwitch.switchTo('https://other.example');
    expect(rig.saved, ['https://other.example']);
    expect(rig.events, ['pause']);
  });
}
