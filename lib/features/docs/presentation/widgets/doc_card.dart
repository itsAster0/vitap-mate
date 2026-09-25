import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/docs/data/doc_models.dart';

class DocKindVisual {
  final Tone tone;
  final IconData icon;
  const DocKindVisual(this.tone, this.icon);

  Color get accent => tone.base;
}

DocKindVisual visualFor(BuildContext context, DocWindow doc) {
  final colors = context.theme.colors;
  final palette = colors.app;
  return switch (doc.kind) {
    DocKind.pdf => DocKindVisual(palette.danger, FLucideIcons.fileText),
    DocKind.image => DocKindVisual(palette.accentTone, FLucideIcons.image),
    DocKind.spreadsheet => DocKindVisual(
      palette.success,
      FLucideIcons.fileSpreadsheet,
    ),
    DocKind.text => DocKindVisual(palette.lab, FLucideIcons.fileCode2),
    DocKind.file => DocKindVisual(
      Tone(
        base: colors.mutedForeground,
        subtle: colors.secondary,
        onSubtle: colors.foreground,
      ),
      FLucideIcons.file,
    ),
    DocKind.none => DocKindVisual(palette.warning, FLucideIcons.utensils),
  };
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
    case DocKind.file:
      return 'FILE';
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

class DocCard extends StatelessWidget {
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
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final visual = visualFor(context, doc);

    return FTappable(
      onPress: onOpen,
      onLongPress: () => _showActions(context),
      builder: (context, variants, child) => AnimatedScale(
        scale: variants.contains(FTappableVariant.pressed) ? 0.97 : 1,
        duration: Motion.fast,
        child: child,
      ),
      child: Surface(
        padding: const EdgeInsets.all(Space.md + 2),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Container(
                  width: 36,
                  height: 36,
                  decoration: BoxDecoration(
                    color: visual.tone.subtle,
                    borderRadius: BorderRadius.circular(Radii.sm + 2),
                  ),
                  child: Icon(visual.icon, size: 18, color: visual.tone.base),
                ),
                const Spacer(),
                ToneBadge.neutral(context, kindLabel(doc.kind)),
              ],
            ),
            const Spacer(),
            Text(
              doc.name,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: typography.body.sm.copyWith(
                height: 1.25,
                fontWeight: FontWeight.w600,
                color: colors.foreground,
              ),
            ),
            const SizedBox(height: 2),
            Text(
              doc.hasFile
                  ? lastOpenedLabel(doc.lastOpenedAt)
                  : 'Tap to add a file',
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: typography.body.xs.copyWith(color: colors.mutedForeground),
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
