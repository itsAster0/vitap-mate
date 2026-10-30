import 'package:vitapmate/observability/log.dart';
import 'package:vitapmate/vtop/vtop_page.dart';

enum FlowStatus { running, done, failed, stopped }

/// A flow with no line for this long, and no end, was cut short (the app
/// was closed).
const _quiet = Duration(minutes: 2);

/// One user action or background run, from its log lines.
typedef LogFlow = ({
  String id,
  String name,
  FlowStatus status,
  int? ms,
  List<LogLine> lines,

  /// How it reached the server or VTOP, in order (see [lineVia]).
  List<String> via,

  /// Whether any line is a warning or an error.
  bool problems,
});

/// [lines] (oldest first) grouped by flow id, newest flow first. A flow is
/// named by its first step, done or failed when that step ended.
List<LogFlow> groupFlows(List<LogLine> lines, {DateTime? now}) {
  final byId = <String, List<LogLine>>{};
  for (final line in lines) {
    if (line.flow case final id?) (byId[id] ??= []).add(line);
  }
  final flows = [
    for (final MapEntry(key: id, value: lines) in byId.entries)
      _flow(id, lines, now ?? DateTime.now()),
  ]..sort((a, b) => b.lines.first.time.compareTo(a.lines.first.time));
  return flows;
}

LogFlow _flow(String id, List<LogLine> lines, DateTime now) {
  final start = lines.firstWhere(
    (line) => line.data['ev'] == 'start',
    orElse: () => lines.first,
  );
  final step = start.data['step'] as String?;
  final end = lines.lastWhere(
    (line) =>
        step != null && line.data['step'] == step && line.data['ev'] != 'start',
    orElse: () => lines.first,
  );
  return (
    id: id,
    name: step == null ? start.message : flowTitle(step),
    status: switch (end.data['ev']) {
      'done' => .done,
      'failed' => .failed,
      _ when now.difference(lines.last.time) > _quiet => .stopped,
      _ => .running,
    },
    ms: end.data['ms'] as int?,
    lines: lines,
    via: {for (final line in lines) ?lineVia(line)}.toList(),
    problems: lines.any(
      (line) => line.level == 'warn' || line.level == 'error',
    ),
  );
}

/// A queue task's label (`vtop_attendance_S` → Fetching attendance), else
/// [step] itself.
String flowTitle(String step) =>
    step.startsWith('vtop_') ? vtopTaskLabel(step).replaceAll('...', '') : step;

/// How [line]'s request went: over UDP, or over HTTPS straight to VTOP,
/// through the server's /vtop/ proxy, or to the server's API.
String? lineVia(LogLine line) => switch (line.message) {
  _ when line.source != 'http' && line.source != 'rust.network' =>
    line.message.contains('UDP') ? 'UDP' : null,
  final text when text.startsWith('udp ') => 'UDP',
  final text when text.contains('vtop.vitap.ac.in') => 'HTTPS direct',
  final text when text.contains('/vtop/') => 'HTTPS proxy',
  final text when text.contains('/v1/') => 'HTTPS server',
  _ => 'HTTPS',
};
