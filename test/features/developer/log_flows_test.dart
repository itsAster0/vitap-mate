import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/features/developer/log_flows.dart';
import 'package:vitapmate/observability/log.dart';

LogLine line(
  int t,
  String? flow,
  String message, {
  String level = 'info',
  String source = 'test',
  Map<String, Object?> data = const {},
}) => (
  time: DateTime.fromMillisecondsSinceEpoch(t),
  level: level,
  source: source,
  message: message,
  flow: flow,
  regno: null,
  data: data,
);

void main() {
  test('groups lines by flow, newest first, named by the first step', () {
    final flows = groupFlows([
      line(1, 'a', 'start', data: {'step': 'vtop_attendance_S', 'ev': 'start'}),
      line(2, 'b', 'start', data: {'step': 'sync', 'ev': 'start'}),
      line(3, 'a', 'attendance over UDP: 400', level: 'warn'),
      line(4, null, 'no flow'),
      line(
        5,
        'a',
        'done',
        data: {'step': 'vtop_attendance_S', 'ev': 'done', 'ms': 1200},
      ),
    ]);
    expect(flows.map((f) => f.id), ['b', 'a']);
    final (:name, :status, :ms, :problems, :lines, id: _, via: _) = flows[1];
    expect(name, 'Fetching attendance');
    expect(status, FlowStatus.done);
    expect(ms, 1200);
    expect(problems, isTrue);
    expect(lines, hasLength(3));
    expect(flows[1].via, ['UDP']);
    expect(flows[0].status, FlowStatus.stopped);
    expect(
      groupFlows([
        line(1, 'b', 'start', data: {'step': 'sync', 'ev': 'start'}),
      ], now: DateTime.fromMillisecondsSinceEpoch(1000)).single.status,
      FlowStatus.running,
    );
  });

  test('each request says how it went', () {
    LogLine from(String source, String message) =>
        line(0, null, message, source: source);
    expect(lineVia(from('vtop.server', 'marks over UDP: 200')), 'UDP');
    expect(
      lineVia(from('http', 'GET vmate-api.aster0.dev/v1/events 200')),
      'HTTPS server',
    );
    expect(
      lineVia(
        from(
          'rust.network',
          'request x POST https://vmate-api.aster0.dev/vtop/login',
        ),
      ),
      'HTTPS proxy',
    );
    expect(
      lineVia(
        from(
          'rust.network',
          'request x GET https://vtop.vitap.ac.in/vtop/open/page',
        ),
      ),
      'HTTPS direct',
    );
    expect(lineVia(from('vtop.queue', 'vtop_marks start')), isNull);
  });

  test('a failed step fails its flow', () {
    final [flow] = groupFlows([
      line(1, 'a', 'start', data: {'step': 'login', 'ev': 'start'}),
      line(2, 'a', 'inner', data: {'step': 'otp', 'ev': 'done', 'ms': 5}),
      line(3, 'a', 'x', data: {'step': 'login', 'ev': 'failed', 'ms': 9}),
    ]);
    expect(flow.status, FlowStatus.failed);
    expect(flow.ms, 9);
  });
}
