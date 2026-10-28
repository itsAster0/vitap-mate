import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:open_file/open_file.dart';
import 'package:vitapmate/app/router/paths.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/ui/app_dialog.dart';
import 'package:vitapmate/features/docs/logic/doc_models.dart';
import 'package:vitapmate/features/docs/logic/docs_query.dart';
import 'package:vitapmate/features/docs/logic/mess_menu.dart';
import 'package:vitapmate/features/docs/docs_sort_provider.dart';
import 'package:vitapmate/features/docs/document_viewer_page.dart';
import 'package:vitapmate/features/docs/docs_provider.dart';
import 'package:vitapmate/features/docs/widgets/doc_card.dart';

class DocsPage extends HookConsumerWidget {
  const DocsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final windowsAsync = ref.watch(docsRegistryProvider);
    final sortAsync = ref.watch(docsSortProvider);
    final sort = sortAsync.value ?? DocSort.recentlyOpened;
    final searchController = useTextEditingController();
    useListenable(searchController);
    final searching = searchController.text.trim().isNotEmpty;

    Future<void> selectSort(DocSort value) async {
      try {
        await ref.read(docsSortProvider.notifier).select(value);
      } catch (_) {
        if (context.mounted) {
          dispToast(
            context,
            'Unable to save sort order',
            'Your selection applies now, but may reset next time.',
          );
        }
      }
    }

    final entrance = useAnimationController(
      duration: const Duration(milliseconds: 450),
    )..forward();

    Future<void> setMessMenu(DocWindow doc, bool value) async {
      try {
        await ref.read(docsRegistryProvider.notifier).setMessMenu(doc, value);
      } on MessMenuFormatException catch (e) {
        if (context.mounted) {
          dispToast(
            context,
            'Not a mess menu',
            '${e.message} It stays a spreadsheet.',
          );
        }
      } catch (e) {
        if (context.mounted) disCommonToast(context, e);
      }
    }

    Future<void> importFlow({bool asMessMenu = false}) async {
      try {
        final pickedPath = await pickDocPath();
        if (pickedPath == null) return;
        final suggested = pickedPath
            .split('/')
            .last
            .replaceAll(RegExp(r'\.[^.]+$'), '');
        final doc = await ref
            .read(docsRegistryProvider.notifier)
            .importNew(pickedPath, suggested);
        if (asMessMenu && doc != null) {
          if (doc.kind == .spreadsheet) {
            await setMessMenu(doc, true);
          } else if (context.mounted) {
            dispToast(
              context,
              'Added as a document',
              'Only the .xlsx mess menu can be shown as a menu.',
            );
          }
        }
      } catch (e) {
        if (context.mounted) disCommonToast(context, e);
      }
    }

    /// A Cancel / [action] dialog; [onAction] runs after it closes.
    void ask(Widget title, Widget body, String action, VoidCallback onAction) =>
        showFDialog(
          context: context,
          // Above the tab shell, so the keyboard inset is counted once.
          useRootNavigator: true,
          builder: (context, style, animation) => AppDialog(
            animation: animation,
            direction: .horizontal,
            title: title,
            body: body,
            actions: [
              FButton(
                variant: .outline,
                onPress: () => Navigator.of(context).pop(),
                child: const Text('Cancel'),
              ),
              FButton(
                onPress: () {
                  Navigator.of(context).pop();
                  onAction();
                },
                child: Text(action),
              ),
            ],
          ),
        );

    void openRename(DocWindow doc) {
      final controller = TextEditingController(text: doc.name);
      ask(
        const Text('Rename document'),
        Column(
          mainAxisSize: .min,
          crossAxisAlignment: .start,
          children: [FTextField(control: .managed(controller: controller))],
        ),
        'Save',
        () => ref
            .read(docsRegistryProvider.notifier)
            .rename(doc.id, controller.text),
      );
    }

    void openDelete(DocWindow doc) => ask(
      Text('Delete "${doc.name}"?'),
      const Text('The imported file will be removed from the app.'),
      'Delete',
      () => ref.read(docsRegistryProvider.notifier).remove(doc.id),
    );

    Future<void> openDoc(DocWindow doc) async {
      if (doc.kind == .file) {
        try {
          final repo = await ref.read(docsRepositoryProvider.future);
          final path = await repo.storedFilePathOf(doc);
          if (path == null) throw StateError('Document file is missing.');
          final result = await OpenFile.open(path);
          if (result.type != ResultType.done) {
            throw StateError(result.message);
          }
          if (context.mounted) {
            await ref
                .read(docsRegistryProvider.notifier)
                .touchLastOpened(doc.id);
          }
        } catch (error) {
          if (context.mounted) disCommonToast(context, error);
        }
        return;
      }
      GoRouter.of(context).pushNamed(Paths.docView, extra: doc);
    }

    return Padding(
      padding: const .symmetric(horizontal: Space.sm),
      child: Column(
        children: [
          Padding(
            padding: const .only(bottom: 12),
            child: Row(
              children: [
                Expanded(
                  child: FTextField(
                    control: .managed(controller: searchController),
                    hint: 'Search documents',
                    prefixBuilder: (_, _, _) => Padding(
                      padding: const .only(left: 12, right: 8),
                      child: Icon(
                        FLucideIcons.search,
                        size: 18,
                        color: context.theme.colors.mutedForeground,
                      ),
                    ),
                    suffixBuilder: (_, _, _) => searchController.text.isEmpty
                        ? const SizedBox.shrink()
                        : IconButton(
                            tooltip: 'Clear search',
                            onPressed: searchController.clear,
                            icon: Icon(
                              FLucideIcons.x,
                              size: 18,
                              color: context.theme.colors.mutedForeground,
                            ),
                          ),
                  ),
                ),
                const SizedBox(width: 8),
                FPopoverMenu(
                  menuAnchor: .topLeft,
                  childAnchor: .bottomLeft,
                  menu: [
                    FItemGroup(
                      children: [
                        for (final option in DocSort.values)
                          FItem(
                            title: Text(option.label),
                            prefix: Icon(
                              option == sort
                                  ? FLucideIcons.check
                                  : FLucideIcons.arrowUpDown,
                            ),
                            onPress: () => selectSort(option),
                          ),
                      ],
                    ),
                  ],
                  builder: (_, controller, _) => FButton(
                    variant: .outline,
                    onPress: sortAsync.isLoading ? null : controller.toggle,
                    child: const Text('Sort'),
                  ),
                ),
              ],
            ),
          ),
          Expanded(
            child: windowsAsync.when(
              loading: () => const Spinner(),
              error: (e, _) => _CenterInfo(
                icon: FLucideIcons.triangleAlert,
                title: 'Unable to load documents',
                subtitle: '$e',
              ),
              data: (windows) => AnimatedSwitcher(
                duration: const Duration(milliseconds: 250),
                child: windows.isEmpty
                    ? _EmptyState(
                        key: const ValueKey('empty'),
                        onImport: () => importFlow(),
                        onMess: () => importFlow(asMessMenu: true),
                      )
                    : _Grid(
                        key: const ValueKey('grid'),
                        windows: queryDocs(
                          windows,
                          sort: sort,
                          query: searchController.text,
                        ),
                        searching: searching,
                        totalCount: windows.length,
                        onClearSearch: searchController.clear,
                        entrance: entrance,
                        onOpen: openDoc,
                        onRename: openRename,
                        onDelete: openDelete,
                        onToggleMessMenu: (doc) =>
                            setMessMenu(doc, !doc.asMessMenu),
                        onImport: () => importFlow(),
                      ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _Grid extends StatefulWidget {
  final List<DocWindow> windows;
  final bool searching;
  final int totalCount;
  final VoidCallback onClearSearch;
  final AnimationController entrance;
  final void Function(DocWindow) onOpen;
  final void Function(DocWindow) onRename;
  final void Function(DocWindow) onDelete;
  final void Function(DocWindow) onToggleMessMenu;
  final VoidCallback onImport;

  const _Grid({
    super.key,
    required this.windows,
    required this.searching,
    required this.totalCount,
    required this.onClearSearch,
    required this.entrance,
    required this.onOpen,
    required this.onRename,
    required this.onDelete,
    required this.onToggleMessMenu,
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
    final recent = widget.windows
        .where((w) => w.hasFile && w.kind != .file && w.lastOpenedAt != null)
        .fold<DocWindow?>(
          null,
          (best, w) =>
              best == null || w.lastOpenedAt! > (best.lastOpenedAt ?? 0)
              ? w
              : best,
        );

    return CustomScrollView(
      physics: _previewActive && !widget.searching
          ? const NeverScrollableScrollPhysics()
          : const AlwaysScrollableScrollPhysics(),
      slivers: [
        SliverPadding(
          padding: const .fromLTRB(0, 4, 0, 8),
          sliver: SliverToBoxAdapter(
            child: Row(
              children: [
                Text(
                  widget.searching
                      ? '${widget.windows.length} of ${widget.totalCount} documents'
                      : '${widget.windows.length} document${widget.windows.length == 1 ? '' : 's'}',
                  style: TextStyle(
                    fontSize: 13,
                    fontWeight: .w600,
                    color: context.theme.colors.mutedForeground,
                  ),
                ),
                const Spacer(),
                FTappable(
                  onPress: widget.onImport,
                  child: Container(
                    padding: const .symmetric(horizontal: 12, vertical: 8),
                    decoration: BoxDecoration(
                      color: context.theme.colors.primary,
                      borderRadius: .circular(10),
                    ),
                    child: Row(
                      mainAxisSize: .min,
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
                            fontWeight: .w600,
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
        if (recent != null && !widget.searching)
          SliverPadding(
            padding: const .only(bottom: 14),
            sliver: SliverToBoxAdapter(
              child: _RecentDocumentPreview(
                doc: recent,
                onOpen: () => widget.onOpen(recent),
                onInteractionStart: () => _setPreviewActive(true),
                onInteractionEnd: () => _setPreviewActive(false),
              ),
            ),
          ),
        if (widget.windows.isEmpty)
          SliverFillRemaining(
            hasScrollBody: false,
            child: Column(
              mainAxisAlignment: .center,
              children: [
                const _CenterInfo(
                  icon: FLucideIcons.search,
                  title: 'No documents found',
                  subtitle: 'Try a different name or clear your search.',
                ),
                const SizedBox(height: 16),
                FButton(
                  variant: .outline,
                  onPress: widget.onClearSearch,
                  child: const Text('Clear search'),
                ),
              ],
            ),
          ),
        SliverGrid.builder(
          itemCount: widget.windows.length,
          gridDelegate: const SliverGridDelegateWithMaxCrossAxisExtent(
            maxCrossAxisExtent: 200,
            mainAxisSpacing: 10,
            crossAxisSpacing: 10,
            // Fits icon, two-line name and subtitle without a dead middle.
            mainAxisExtent: 140,
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
                    onToggleMessMenu: () => widget.onToggleMessMenu(doc),
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
  final VoidCallback onOpen;
  final VoidCallback onInteractionStart;
  final VoidCallback onInteractionEnd;

  const _RecentDocumentPreview({
    required this.doc,
    required this.onOpen,
    required this.onInteractionStart,
    required this.onInteractionEnd,
  });

  @override
  Widget build(BuildContext context) {
    final visual = visualFor(context, doc);

    return Container(
      decoration: BoxDecoration(
        color: context.theme.colors.card,
        borderRadius: .circular(16),
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
        crossAxisAlignment: .stretch,
        children: [
          Padding(
            padding: const .fromLTRB(14, 12, 10, 12),
            child: Row(
              children: [
                Container(
                  padding: const .all(8),
                  decoration: BoxDecoration(
                    color: visual.accent.withValues(alpha: 0.12),
                    borderRadius: .circular(10),
                  ),
                  child: Icon(visual.icon, size: 19, color: visual.accent),
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: .start,
                    children: [
                      Text(
                        'Continue viewing',
                        style: TextStyle(
                          fontSize: 11,
                          fontWeight: .w700,
                          letterSpacing: 0.3,
                          color: context.theme.colors.mutedForeground,
                        ),
                      ),
                      const SizedBox(height: 2),
                      Text(
                        doc.name,
                        maxLines: 1,
                        overflow: .ellipsis,
                        style: TextStyle(
                          fontSize: 14,
                          fontWeight: .w700,
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
                    padding: const .symmetric(horizontal: 10, vertical: 8),
                    decoration: BoxDecoration(
                      color: context.theme.colors.primary,
                      borderRadius: .circular(10),
                    ),
                    child: Row(
                      mainAxisSize: .min,
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
                            fontWeight: .w700,
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
              height: constraints.maxWidth >= 700 ? 320 : 210,
              child: MouseRegion(
                onEnter: (_) => onInteractionStart(),
                onExit: (_) => onInteractionEnd(),
                child: Listener(
                  behavior: .opaque,
                  onPointerDown: (_) => onInteractionStart(),
                  onPointerUp: (_) => onInteractionEnd(),
                  onPointerCancel: (_) => onInteractionEnd(),
                  child: ClipRRect(
                    borderRadius: const .vertical(bottom: Radius.circular(15)),
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

class _EmptyState extends StatelessWidget {
  final VoidCallback onImport;
  final VoidCallback onMess;
  const _EmptyState({super.key, required this.onImport, required this.onMess});

  @override
  Widget build(BuildContext context) => Center(
    child: SingleChildScrollView(
      child: Column(
        mainAxisAlignment: .center,
        children: [
          Container(
            padding: const .all(24),
            decoration: BoxDecoration(
              color: context.theme.colors.primary.withValues(alpha: 0.1),
              shape: .circle,
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
              fontWeight: .w700,
              color: context.theme.colors.primary,
            ),
          ),
          const SizedBox(height: 8),
          Padding(
            padding: const .symmetric(horizontal: 40),
            child: Text(
              'Import a PDF or image — like your mess menu — and it stays here, right where you left off.',
              textAlign: .center,
              style: TextStyle(
                fontSize: 13.5,
                height: 1.4,
                color: context.theme.colors.mutedForeground,
              ),
            ),
          ),
          const SizedBox(height: 20),
          FButton(
            onPress: onImport,
            child: const Row(
              mainAxisSize: .min,
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
              'or add the mess menu (.xlsx)',
              style: TextStyle(
                fontSize: 12.5,
                fontWeight: .w600,
                color: context.theme.colors.primary,
              ),
            ),
          ),
        ],
      ),
    ),
  );
}

class _CenterInfo extends StatelessWidget {
  final IconData icon;
  final String title;
  final String? subtitle;

  const _CenterInfo({required this.icon, required this.title, this.subtitle});

  @override
  Widget build(BuildContext context) => Center(
    child: Column(
      mainAxisAlignment: .center,
      children: [
        Icon(icon, size: 28, color: context.theme.colors.mutedForeground),
        const SizedBox(height: 10),
        Text(
          title,
          style: TextStyle(
            fontWeight: .w600,
            color: context.theme.colors.primary,
          ),
        ),
        if (subtitle != null) ...[
          const SizedBox(height: 6),
          Text(
            subtitle!,
            textAlign: .center,
            style: TextStyle(color: context.theme.colors.mutedForeground),
          ),
        ],
      ],
    ),
  );
}
