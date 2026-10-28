import 'dart:async';
import 'dart:convert';
import 'dart:io' show HttpException;
import 'dart:math' show Random, min;

import 'package:vitapmate/observability/log.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart' show AppLifecycleListener;
import 'package:http/http.dart' as http;
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/vtop/route/server_url.dart';
import 'package:vitapmate/vtop/http/app_http_client.dart';
import 'package:vitapmate/vtop/edge/edge_device.dart';
import 'package:vitapmate/vtop/route/on_device_mode.dart';

/// One entry of this install's event log on the edge.
class EdgeEvent {
  const EdgeEvent({required this.seq, required this.kind, required this.data});

  final int seq;

  /// What happened, e.g. `mail_otp`.
  final String kind;

  /// The whole event as sent, `seq` and `kind` included.
  final Map<String, dynamic> data;

  /// From a stream frame's `event`, or an FCM data map (strings only,
  /// `type` for `kind`).
  static EdgeEvent? fromJson(Map<String, dynamic> json) {
    final seq = int.tryParse('${json['seq'] ?? ''}');
    final kind = '${json['kind'] ?? json['type'] ?? ''}';
    if (seq == null || seq <= 0 || kind.isEmpty) return null;
    return EdgeEvent(seq: seq, kind: kind, data: json);
  }
}

typedef EdgeEventHandler = Future<void> Function(EdgeEvent event);

/// The live link to the edge. While the app is in front and something
/// needs events at once (see [alwaysOn] and [hold]) a server-sent event
/// stream is open and events arrive on it as they happen. It is read on
/// [sharedAppHttpClient], so it rides the HTTP/3 connection the app's other
/// calls use, and its heartbeat keeps that connection from going idle. In
/// the background Android may kill connections, so FCM carries events and
/// the stream catches up on what it missed when the app comes back. Both paths go through [accept], which
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

  /// How often the edge sends a heartbeat; two missed ones drop the link.
  static const _heartbeat = Duration(seconds: 15);
  static const _maxBackoff = Duration(seconds: 30);

  /// How long a stream must stay up before the backoff starts over. A
  /// stream that opens and drops at once (a flaky network, a server that
  /// ends it) keeps backing off instead of reconnecting every second.
  static const _stableAfter = Duration(minutes: 1);

  /// Whether a stream is open and beating. While it is, nothing
  /// needs to poll.
  final connected = ValueNotifier<bool>(false);

  final _handlers = <String, EdgeEventHandler>{};
  final _seen = <int>{};
  final _devices = EdgeDeviceService();
  StreamSubscription<String>? _stream;
  String? _deviceId;
  Timer? _watchdog;
  Timer? _retry;
  AppLifecycleListener? _lifecycle;
  DateTime _lastBeat = DateTime.now();
  DateTime? _openedAt;
  final _data = StringBuffer();
  var _failures = 0;
  var _foreground = true;
  var _connecting = false;

  /// Whether the stream stays open in front with no one [hold]ing it. Set
  /// at launch: on for a linked account (the extension and agents ask for
  /// cookies through it) and where FCM is missing. Otherwise the stream
  /// opens only while held, and FCM carries the rest; that saves a TLS
  /// handshake and a held-open connection on every launch, which on a slow
  /// network is most of what the launch refresh has to share the link with.
  Future<bool> Function() alwaysOn = () async => true;
  var _holds = 0;

  /// Keeps the stream open until the returned function is called, e.g.
  /// while a sign-in waits for its forwarded OTP.
  VoidCallback hold() {
    _holds++;
    unawaited(connect());
    var released = false;
    return () {
      if (released) return;
      released = true;
      _holds--;
      if (_holds == 0) unawaited(_closeIfUnwanted());
    };
  }

  Future<bool> _wanted() async => _holds > 0 || await alwaysOn();

  Future<void> _closeIfUnwanted() async {
    if (!await _wanted()) _close();
  }

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

  /// Opens the stream if this install is registered and none is open;
  /// also after registering, to start listening at once.
  Future<void> connect() async {
    if (!_foreground || _stream != null || _connecting) return;
    if (await isOnDeviceMode()) return;
    if (!await _wanted()) return;
    _connecting = true;
    try {
      final device = await _devices.load();
      if (device == null) return;
      final after = await _cursor(device.id);
      final server = await currentServerUrl();
      final url = Uri.parse(server).replace(
        path: '/v1/events',
        queryParameters: {'device': device.id, 'after': '$after'},
      );
      final request = http.Request('GET', url)
        ..headers.addAll({
          'Accept': 'text/event-stream',
          'Authorization': 'Bearer ${device.secret}',
        });
      final response = await sharedAppHttpClient
          .send(request)
          .timeout(const Duration(seconds: 15));
      final type = response.headers['content-type'] ?? '';
      if (response.statusCode != 200 || !type.startsWith('text/event-stream')) {
        unawaited(response.stream.listen(null).cancel());
        throw HttpException('HTTP ${response.statusCode} ($type)', uri: url);
      }
      if (!_foreground) {
        unawaited(response.stream.listen(null).cancel());
        return;
      }
      _deviceId = device.id;
      _openedAt = _lastBeat = DateTime.now();
      _data.clear();
      connected.value = true;
      _stream = response.stream
          .transform(utf8.decoder)
          .transform(const LineSplitter())
          .listen(
            _onLine,
            onDone: _onDropped,
            onError: (_) => _onDropped(),
            cancelOnError: true,
          );
      _watchdog = Timer.periodic(_heartbeat, (_) => _checkBeat());
    } catch (error) {
      Log.info('edge.events', 'Event stream failed: $error');
      _scheduleRetry();
    } finally {
      _connecting = false;
    }
  }

  void _checkBeat() {
    // Two beats missed: the link is dead even if no one said so.
    if (DateTime.now().difference(_lastBeat) > _heartbeat * 2) {
      Log.info('edge.events', 'Event stream stopped beating');
      _onDropped();
    }
  }

  /// One line of the stream: `data:` lines make up a frame, a blank line
  /// ends it, and `:` lines are the heartbeat.
  void _onLine(String line) {
    _lastBeat = DateTime.now();
    if (line.isEmpty) {
      if (_data.isEmpty) return;
      final frame = _data.toString();
      _data.clear();
      _onFrame(frame);
    } else if (line.startsWith('data:')) {
      if (_data.isNotEmpty) _data.write('\n');
      final value = line.substring(5);
      _data.write(value.startsWith(' ') ? value.substring(1) : value);
    }
  }

  void _onFrame(String frame) {
    try {
      final json = jsonDecode(frame);
      if (json is! Map<String, dynamic> || json['type'] != 'event') return;
      final event = json['event'];
      if (event is! Map<String, dynamic>) return;
      final parsed = EdgeEvent.fromJson(event);
      if (parsed != null) unawaited(accept(parsed, advance: true));
    } catch (_) {}
  }

  /// Closes the stream and stays closed until [connect].
  void stop() => _close();

  void _onDropped() {
    final openedAt = _openedAt;
    if (openedAt != null &&
        DateTime.now().difference(openedAt) >= _stableAfter) {
      _failures = 0;
    }
    _close();
    _scheduleRetry();
  }

  void _close() {
    _watchdog?.cancel();
    _watchdog = null;
    _retry?.cancel();
    _retry = null;
    // Cancelling the subscription cancels the request, ending the stream.
    final stream = _stream;
    _stream = null;
    _openedAt = null;
    connected.value = false;
    if (stream != null) unawaited(stream.cancel().catchError((_) {}));
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
  /// already. Stream events arrive in order, so they also [advance] where
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
      Log.info('edge.events', 'Event ${event.kind} failed: $error');
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
