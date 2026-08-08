import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/features/docs/data/doc_models.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';

class DocKindVisual {
  final List<Color> gradient;
  final Color accent;
  final IconData icon;
  const DocKindVisual(this.gradient, this.accent, this.icon);
}

DocKindVisual visualFor(DocWindow doc) {
  switch (doc.kind) {
    case DocKind.pdf:
      return const DocKindVisual(
        [Color(0xFFFFDAD6), Color(0xFFFFB4AB)],
        Color(0xFFB3261E),
        FLucideIcons.fileText,
      );
    case DocKind.image:
      return const DocKindVisual(
        [Color(0xFFC6E7FF), Color(0xFFB3DDFF)],
        Color(0xFF1976D2),
        FLucideIcons.image,
      );
    case DocKind.spreadsheet:
      return const DocKindVisual(
        [Color(0xFFD4F6DD), Color(0xFFC1EFCB)],
        Color(0xFF1B5E20),
        FLucideIcons.fileSpreadsheet,
      );
    case DocKind.text:
      return const DocKindVisual(
        [Color(0xFFE4D7F5), Color(0xFFD3C2EE)],
        Color(0xFF673AB7),
        FLucideIcons.fileCode2,
      );
    case DocKind.none:
      return const DocKindVisual(
        [Color(0xFFFFE8CD), Color(0xFFFFDDB3)],
        Color(0xFFE65100),
        FLucideIcons.utensils,
      );
  }
}

String kindLabel(DocKind kind) {
  switch (kind) {
    case DocKind.pdf:
      return 'PDF';
    case DocKind.image:
      return 'IMAGE';
    case DocKind.spreadsheet:
      return 'SHEET';
    case DocKind.text:
      return 'TEXT';
    case DocKind.none:
      return 'MENU';
  }
}

String lastOpenedLabel(int? ms) {
  if (ms == null || ms == 0) return 'never opened';
  final diff = DateTime.now().millisecondsSinceEpoch - ms;
  final mins = diff ~/ 60000;
  if (mins < 1) return 'opened just now';
  if (mins < 60) return 'opened ${mins}m ago';
  final hours = mins ~/ 60;
  if (hours < 24) return 'opened ${hours}h ago';
  return 'opened ${hours ~/ 24}d ago';
}

class DocCard extends ConsumerWidget {
  final DocWindow doc;
  final VoidCallback onOpen;
  final VoidCallback onRename;
  final VoidCallback onDelete;

  const DocCard({
    super.key,
    required this.doc,
    required this.onOpen,
    required this.onRename,
    required this.onDelete,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final darkMode = ref.watch(themeProvider) == ThemeMode.dark;
    final visual = visualFor(doc);

    return FTappable(
      onPress: onOpen,
      onLongPress: () => _showActions(context),
      child: Container(
        decoration: BoxDecoration(
          gradient: !darkMode && doc.hasFile
              ? LinearGradient(
                  colors: visual.gradient,
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                )
              : null,
          color: darkMode ? context.theme.colors.primaryForeground : null,
          borderRadius: BorderRadius.circular(16),
          border: darkMode
              ? Border.all(color: context.theme.colors.border)
              : null,
          boxShadow: const [
            BoxShadow(
              color: Color(0x1A000000),
              blurRadius: 10,
              offset: Offset(0, 4),
            ),
          ],
        ),
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Container(
                  padding: const EdgeInsets.all(8),
                  decoration: BoxDecoration(
                    color: darkMode
                        ? context.theme.colors.background
                        : Colors.white.withValues(alpha: 0.75),
                    borderRadius: BorderRadius.circular(10),
                  ),
                  child: Icon(visual.icon, size: 20, color: visual.accent),
                ),
                const Spacer(),
                Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 7,
                    vertical: 3,
                  ),
                  decoration: BoxDecoration(
                    color: visual.accent.withValues(alpha: 0.14),
                    borderRadius: BorderRadius.circular(999),
                    border: Border.all(
                      color: visual.accent.withValues(alpha: 0.35),
                    ),
                  ),
                  child: Text(
                    kindLabel(doc.kind),
                    style: TextStyle(
                      fontSize: 9,
                      fontWeight: FontWeight.w700,
                      letterSpacing: 0.5,
                      color: darkMode ? context.theme.colors.primary : visual.accent,
                    ),
                  ),
                ),
              ],
            ),
            const Spacer(),
            Text(
              doc.name,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(
                fontWeight: FontWeight.w700,
                fontSize: 14,
                height: 1.2,
                color: darkMode
                    ? context.theme.colors.primary
                    : const Color(0xFF212121),
              ),
            ),
            const SizedBox(height: 4),
            Text(
              doc.hasFile ? lastOpenedLabel(doc.lastOpenedAt) : 'Tap to add file',
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(
                fontSize: 11,
                fontWeight: FontWeight.w500,
                color: darkMode
                    ? context.theme.colors.mutedForeground
                    : const Color(0xFF616161),
              ),
            ),
          ],
        ),
      ),
    );
  }

  void _showActions(BuildContext context) {
    showFSheet(
      context: context,
      side: FLayout.btt,
      builder: (context) => Container(
        decoration: BoxDecoration(
          color: context.theme.colors.background,
          borderRadius: const BorderRadius.vertical(top: Radius.circular(10)),
        ),
        child: SafeArea(
          child: Padding(
            padding: const EdgeInsets.fromLTRB(12, 16, 12, 12),
            child: FTileGroup(
              divider: FItemDivider.indented,
              children: [
                FTile(
                  prefix: const Icon(FLucideIcons.pencilLine),
                  title: const Text('Rename'),
                  onPress: () {
                    Navigator.of(context).pop();
                    onRename();
                  },
                ),
                FTile(
                  prefix: const Icon(FLucideIcons.trash2),
                  title: const Text('Delete'),
                  onPress: () {
                    Navigator.of(context).pop();
                    onDelete();
                  },
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
