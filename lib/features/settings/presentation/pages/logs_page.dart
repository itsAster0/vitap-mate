import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/core/logging/app_logger.dart';
import 'package:vitapmate/src/api/native_logs.dart';

class LogsPage extends HookWidget {
  const LogsPage({super.key});

  @override
  Widget build(BuildContext context) {
    final clientLogger = AppLogger.instance;
    final rustLogs = useState<List<String>>(<String>[]);

    Future<void> refreshRustLogs() async {
      rustLogs.value = nativeLogsGetEntries();
    }

    useEffect(() {
      unawaited(refreshRustLogs());
      final timer = Timer.periodic(const Duration(seconds: 2), (_) {
        unawaited(refreshRustLogs());
      });
      return timer.cancel;
    }, const []);

    return Container(
      color: context.theme.colors.background,
      child: Padding(
        padding: const EdgeInsets.all(8),
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
      ),
    );
  }
}

class _LogsListView extends StatelessWidget {
  final List<String> lines;
  final VoidCallback onClear;

  const _LogsListView({required this.lines, required this.onClear});

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.all(8),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.end,
            children: [
              FButton(
                variant: FButtonVariant.outline,
                onPress: lines.isEmpty
                    ? null
                    : () async {
                        await Clipboard.setData(
                          ClipboardData(text: lines.join('\n')),
                        );
                      },
                child: const Text('Copy Logs'),
              ),
              const SizedBox(width: 8),
              FButton(
                variant: FButtonVariant.destructive,
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
                  padding: const EdgeInsets.all(8),
                  itemCount: lines.length,
                  separatorBuilder: (_, _) => const SizedBox(height: 8),
                  itemBuilder: (context, index) {
                    final line = lines[index];
                    return FCard(child: Text(line));
                  },
                ),
        ),
      ],
    );
  }
}
