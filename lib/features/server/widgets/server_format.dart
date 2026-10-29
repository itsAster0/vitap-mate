import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/ui.dart';

/// "just now", "5 min ago", "3 h ago", "2 d ago".
String ago(DateTime time, {DateTime? now}) {
  final elapsed = (now ?? DateTime.now()).difference(time);
  if (elapsed.inMinutes < 1) return 'just now';
  if (elapsed.inHours < 1) return '${elapsed.inMinutes} min ago';
  if (elapsed.inDays < 1) return '${elapsed.inHours} h ago';
  return '${elapsed.inDays} d ago';
}

/// The shell header's title for a path under `/settings/server`.
String serverHeaderTitle(String path) => switch (path.split('/').last) {
  'apps' => 'Connected apps',
  'chrome-extension' => 'Browser extension',
  'phones' => 'Linked phones',
  'address' => 'Server address',
  'data' => 'Your data on the server',
  _ => 'Server',
};

/// A Server page body: [children] in a scroll. Its [title] is shown by the
/// shell header ([serverHeaderTitle]).
class ServerSubPage extends StatelessWidget {
  const ServerSubPage({super.key, required this.title, required this.children});

  final String title;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) => SingleChildScrollView(
    padding: const .fromLTRB(Space.md, 0, Space.md, Space.xxl),
    child: Column(crossAxisAlignment: .stretch, children: children),
  );
}

/// The muted note under a group of tiles.
class ServerNote extends StatelessWidget {
  const ServerNote(this.text, {super.key});

  final String text;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const .fromLTRB(4, Space.sm, 4, 0),
    child: Text(
      text,
      style: context.theme.typography.body.sm.copyWith(
        color: context.theme.colors.mutedForeground,
      ),
    ),
  );
}

/// Label and value lines under a group of tiles, for technical detail.
class ServerFacts extends StatelessWidget {
  const ServerFacts(this.facts, {super.key});

  final List<(String, String)> facts;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final small = context.theme.typography.body.sm;
    return Padding(
      padding: const .fromLTRB(4, Space.sm, 4, 0),
      child: Column(
        crossAxisAlignment: .start,
        children: [
          for (final (label, value) in facts)
            Padding(
              padding: const .symmetric(vertical: 2),
              child: Row(
                crossAxisAlignment: .start,
                children: [
                  SizedBox(
                    width: 104,
                    child: Text(
                      label,
                      style: small.copyWith(color: colors.mutedForeground),
                    ),
                  ),
                  Expanded(child: Text(value, style: small)),
                ],
              ),
            ),
        ],
      ),
    );
  }
}
