import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/observability/app_logger.dart';
import 'package:vitapmate/src/api/native_logs.dart';

class LogsPage extends HookWidget {
  const LogsPage({super.key});

  @override
  Widget build(BuildContext context) {
    final clientLogger = AppLogger.instance;
    final rustLogs = useState<List<String>>(<String>[]);

    useEffect(() {
      void refresh() => rustLogs.value = nativeLogsGetEntries();
      refresh();
      return Timer.periodic(
        const Duration(seconds: 2),
        (_) => refresh(),
      ).cancel;
    }, const []);

    return Container(
      color: context.theme.colors.background,
      padding: const .all(8),
      child: FTabs(
        expands: true,
        children: [
          FTabEntry(
            label: const Text('Client Logs'),
            child: AnimatedBuilder(
              animation: clientLogger,
              builder: (context, _) => _LogsListView(
                lines: clientLogger.entries
                    .map((entry) => entry.toLine())
                    .toList(),
                onClear: clientLogger.clear,
              ),
            ),
          ),
          FTabEntry(
            label: const Text('Rust Logs'),
            child: _LogsListView(
              lines: rustLogs.value,
              onClear: () {
                nativeLogsClear();
                rustLogs.value = const [];
              },
            ),
          ),
        ],
      ),
    );
  }
}

class _LogsListView extends StatelessWidget {
  final List<String> lines;
  final VoidCallback onClear;

  const _LogsListView({required this.lines, required this.onClear});

  @override
  Widget build(BuildContext context) => Column(
    children: [
      Padding(
        padding: const .all(8),
        child: Row(
          mainAxisAlignment: .end,
          children: [
            FButton(
              variant: .outline,
              onPress: lines.isEmpty
                  ? null
                  : () => Clipboard.setData(
                      ClipboardData(text: lines.join('\n')),
                    ),
              child: const Text('Copy Logs'),
            ),
            const SizedBox(width: 8),
            FButton(
              variant: .destructive,
              onPress: lines.isEmpty ? null : onClear,
              child: const Text('Clear Logs'),
            ),
          ],
        ),
      ),
      Expanded(
        child: lines.isEmpty
            ? const Center(
                child: Text('No logs yet. Start using the app to see logs.'),
              )
            : ListView.separated(
                padding: const .all(8),
                itemCount: lines.length,
                separatorBuilder: (_, _) => const SizedBox(height: 8),
                itemBuilder: (context, index) =>
                    FCard(child: Text(lines[index])),
              ),
      ),
    ],
  );
}
