import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/docs/logic/doc_models.dart';

class DocKindVisual {
  final Tone tone;
  final IconData icon;
  const DocKindVisual(this.tone, this.icon);

  Color get accent => tone.base;
}

DocKindVisual visualFor(BuildContext context, DocWindow doc) {
  final colors = context.theme.colors;
  final palette = colors.app;
  if (doc.asMessMenu) {
    return DocKindVisual(palette.warning, FLucideIcons.utensils);
  }
  return switch (doc.kind) {
    .pdf => DocKindVisual(palette.danger, FLucideIcons.fileText),
    .image => DocKindVisual(palette.accentTone, FLucideIcons.image),
    .spreadsheet => DocKindVisual(
      palette.success,
      FLucideIcons.fileSpreadsheet,
    ),
    .text => DocKindVisual(palette.lab, FLucideIcons.fileCode2),
    .file => DocKindVisual(
      Tone(
        base: colors.mutedForeground,
        subtle: colors.secondary,
        onSubtle: colors.foreground,
      ),
      FLucideIcons.file,
    ),
    .none => DocKindVisual(palette.warning, FLucideIcons.utensils),
  };
}

String kindLabel(DocKind kind) => switch (kind) {
  .pdf => 'PDF',
  .image => 'IMAGE',
  .spreadsheet => 'SHEET',
  .text => 'TEXT',
  .file => 'FILE',
  .none => 'MENU',
};

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

  /// Switches a spreadsheet between the mess menu and table views.
  final VoidCallback onToggleMessMenu;

  const DocCard({
    super.key,
    required this.doc,
    required this.onOpen,
    required this.onRename,
    required this.onDelete,
    required this.onToggleMessMenu,
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
        padding: const .all(Space.md + 2),
        child: Column(
          crossAxisAlignment: .start,
          children: [
            Row(
              children: [
                Container(
                  width: 36,
                  height: 36,
                  decoration: BoxDecoration(
                    color: visual.tone.subtle,
                    borderRadius: .circular(Radii.sm + 2),
                  ),
                  child: Icon(visual.icon, size: 18, color: visual.tone.base),
                ),
                const Spacer(),
                ToneBadge.neutral(
                  context,
                  doc.asMessMenu ? 'MENU' : kindLabel(doc.kind),
                ),
              ],
            ),
            const Spacer(),
            Text(
              doc.name,
              maxLines: 2,
              overflow: .ellipsis,
              style: typography.body.sm.copyWith(
                height: 1.25,
                fontWeight: .w600,
                color: colors.foreground,
              ),
            ),
            const SizedBox(height: 2),
            Text(
              doc.hasFile
                  ? lastOpenedLabel(doc.lastOpenedAt)
                  : 'Tap to add a file',
              maxLines: 1,
              overflow: .ellipsis,
              style: typography.body.xs.copyWith(color: colors.mutedForeground),
            ),
          ],
        ),
      ),
    );
  }

  void _showActions(BuildContext context) => showFSheet(
    context: context,
    side: .btt,
    builder: (context) {
      /// Closes the sheet, then runs [action].
      VoidCallback then(VoidCallback action) => () {
        Navigator.of(context).pop();
        action();
      };
      return Container(
        decoration: BoxDecoration(
          color: context.theme.colors.background,
          borderRadius: const .vertical(top: Radius.circular(10)),
        ),
        child: SafeArea(
          child: Padding(
            padding: const .fromLTRB(12, 16, 12, 12),
            child: FTileGroup(
              divider: .indented,
              children: [
                FTile(
                  prefix: const Icon(FLucideIcons.pencilLine),
                  title: const Text('Rename'),
                  onPress: then(onRename),
                ),
                if (doc.kind == .spreadsheet)
                  FTile(
                    prefix: Icon(
                      doc.asMessMenu
                          ? FLucideIcons.sheet
                          : FLucideIcons.utensils,
                    ),
                    title: Text(
                      doc.asMessMenu
                          ? 'Show as spreadsheet'
                          : 'Show as mess menu',
                    ),
                    subtitle: const Text("The file itself isn't changed"),
                    onPress: then(onToggleMessMenu),
                  ),
                FTile(
                  prefix: const Icon(FLucideIcons.trash2),
                  title: const Text('Delete'),
                  onPress: then(onDelete),
                ),
              ],
            ),
          ),
        ),
      );
    },
  );
}
