import 'dart:async';
import 'dart:convert';
import 'dart:developer' show log;
import 'dart:io';
import 'dart:math' show Random, min;

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart' show AppLifecycleListener;
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/core/utils/app_urls.dart';
import 'package:vitapmate/core/utils/edge/edge_device.dart';

/// One entry of this install's event log on the edge.
class EdgeEvent {
  const EdgeEvent({required this.seq, required this.kind, required this.data});

  final int seq;

  /// What happened, e.g. `mail_otp`.
  final String kind;

  /// The whole event as sent, `seq` and `kind` included.
  final Map<String, dynamic> data;

  /// From a socket frame's `event`, or an FCM data map (strings only,
  /// `type` for `kind`).
  static EdgeEvent? fromJson(Map<String, dynamic> json) {
    final seq = int.tryParse('${json['seq'] ?? ''}');
    final kind = '${json['kind'] ?? json['type'] ?? ''}';
    if (seq == null || seq <= 0 || kind.isEmpty) return null;
    return EdgeEvent(seq: seq, kind: kind, data: json);
  }
}

typedef EdgeEventHandler = Future<void> Function(EdgeEvent event);

/// The live link to the edge. While the app is in front a WebSocket is
/// open and events arrive on it at once; in the background Android may
/// kill sockets, so FCM carries them and the socket catches up on what it
/// missed when the app comes back. Both paths go through [accept], which
/// hands each sequence number to the handler once.
class EdgeEventChannel {
  EdgeEventChannel._();

  static final instance = EdgeEventChannel._();

  /// Per install: a new registration starts a new log at 1.
  static String _cursorKey(String device) => 'edge_events_after_v1_$device';

  /// The sequence numbers handled lately, kept on disk so the FCM
  /// background isolate and a restarted app see them too.
  static const _handledKey = 'edge_events_handled_v1';
  static const _handledKept = 128;
  static const _pingEvery = Duration(seconds: 15);
  static const _maxBackoff = Duration(seconds: 30);

  /// Whether a socket is open and answering pings. While it is, nothing
  /// needs to poll.
  final connected = ValueNotifier<bool>(false);

  final _handlers = <String, EdgeEventHandler>{};
  final _seen = <int>{};
  final _devices = EdgeDeviceService();
  WebSocket? _socket;
  String? _deviceId;
  Timer? _ping;
  Timer? _retry;
  AppLifecycleListener? _lifecycle;
  DateTime _lastPong = DateTime.now();
  var _failures = 0;
  var _foreground = true;
  var _connecting = false;

  void on(String kind, EdgeEventHandler handler) => _handlers[kind] = handler;

  /// Connects now and follows the app between front and back.
  void start() {
    _lifecycle ??= AppLifecycleListener(
      onResume: () {
        _foreground = true;
        unawaited(connect());
      },
      onPause: () {
        _foreground = false;
        _close();
      },
    );
    unawaited(connect());
  }

  /// Opens the socket if this install is registered and none is open;
  /// also after registering, to start listening at once.
  Future<void> connect() async {
    if (!_foreground || _socket != null || _connecting) return;
    if (vtopEdgeBaseUrl.isEmpty) return;
    _connecting = true;
    try {
      final device = await _devices.load();
      if (device == null) return;
      final after = await _cursor(device.id);
      final url = Uri.parse(vtopEdgeBaseUrl).replace(
        scheme: vtopEdgeBaseUrl.startsWith('http://') ? 'ws' : 'wss',
        path: '/v1/events',
        queryParameters: {'device': device.id, 'after': '$after'},
      );
      final socket = await WebSocket.connect(
        url.toString(),
        headers: {'Authorization': 'Bearer ${device.secret}'},
      ).timeout(const Duration(seconds: 15));
      if (!_foreground) {
        await socket.close(1000);
        return;
      }
      _socket = socket;
      _deviceId = device.id;
      _failures = 0;
      _lastPong = DateTime.now();
      connected.value = true;
      socket.listen(
        _onFrame,
        onDone: _onDropped,
        onError: (_) => _onDropped(),
        cancelOnError: true,
      );
      _ping = Timer.periodic(_pingEvery, (_) => _sendPing());
    } catch (error) {
      log('Event socket failed: $error', name: 'edge.events');
      _scheduleRetry();
    } finally {
      _connecting = false;
    }
  }

  void _sendPing() {
    // Two pings without a pong: the link is dead even if no one said so.
    if (DateTime.now().difference(_lastPong) > _pingEvery * 2) {
      log('Event socket stopped answering', name: 'edge.events');
      _onDropped();
      return;
    }
    _socket?.add('ping');
  }

  void _onFrame(dynamic frame) {
    _lastPong = DateTime.now();
    if (frame is! String || frame == 'pong') return;
    try {
      final json = jsonDecode(frame);
      if (json is! Map<String, dynamic> || json['type'] != 'event') return;
      final event = json['event'];
      if (event is! Map<String, dynamic>) return;
      final parsed = EdgeEvent.fromJson(event);
      if (parsed != null) unawaited(accept(parsed, advance: true));
    } catch (_) {}
  }

  void _onDropped() {
    _close();
    _scheduleRetry();
  }

  void _close() {
    _ping?.cancel();
    _ping = null;
    _retry?.cancel();
    _retry = null;
    final socket = _socket;
    _socket = null;
    connected.value = false;
    if (socket != null) unawaited(socket.close(1000).catchError((_) {}));
  }

  void _scheduleRetry() {
    if (!_foreground || _retry != null) return;
    _failures++;
    final seconds = min(_maxBackoff.inSeconds, 1 << min(_failures - 1, 5));
    final jitter = Random().nextInt(500);
    _retry = Timer(Duration(seconds: seconds, milliseconds: jitter), () {
      _retry = null;
      unawaited(connect());
    });
  }

  Future<int> _cursor(String device) async {
    final prefs = await SharedPreferences.getInstance();
    return prefs.getInt(_cursorKey(device)) ?? 0;
  }

  /// Hands [event] to its handler unless this sequence number was handled
  /// already. Socket events arrive in order, so they also [advance] where
  /// the next catch-up starts; a push may skip ahead and does not.
  Future<void> accept(EdgeEvent event, {bool advance = false}) async {
    final prefs = await SharedPreferences.getInstance();
    final device = _deviceId;
    // Advanced even for a number handled already (it may have come by
    // push first), so catch-ups don't start before it again.
    if (advance && device != null) {
      if ((prefs.getInt(_cursorKey(device)) ?? 0) < event.seq) {
        await prefs.setInt(_cursorKey(device), event.seq);
      }
    }
    if (!_seen.add(event.seq)) return;
    if (_seen.length > 256) _seen.remove(_seen.reduce(min));
    // Another isolate may have handled it.
    await prefs.reload();
    final handled = [...?prefs.getStringList(_handledKey)];
    if (handled.contains('${event.seq}')) return;
    handled.add('${event.seq}');
    await prefs.setStringList(
      _handledKey,
      handled.length > _handledKept
          ? handled.sublist(handled.length - _handledKept)
          : handled,
    );
    final handler = _handlers[event.kind];
    if (handler == null) return;
    try {
      await handler(event);
    } catch (error) {
      log('Event ${event.kind} failed: $error', name: 'edge.events');
    }
  }

  /// Moves to another registration of this install: its log starts over.
  void restart() {
    _seen.clear();
    unawaited(
      SharedPreferences.getInstance().then(
        (prefs) => prefs.remove(_handledKey),
      ),
    );
    _close();
    unawaited(connect());
  }
}
