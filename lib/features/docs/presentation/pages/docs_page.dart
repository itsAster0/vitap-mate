import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/widgets/app_dialog.dart';
import 'package:vitapmate/features/docs/data/doc_models.dart';
import 'package:vitapmate/features/docs/presentation/pages/document_viewer_page.dart';
import 'package:vitapmate/features/docs/presentation/providers/docs_provider.dart';
import 'package:vitapmate/features/docs/presentation/widgets/doc_card.dart';

class DocsPage extends HookConsumerWidget {
  const DocsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final windowsAsync = ref.watch(docsRegistryProvider);
    final darkMode = ref.watch(themeProvider) == ThemeMode.dark;
    final entrance = useAnimationController(
      duration: const Duration(milliseconds: 450),
    )..forward();

    Future<void> importFlow([DocWindow? preset]) async {
      try {
        final pickedPath = await pickDocPath();
        if (pickedPath == null) return;
        if (preset != null) {
          await ref
              .read(docsRegistryProvider.notifier)
              .fillPreset(preset, pickedPath);
        } else {
          final suggested = pickedPath
              .split('/')
              .last
              .replaceAll(RegExp(r'\.[^.]+$'), '');
          await ref
              .read(docsRegistryProvider.notifier)
              .importNew(pickedPath, suggested);
        }
      } catch (e) {
        if (context.mounted) disCommonToast(context, e);
      }
    }

    void openRename(DocWindow doc) async {
      final controller = TextEditingController(text: doc.name);
      await showFDialog(
        context: context,
        builder: (context, style, animation) => AppDialog(
          animation: animation,
          direction: Axis.horizontal,
          title: const Text('Rename document'),
          body: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              FTextField(
                control: FTextFieldControl.managed(controller: controller),
              ),
            ],
          ),
          actions: [
            FButton(
              variant: FButtonVariant.outline,
              onPress: () => Navigator.of(context).pop(),
              child: const Text('Cancel'),
            ),
            FButton(
              onPress: () {
                Navigator.of(context).pop();
                ref
                    .read(docsRegistryProvider.notifier)
                    .rename(doc.id, controller.text);
              },
              child: const Text('Save'),
            ),
          ],
        ),
      );
    }

    void openDelete(DocWindow doc) async {
      await showFDialog(
        context: context,
        builder: (context, style, animation) => AppDialog(
          animation: animation,
          direction: Axis.horizontal,
          title: Text('Delete "${doc.name}"?'),
          body: const Text('The imported file will be removed from the app.'),
          actions: [
            FButton(
              variant: FButtonVariant.outline,
              onPress: () => Navigator.of(context).pop(),
              child: const Text('Cancel'),
            ),
            FButton(
              onPress: () {
                Navigator.of(context).pop();
                ref.read(docsRegistryProvider.notifier).remove(doc.id);
              },
              child: const Text('Delete'),
            ),
          ],
        ),
      );
    }

    void openDoc(DocWindow doc) {
      if (!doc.hasFile) {
        importFlow(doc);
        return;
      }
      GoRouter.of(context).pushNamed(Paths.docView, extra: doc);
    }

    return windowsAsync.when(
      loading: () => Center(
        child: CircularProgressIndicator(
          color: context.theme.colors.primary,
          strokeWidth: 3,
        ),
      ),
      error: (e, _) => _CenterInfo(
        icon: FLucideIcons.triangleAlert,
        title: 'Unable to load documents',
        subtitle: '$e',
      ),
      data: (windows) {
        final messPreset = windows
            .where((w) => w.isPreset && !w.hasFile)
            .firstOrNull;
        return AnimatedSwitcher(
          duration: const Duration(milliseconds: 250),
          child: windows.isEmpty
              ? _EmptyState(
                  key: const ValueKey('empty'),
                  onImport: () => importFlow(),
                  onMess: () => importFlow(messPreset),
                )
              : _Grid(
                  key: const ValueKey('grid'),
                  windows: windows,
                  darkMode: darkMode,
                  entrance: entrance,
                  onOpen: openDoc,
                  onRename: openRename,
                  onDelete: openDelete,
                  onImport: () => importFlow(),
                ),
        );
      },
    );
  }
}

class _Grid extends StatefulWidget {
  final List<DocWindow> windows;
  final bool darkMode;
  final AnimationController entrance;
  final void Function(DocWindow) onOpen;
  final void Function(DocWindow) onRename;
  final void Function(DocWindow) onDelete;
  final VoidCallback onImport;

  const _Grid({
    super.key,
    required this.windows,
    required this.darkMode,
    required this.entrance,
    required this.onOpen,
    required this.onRename,
    required this.onDelete,
    required this.onImport,
  });

  @override
  State<_Grid> createState() => _GridState();
}

class _GridState extends State<_Grid> {
  bool _previewActive = false;

  void _setPreviewActive(bool active) {
    if (_previewActive == active) return;
    setState(() => _previewActive = active);
  }

  @override
  Widget build(BuildContext context) {
    DocWindow? recent;
    for (final window in widget.windows) {
      if (!window.hasFile || window.lastOpenedAt == null) continue;
      if (recent == null || window.lastOpenedAt! > (recent.lastOpenedAt ?? 0)) {
        recent = window;
      }
    }

    return CustomScrollView(
      physics: _previewActive
          ? const NeverScrollableScrollPhysics()
          : const AlwaysScrollableScrollPhysics(),
      slivers: [
        SliverPadding(
          padding: const EdgeInsets.fromLTRB(0, 4, 0, 8),
          sliver: SliverToBoxAdapter(
            child: Row(
              children: [
                Text(
                  '${widget.windows.length} document${widget.windows.length == 1 ? '' : 's'}',
                  style: TextStyle(
                    fontSize: 13,
                    fontWeight: FontWeight.w600,
                    color: context.theme.colors.mutedForeground,
                  ),
                ),
                const Spacer(),
                FTappable(
                  onPress: widget.onImport,
                  child: Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 12,
                      vertical: 8,
                    ),
                    decoration: BoxDecoration(
                      color: context.theme.colors.primary,
                      borderRadius: BorderRadius.circular(10),
                    ),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(
                          FLucideIcons.filePlus2,
                          size: 16,
                          color: context.theme.colors.primaryForeground,
                        ),
                        const SizedBox(width: 6),
                        Text(
                          'Add',
                          style: TextStyle(
                            fontSize: 13,
                            fontWeight: FontWeight.w600,
                            color: context.theme.colors.primaryForeground,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
        if (recent != null)
          SliverPadding(
            padding: const EdgeInsets.only(bottom: 14),
            sliver: SliverToBoxAdapter(
              child: _RecentDocumentPreview(
                doc: recent,
                darkMode: widget.darkMode,
                onOpen: () => widget.onOpen(recent!),
                onInteractionStart: () => _setPreviewActive(true),
                onInteractionEnd: () => _setPreviewActive(false),
              ),
            ),
          ),
        SliverGrid.builder(
          itemCount: widget.windows.length,
          gridDelegate: const SliverGridDelegateWithMaxCrossAxisExtent(
            maxCrossAxisExtent: 200,
            mainAxisSpacing: 10,
            crossAxisSpacing: 10,
            childAspectRatio: 0.92,
          ),
          itemBuilder: (context, i) {
            final doc = widget.windows[i];
            final start = (i * 0.06).clamp(0.0, 0.6);
            final anim = CurvedAnimation(
              parent: widget.entrance,
              curve: Interval(start, start + 0.4, curve: Curves.easeOutCubic),
            );
            return RepaintBoundary(
              key: ValueKey(doc.id),
              child: FadeTransition(
                opacity: anim,
                child: SlideTransition(
                  position: Tween<Offset>(
                    begin: const Offset(0, 0.12),
                    end: Offset.zero,
                  ).animate(anim),
                  child: DocCard(
                    doc: doc,
                    onOpen: () => widget.onOpen(doc),
                    onRename: () => widget.onRename(doc),
                    onDelete: () => widget.onDelete(doc),
                  ),
                ),
              ),
            );
          },
        ),
      ],
    );
  }
}

class _RecentDocumentPreview extends StatelessWidget {
  final DocWindow doc;
  final bool darkMode;
  final VoidCallback onOpen;
  final VoidCallback onInteractionStart;
  final VoidCallback onInteractionEnd;

  const _RecentDocumentPreview({
    required this.doc,
    required this.darkMode,
    required this.onOpen,
    required this.onInteractionStart,
    required this.onInteractionEnd,
  });

  @override
  Widget build(BuildContext context) {
    final visual = visualFor(doc);

    return Container(
      decoration: BoxDecoration(
        color: darkMode ? context.theme.colors.primaryForeground : Colors.white,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: context.theme.colors.border),
        boxShadow: const [
          BoxShadow(
            color: Color(0x18000000),
            blurRadius: 12,
            offset: Offset(0, 4),
          ),
        ],
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(14, 12, 10, 12),
            child: Row(
              children: [
                Container(
                  padding: const EdgeInsets.all(8),
                  decoration: BoxDecoration(
                    color: visual.accent.withValues(alpha: 0.12),
                    borderRadius: BorderRadius.circular(10),
                  ),
                  child: Icon(visual.icon, size: 19, color: visual.accent),
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        'Continue viewing',
                        style: TextStyle(
                          fontSize: 11,
                          fontWeight: FontWeight.w700,
                          letterSpacing: 0.3,
                          color: context.theme.colors.mutedForeground,
                        ),
                      ),
                      const SizedBox(height: 2),
                      Text(
                        doc.name,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(
                          fontSize: 14,
                          fontWeight: FontWeight.w700,
                          color: context.theme.colors.primary,
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 8),
                FTappable(
                  onPress: onOpen,
                  child: Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 10,
                      vertical: 8,
                    ),
                    decoration: BoxDecoration(
                      color: context.theme.colors.primary,
                      borderRadius: BorderRadius.circular(10),
                    ),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(
                          FLucideIcons.externalLink,
                          size: 15,
                          color: context.theme.colors.primaryForeground,
                        ),
                        const SizedBox(width: 6),
                        Text(
                          'Full screen',
                          style: TextStyle(
                            fontSize: 12,
                            fontWeight: FontWeight.w700,
                            color: context.theme.colors.primaryForeground,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ],
            ),
          ),
          Divider(height: 1, thickness: 1, color: context.theme.colors.border),
          LayoutBuilder(
            builder: (context, constraints) => SizedBox(
              height: constraints.maxWidth >= 700 ? 420 : 310,
              child: MouseRegion(
                onEnter: (_) => onInteractionStart(),
                onExit: (_) => onInteractionEnd(),
                child: Listener(
                  behavior: HitTestBehavior.opaque,
                  onPointerDown: (_) => onInteractionStart(),
                  onPointerUp: (_) => onInteractionEnd(),
                  onPointerCancel: (_) => onInteractionEnd(),
                  child: ClipRRect(
                    borderRadius: const BorderRadius.vertical(
                      bottom: Radius.circular(15),
                    ),
                    child: ColoredBox(
                      color: context.theme.colors.background,
                      child: DocumentViewerPage(
                        key: ValueKey('recent-${doc.id}'),
                        doc: doc,
                        embedded: true,
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _EmptyState extends ConsumerWidget {
  final VoidCallback onImport;
  final VoidCallback onMess;
  const _EmptyState({super.key, required this.onImport, required this.onMess});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Center(
      child: SingleChildScrollView(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Container(
              padding: const EdgeInsets.all(24),
              decoration: BoxDecoration(
                color: context.theme.colors.primary.withValues(alpha: 0.1),
                shape: BoxShape.circle,
              ),
              child: Icon(
                FLucideIcons.files,
                size: 44,
                color: context.theme.colors.primary,
              ),
            ),
            const SizedBox(height: 18),
            Text(
              'Your personal board',
              style: TextStyle(
                fontSize: 17,
                fontWeight: FontWeight.w700,
                color: context.theme.colors.primary,
              ),
            ),
            const SizedBox(height: 8),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 40),
              child: Text(
                'Import a PDF or image — like your mess menu — and it stays here, right where you left off.',
                textAlign: TextAlign.center,
                style: TextStyle(
                  fontSize: 13.5,
                  height: 1.4,
                  color: context.theme.colors.mutedForeground,
                ),
              ),
            ),
            const SizedBox(height: 20),
            FButton(
              onPress: () => onImport(),
              child: const Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(FLucideIcons.filePlus2, size: 16),
                  SizedBox(width: 8),
                  Text('Import a document'),
                ],
              ),
            ),
            const SizedBox(height: 14),
            FTappable(
              onPress: onMess,
              child: Text(
                'or add straight to Mess Menu',
                style: TextStyle(
                  fontSize: 12.5,
                  fontWeight: FontWeight.w600,
                  color: context.theme.colors.primary,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _CenterInfo extends StatelessWidget {
  final IconData icon;
  final String title;
  final String? subtitle;

  const _CenterInfo({required this.icon, required this.title, this.subtitle});

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(icon, size: 28, color: context.theme.colors.mutedForeground),
          const SizedBox(height: 10),
          Text(
            title,
            style: TextStyle(
              fontWeight: FontWeight.w600,
              color: context.theme.colors.primary,
            ),
          ),
          if (subtitle != null) ...[
            const SizedBox(height: 6),
            Text(
              subtitle!,
              textAlign: TextAlign.center,
              style: TextStyle(color: context.theme.colors.mutedForeground),
            ),
          ],
        ],
      ),
    );
  }
}
