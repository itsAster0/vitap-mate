import 'dart:async';
import 'dart:convert';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:intl/intl.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/ui/app_card.dart';
import 'package:vitapmate/features/developer/log_flows.dart';
import 'package:vitapmate/observability/log.dart';
import 'package:vitapmate/ui/ui.dart';

enum _Tab { flows, lines }

/// Every log line on the phone (Dart's and Rust's). Flows groups them by user
/// action or background run, newest first; Lines lists them all. The filter
/// matches level, source, flow id or text.
class LogsPage extends HookWidget {
  const LogsPage({super.key});

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final reload = useState(0);
    final tab = useState(_Tab.flows);
    final problemsOnly = useState(false);
    final live = useState(false);
    final verbose = useState(Log.verbose);
    useEffect(() {
      if (!live.value) return null;
      final timer = Timer.periodic(
        const Duration(seconds: 2),
        (_) => reload.value++,
      );
      return timer.cancel;
    }, [live.value]);
    final lines = useFuture(useMemoized(Log.read, [reload.value])).data ?? [];
    final filter = useTextEditingController();
    useListenable(filter);
    final query = filter.text.trim().toLowerCase();

    bool matches(LogLine line) =>
        (!problemsOnly.value || _isProblem(line)) &&
        (query.isEmpty || _text(line).toLowerCase().contains(query));
    final shown = lines.reversed.where(matches).toList();
    final flows = [
      for (final flow in groupFlows(lines))
        if ((!problemsOnly.value || flow.problems) &&
            (flow.name.toLowerCase().contains(query) ||
                flow.lines.any(matches)))
          flow,
    ];
    final dayAgo = DateTime.now().subtract(const Duration(days: 1));
    final errors = lines
        .where((line) => line.level == 'error' && line.time.isAfter(dayAgo))
        .length;
    final export = shown.reversed.map(_text).join('\n');

    Future<void> save() async {
      final stamp = DateFormat('yyyy-MM-dd-HHmmss').format(DateTime.now());
      final saved = await FilePicker.saveFile(
        fileName: 'vitapmate-log-$stamp.txt',
        bytes: utf8.encode(export),
        mimeType: 'text/plain',
      );
      if (saved != null && context.mounted) {
        dispToast(context, 'Saved', 'The log is in the folder you picked.');
      }
    }

    FButton toggle(
      String label,
      ValueNotifier<bool> on, [
      VoidCallback? also,
    ]) => FButton(
      variant: on.value ? .primary : .outline,
      size: .sm,
      mainAxisSize: .min,
      onPress: () {
        on.value = !on.value;
        also?.call();
      },
      child: Text(label),
    );

    return Container(
      color: colors.background,
      padding: const .all(Space.sm),
      child: Column(
        crossAxisAlignment: .stretch,
        spacing: Space.sm,
        children: [
          Text(
            '${lines.length} lines · ${(Log.size / 1024).round()} KB · '
            '$errors ${errors == 1 ? 'error' : 'errors'} in 24 h',
            style: context.theme.typography.body.sm.copyWith(
              color: colors.mutedForeground,
            ),
          ),
          Segmented(
            segments: const [(_Tab.flows, 'Flows'), (_Tab.lines, 'Lines')],
            value: tab.value,
            onChanged: (value) => tab.value = value,
            counts: [flows.length, shown.length],
          ),
          FTextField(
            control: .managed(controller: filter),
            hint: 'Filter by level, source, flow or text',
          ),
          Wrap(
            spacing: Space.sm,
            runSpacing: Space.sm,
            children: [
              toggle('Problems', problemsOnly),
              toggle('Live', live),
              toggle(
                'Verbose',
                verbose,
                () => unawaited(Log.setVerbose(verbose.value)),
              ),
              FButton(
                variant: .outline,
                size: .sm,
                mainAxisSize: .min,
                onPress: shown.isEmpty
                    ? null
                    : () => Clipboard.setData(ClipboardData(text: export)),
                child: const Text('Copy'),
              ),
              FButton(
                variant: .outline,
                size: .sm,
                mainAxisSize: .min,
                onPress: shown.isEmpty ? null : save,
                child: const Text('Save'),
              ),
              FButton(
                variant: .destructive,
                size: .sm,
                mainAxisSize: .min,
                onPress: lines.isEmpty
                    ? null
                    : () async {
                        await Log.clear();
                        reload.value++;
                      },
                child: const Text('Clear'),
              ),
            ],
          ),
          Expanded(
            child: RefreshIndicator(
              onRefresh: () async => reload.value++,
              child: switch (tab.value) {
                _ when (tab.value == .flows ? flows : shown).isEmpty =>
                  ListView(
                    children: const [
                      SizedBox(height: Space.xxl),
                      Center(child: Text('No log lines yet.')),
                    ],
                  ),
                .flows => ListView.separated(
                  itemCount: flows.length,
                  separatorBuilder: (_, _) => const SizedBox(height: Space.sm),
                  itemBuilder: (context, i) =>
                      _FlowCard(flows[i], key: ValueKey(flows[i].id)),
                ),
                .lines => ListView.separated(
                  itemCount: shown.length,
                  separatorBuilder: (_, _) =>
                      Divider(height: Space.md, color: colors.border),
                  itemBuilder: (context, i) => _Line(shown[i]),
                ),
              },
            ),
          ),
        ],
      ),
    );
  }
}

/// A flow: status, how it reached VTOP, name, time taken and when; tap for
/// its lines.
class _FlowCard extends HookWidget {
  const _FlowCard(this.flow, {super.key});

  final LogFlow flow;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final open = useState(false);
    final (label, tone) = switch (flow.status) {
      .done => ('done', colors.app.success),
      .failed => ('failed', colors.app.danger),
      .running => ('running', colors.app.accentTone),
      .stopped => ('stopped', colors.app.warning),
    };
    return AppCard(
      child: Column(
        crossAxisAlignment: .stretch,
        spacing: Space.sm,
        children: [
          FTappable(
            onPress: () => open.value = !open.value,
            child: Column(
              crossAxisAlignment: .stretch,
              spacing: Space.xs,
              children: [
                Row(
                  crossAxisAlignment: .start,
                  spacing: Space.sm,
                  children: [
                    Expanded(
                      child: Text(
                        flow.name,
                        style: typography.body.sm.copyWith(fontWeight: .w600),
                        maxLines: open.value ? 3 : 1,
                        overflow: .ellipsis,
                      ),
                    ),
                    if (flow.ms case final ms?)
                      Text(
                        _duration(ms),
                        style: typography.body.sm.copyWith(color: tone.base),
                      ),
                  ],
                ),
                Wrap(
                  spacing: Space.xs,
                  runSpacing: Space.xs,
                  children: [
                    ToneBadge(label: label, tone: tone, solid: true),
                    for (final via in flow.via)
                      ToneBadge(label: via, tone: colors.app.accentTone),
                    if (flow.problems && flow.status == .done)
                      ToneBadge(label: 'warn', tone: colors.app.warning),
                  ],
                ),
                Text(
                  '${_time.format(flow.lines.first.time)} · '
                  '${flow.lines.length} lines · #${flow.id}',
                  style: typography.body.xs.copyWith(
                    color: colors.mutedForeground,
                  ),
                ),
              ],
            ),
          ),
          if (open.value) ...[
            Divider(height: 1, color: colors.border),
            for (final (i, line) in flow.lines.indexed) ...[
              if (i > 0) Divider(height: Space.md, color: colors.border),
              _Line(line, withFlow: false),
            ],
          ],
        ],
      ),
    );
  }
}

/// One line: time, level, source (and flow) and how its request went above
/// the message, warnings
/// and errors in their colour, with its stack if any.
class _Line extends StatelessWidget {
  const _Line(this.line, {this.withFlow = true});

  final LogLine line;
  final bool withFlow;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final color = switch (line.level) {
      'error' => colors.app.danger.base,
      'warn' => colors.app.warning.base,
      _ => colors.foreground,
    };
    final mono = typography.body.xs.copyWith(fontFamily: 'monospace');
    return Column(
      crossAxisAlignment: .stretch,
      spacing: 2,
      children: [
        Row(
          spacing: Space.sm,
          children: [
            Flexible(
              child: Text(
                [
                  DateFormat('HH:mm:ss.SSS').format(line.time),
                  line.level.toUpperCase(),
                  line.source,
                  if (line.flow case final flow? when withFlow) '#$flow',
                ].join('  '),
                style: mono.copyWith(color: colors.mutedForeground),
              ),
            ),
            if (lineVia(line) case final via?)
              ToneBadge(label: via, tone: colors.app.accentTone),
          ],
        ),
        SelectableText(
          [
            line.message,
            if (line.data['stack'] case final String stack) stack,
          ].join('\n'),
          style: mono.copyWith(color: color),
        ),
      ],
    );
  }
}

bool _isProblem(LogLine line) => line.level == 'warn' || line.level == 'error';

String _duration(int ms) =>
    ms < 1000 ? '${ms}ms' : '${(ms / 1000).toStringAsFixed(1)}s';

final _time = DateFormat('MM-dd HH:mm:ss.SSS');

String _text(LogLine line) => [
  _time.format(line.time),
  line.level.toUpperCase(),
  line.source,
  if (line.flow case final flow?) '#$flow',
  ?line.regno,
  line.message,
  if (line.data['stack'] case final String stack) '\n$stack',
].join(' ');
