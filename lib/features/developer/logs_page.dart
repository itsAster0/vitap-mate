import 'dart:convert';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:intl/intl.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/observability/log.dart';

/// Every log line on the phone (Dart's and Rust's), newest first. The
/// filter matches level, source, flow id or text.
class LogsPage extends HookWidget {
  const LogsPage({super.key});

  @override
  Widget build(BuildContext context) {
    final reload = useState(0);
    final lines = useFuture(useMemoized(Log.read, [reload.value]));
    final filter = useTextEditingController();
    useListenable(filter);
    final query = filter.text.trim().toLowerCase();
    final shown = [
      for (final line in (lines.data ?? const <LogLine>[]).reversed)
        if (query.isEmpty || _text(line).toLowerCase().contains(query)) line,
    ];
    final all = shown.map(_text).join('\n');

    Future<void> save() async {
      final stamp = DateFormat('yyyy-MM-dd-HHmmss').format(DateTime.now());
      final saved = await FilePicker.saveFile(
        fileName: 'vitapmate-log-$stamp.txt',
        bytes: utf8.encode(all),
        mimeType: 'text/plain',
      );
      if (saved != null && context.mounted) {
        dispToast(context, 'Saved', 'The log is in the folder you picked.');
      }
    }

    return Container(
      color: context.theme.colors.background,
      padding: const .all(8),
      child: Column(
        children: [
          Padding(
            padding: const .all(8),
            child: FTextField(
              control: .managed(controller: filter),
              hint: 'Filter by level, source, flow or text',
            ),
          ),
          Padding(
            padding: const .symmetric(horizontal: 8),
            child: Row(
              mainAxisAlignment: .end,
              spacing: 8,
              children: [
                FButton(
                  variant: .outline,
                  onPress: shown.isEmpty
                      ? null
                      : () => Clipboard.setData(ClipboardData(text: all)),
                  child: const Text('Copy'),
                ),
                FButton(
                  variant: .outline,
                  onPress: shown.isEmpty ? null : save,
                  child: const Text('Save'),
                ),
                FButton(
                  variant: .destructive,
                  onPress: shown.isEmpty
                      ? null
                      : () async {
                          await Log.clear();
                          reload.value++;
                        },
                  child: const Text('Clear'),
                ),
              ],
            ),
          ),
          Expanded(
            child: shown.isEmpty
                ? const Center(child: Text('No log lines yet.'))
                : RefreshIndicator(
                    onRefresh: () async => reload.value++,
                    child: ListView.separated(
                      padding: const .all(8),
                      itemCount: shown.length,
                      separatorBuilder: (_, _) => const SizedBox(height: 8),
                      itemBuilder: (context, i) =>
                          FCard(child: SelectableText(_text(shown[i]))),
                    ),
                  ),
          ),
        ],
      ),
    );
  }
}

final _time = DateFormat('MM-dd HH:mm:ss.SSS');

String _text(LogLine line) => [
  _time.format(line.time),
  line.level.toUpperCase(),
  line.source,
  if (line.flow case final flow?) '#$flow',
  ?line.regno,
  line.message,
].join(' ');
