import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:pdfx/pdfx.dart' as pdfx;
import 'package:shared_preferences/shared_preferences.dart';
import 'package:spreadsheet_decoder/spreadsheet_decoder.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/features/docs/data/doc_models.dart';
import 'package:vitapmate/features/docs/domain/document_transform.dart';
import 'package:vitapmate/features/docs/presentation/providers/docs_provider.dart';

class DocumentViewerPage extends HookConsumerWidget {
  final DocWindow doc;
  final bool embedded;

  const DocumentViewerPage({
    super.key,
    required this.doc,
    this.embedded = false,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final darkMode = ref.watch(themeProvider) == ThemeMode.dark;
    final activeTitle = ref.read(activeDocumentTitleProvider.notifier);
    final registry = ref.read(docsRegistryProvider.notifier);

    useEffect(() {
      if (embedded) return null;
      Future.microtask(() {
        activeTitle.show(doc.name);
        unawaited(registry.touchLastOpened(doc.id));
      });
      return () {
        Future.microtask(() => activeTitle.clear(doc.name));
      };
    }, [doc.id, doc.name, embedded]);

    final transform = useMemoized(() {
      final m = Matrix4(
        doc.scale,
        0,
        0,
        0,
        0,
        doc.scale,
        0,
        0,
        0,
        0,
        1,
        0,
        doc.offsetX,
        doc.offsetY,
        0,
        1,
      );
      return TransformationController(m);
    }, [doc.id]);
    useEffect(() => transform.dispose, [transform]);

    if (!doc.hasFile) {
      return _EmptyPrompt(doc: doc, darkMode: darkMode);
    }

    return _DocSurface(
      doc: doc,
      darkMode: darkMode,
      transform: transform,
      embedded: embedded,
    );
  }
}

class _EmptyPrompt extends HookConsumerWidget {
  final DocWindow doc;
  final bool darkMode;
  const _EmptyPrompt({required this.doc, required this.darkMode});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Center(
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
              FLucideIcons.utensils,
              size: 40,
              color: context.theme.colors.primary,
            ),
          ),
          const SizedBox(height: 16),
          Text(
            doc.name,
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
              'No file added yet. Import one to view it here anytime.',
              textAlign: TextAlign.center,
              style: TextStyle(color: context.theme.colors.mutedForeground),
            ),
          ),
          const SizedBox(height: 20),
          FButton(
            onPress: () async {
              final pickedPath = await pickDocPath();
              if (pickedPath == null) return;
              // ignore: use_build_context_synchronously
              await ref
                  .read(docsRegistryProvider.notifier)
                  .fillPreset(doc, pickedPath);
            },
            child: const Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(FLucideIcons.filePlus2, size: 16),
                SizedBox(width: 8),
                Text('Add file'),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _DocSurface extends HookConsumerWidget {
  final DocWindow doc;
  final bool darkMode;
  final TransformationController transform;
  final bool embedded;

  const _DocSurface({
    required this.doc,
    required this.darkMode,
    required this.transform,
    required this.embedded,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final dpr = MediaQuery.devicePixelRatioOf(context);
    final searchOpen = useState(false);
    final searchQuery = useState('');
    final searchController = useTextEditingController();
    final searchFocus = useFocusNode();
    final showGestureHint = useState(false);
    final preSearchTransform = useRef<Matrix4?>(null);
    final saveTimer = useRef<Timer?>(null);
    final lastScrollOffset = useRef(doc.scrollOffset);
    final pendingSave =
        useRef<
          ({double scale, double offsetX, double offsetY, double scrollOffset})?
        >(null);
    final registry = ref.read(docsRegistryProvider.notifier);
    final searchable =
        doc.kind == DocKind.spreadsheet || doc.kind == DocKind.text;

    useEffect(() {
      if (embedded) return null;
      var cancelled = false;
      Timer? dismissTimer;
      SharedPreferences.getInstance().then((prefs) {
        if (cancelled ||
            (prefs.getBool('docs_viewer_gesture_hint_seen') ?? false)) {
          return;
        }
        showGestureHint.value = true;
        unawaited(prefs.setBool('docs_viewer_gesture_hint_seen', true));
        dismissTimer = Timer(const Duration(seconds: 4), () {
          if (!cancelled) showGestureHint.value = false;
        });
      });
      return () {
        cancelled = true;
        dismissTimer?.cancel();
      };
    }, [embedded]);

    void flushSave() {
      saveTimer.value?.cancel();
      saveTimer.value = null;
      final state = pendingSave.value;
      if (state == null) return;
      pendingSave.value = null;
      unawaited(
        registry.saveScrollState(
          doc.id,
          scale: state.scale,
          offsetX: state.offsetX,
          offsetY: state.offsetY,
          scrollOffset: state.scrollOffset,
        ),
      );
    }

    void persist({double scrollOffset = -1}) {
      showGestureHint.value = false;
      final m = transform.value;
      if (scrollOffset >= 0) lastScrollOffset.value = scrollOffset;
      pendingSave.value = (
        scale: m.getMaxScaleOnAxis(),
        offsetX: m.getTranslation().x,
        offsetY: m.getTranslation().y,
        scrollOffset: lastScrollOffset.value,
      );
      saveTimer.value?.cancel();
      saveTimer.value = Timer(const Duration(milliseconds: 500), flushSave);
    }

    useEffect(() {
      return flushSave;
    }, []);

    Widget content;
    switch (doc.kind) {
      case DocKind.pdf:
        content = _PdfContent(
          doc: doc,
          dpr: dpr,
          darkMode: darkMode,
          transform: transform,
          persist: persist,
        );
        break;
      case DocKind.image:
        content = _SingleContent(
          doc: doc,
          darkMode: darkMode,
          dpr: dpr,
          transform: transform,
          persist: persist,
          searchQuery: searchQuery.value,
        );
        break;
      case DocKind.spreadsheet:
        content = _SpreadsheetBody(
          doc: doc,
          darkMode: darkMode,
          transform: transform,
          persist: persist,
          searchQuery: searchQuery.value,
        );
        break;
      case DocKind.text:
      case DocKind.none:
        content = _SingleContent(
          doc: doc,
          darkMode: darkMode,
          dpr: dpr,
          transform: transform,
          persist: persist,
          searchQuery: searchQuery.value,
        );
        break;
    }

    return Stack(
      children: [
        Positioned.fill(child: content),
        if (!embedded && searchable)
          Positioned(
            top: 12,
            right: 12,
            child: _ViewerSearch(
              open: searchOpen.value,
              controller: searchController,
              focusNode: searchFocus,
              onOpen: () {
                preSearchTransform.value = Matrix4.copy(transform.value);
                searchOpen.value = true;
                WidgetsBinding.instance.addPostFrameCallback(
                  (_) => searchFocus.requestFocus(),
                );
              },
              onChanged: (value) {
                searchQuery.value = value;
                if (value.trim().isNotEmpty) {
                  transform.value = Matrix4.identity();
                }
              },
              onClose: () {
                searchController.clear();
                searchQuery.value = '';
                searchOpen.value = false;
                searchFocus.unfocus();
                final previous = preSearchTransform.value;
                if (previous != null) transform.value = previous;
                preSearchTransform.value = null;
              },
            ),
          ),
        if (showGestureHint.value)
          Positioned(
            left: 24,
            right: 24,
            bottom: 22,
            child: IgnorePointer(
              child: Center(
                child: Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 14,
                    vertical: 10,
                  ),
                  decoration: BoxDecoration(
                    color: context.theme.colors.primary,
                    borderRadius: BorderRadius.circular(999),
                    boxShadow: const [
                      BoxShadow(
                        color: Color(0x33000000),
                        blurRadius: 10,
                        offset: Offset(0, 3),
                      ),
                    ],
                  ),
                  child: Text(
                    'Drag to move  •  Pinch to zoom',
                    style: TextStyle(
                      color: context.theme.colors.primaryForeground,
                      fontSize: 12,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                ),
              ),
            ),
          ),
        Positioned(
          right: 12,
          bottom: 12,
          child: FTappable(
            onPress: () {
              transform.value = resetHorizontalOffset(transform.value);
              persist();
            },
            child: Container(
              padding: const EdgeInsets.all(10),
              decoration: BoxDecoration(
                color: darkMode
                    ? context.theme.colors.primaryForeground
                    : Colors.white,
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: context.theme.colors.border),
                boxShadow: const [
                  BoxShadow(
                    color: Color(0x22000000),
                    blurRadius: 8,
                    offset: Offset(0, 2),
                  ),
                ],
              ),
              child: Icon(
                FLucideIcons.locateFixed,
                size: 18,
                color: context.theme.colors.primary,
              ),
            ),
          ),
        ),
      ],
    );
  }
}

class _ViewerSearch extends StatelessWidget {
  final bool open;
  final TextEditingController controller;
  final FocusNode focusNode;
  final VoidCallback onOpen;
  final ValueChanged<String> onChanged;
  final VoidCallback onClose;

  const _ViewerSearch({
    required this.open,
    required this.controller,
    required this.focusNode,
    required this.onOpen,
    required this.onChanged,
    required this.onClose,
  });

  @override
  Widget build(BuildContext context) {
    final surface = context.theme.colors.background;
    if (!open) {
      return FTappable(
        onPress: onOpen,
        child: _ViewerControlSurface(
          child: Icon(
            Icons.search_rounded,
            size: 20,
            color: context.theme.colors.primary,
          ),
        ),
      );
    }

    return Container(
      width: (MediaQuery.sizeOf(context).width - 40).clamp(180.0, 340.0),
      height: 44,
      decoration: BoxDecoration(
        color: surface,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: context.theme.colors.border),
        boxShadow: const [
          BoxShadow(
            color: Color(0x22000000),
            blurRadius: 8,
            offset: Offset(0, 2),
          ),
        ],
      ),
      child: Row(
        children: [
          const SizedBox(width: 11),
          Icon(
            Icons.search_rounded,
            size: 19,
            color: context.theme.colors.mutedForeground,
          ),
          const SizedBox(width: 8),
          Expanded(
            child: TextField(
              focusNode: focusNode,
              controller: controller,
              onChanged: onChanged,
              style: TextStyle(
                fontSize: 13,
                color: context.theme.colors.primary,
              ),
              decoration: const InputDecoration(
                hintText: 'Search document',
                border: InputBorder.none,
                isDense: true,
                contentPadding: EdgeInsets.zero,
              ),
            ),
          ),
          IconButton(
            onPressed: onClose,
            tooltip: 'Close search',
            icon: const Icon(Icons.close_rounded, size: 18),
          ),
        ],
      ),
    );
  }
}

class _ViewerControlSurface extends StatelessWidget {
  final Widget child;

  const _ViewerControlSurface({required this.child});

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 44,
      height: 44,
      alignment: Alignment.center,
      decoration: BoxDecoration(
        color: context.theme.colors.background,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: context.theme.colors.border),
        boxShadow: const [
          BoxShadow(
            color: Color(0x22000000),
            blurRadius: 8,
            offset: Offset(0, 2),
          ),
        ],
      ),
      child: child,
    );
  }
}

class _PdfContent extends HookConsumerWidget {
  final DocWindow doc;
  final double dpr;
  final bool darkMode;
  final TransformationController transform;
  final void Function({double scrollOffset}) persist;

  const _PdfContent({
    required this.doc,
    required this.dpr,
    required this.darkMode,
    required this.transform,
    required this.persist,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final pathState = useFuture(
      useMemoized(() async {
        final repo = await ref.read(docsRepositoryProvider.future);
        return repo.storedFilePathOf(doc);
      }, [doc.fileName]),
    );

    if (pathState.connectionState != ConnectionState.done) {
      return Center(
        child: CircularProgressIndicator(
          color: context.theme.colors.primary,
          strokeWidth: 3,
        ),
      );
    }
    final path = pathState.data;
    if (path == null) {
      return Center(
        child: Text(
          'File missing. Remove and re-import this document.',
          style: TextStyle(color: context.theme.colors.mutedForeground),
        ),
      );
    }

    return _PdfLoaded(
      path: path,
      doc: doc,
      dpr: dpr,
      darkMode: darkMode,
      transform: transform,
      persist: persist,
    );
  }
}

class _PdfLoaded extends HookConsumerWidget {
  final String path;
  final DocWindow doc;
  final double dpr;
  final bool darkMode;
  final TransformationController transform;
  final void Function({double scrollOffset}) persist;

  const _PdfLoaded({
    required this.path,
    required this.doc,
    required this.dpr,
    required this.darkMode,
    required this.transform,
    required this.persist,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final docFuture = useFuture(
      useMemoized(() => pdfx.PdfDocument.openFile(path), [path]),
    );
    final scroll = useScrollController(
      initialScrollOffset: doc.scrollOffset.clamp(0.0, 100000.0),
    );

    bool onScrollNotification(ScrollNotification n) {
      if (n is ScrollEndNotification) {
        persist(scrollOffset: n.metrics.pixels);
      }
      return false;
    }

    final pdfDoc = docFuture.data;
    if (docFuture.connectionState != ConnectionState.done || pdfDoc == null) {
      if (docFuture.hasError) {
        return Center(
          child: Text(
            'Unable to open this PDF.',
            style: TextStyle(color: context.theme.colors.mutedForeground),
          ),
        );
      }
      return Center(
        child: CircularProgressIndicator(
          color: context.theme.colors.primary,
          strokeWidth: 3,
        ),
      );
    }
    return InteractiveViewer(
      transformationController: transform,
      boundaryMargin: const EdgeInsets.all(double.infinity),
      minScale: 0.5,
      maxScale: 6,
      panEnabled: true,
      clipBehavior: Clip.none,
      onInteractionEnd: (_) => persist(),
      child: NotificationListener<ScrollNotification>(
        onNotification: onScrollNotification,
        child: ListView.builder(
          controller: scroll,
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 16),
          itemCount: pdfDoc.pagesCount,
          itemBuilder: (context, i) => RepaintBoundary(
            key: ValueKey('page-$i'),
            child: _PdfPageTile(
              document: pdfDoc,
              pageNumber: i + 1,
              dpr: dpr,
              darkMode: darkMode,
            ),
          ),
        ),
      ),
    );
  }
}

class _PdfPageTile extends StatefulWidget {
  final pdfx.PdfDocument document;
  final int pageNumber;
  final double dpr;
  final bool darkMode;

  const _PdfPageTile({
    required this.document,
    required this.pageNumber,
    required this.dpr,
    required this.darkMode,
  });

  @override
  State<_PdfPageTile> createState() => _PdfPageTileState();
}

class _PdfPageTileState extends State<_PdfPageTile>
    with AutomaticKeepAliveClientMixin {
  Uint8List? _bytes;
  double? _aspect;
  bool _failed = false;

  @override
  bool get wantKeepAlive => true;

  @override
  void initState() {
    super.initState();
    _render();
  }

  Future<void> _render() async {
    try {
      final page = await widget.document.getPage(widget.pageNumber);
      if (!mounted) return;
      final targetW = (page.width * widget.dpr).clamp(720.0, 1800.0);
      final targetH = targetW * (page.height / page.width);
      final rendered = await page.render(
        width: targetW,
        height: targetH,
        backgroundColor: '#FFFFFF',
        format: pdfx.PdfPageImageFormat.jpeg,
        quality: 90,
      );
      await page.close();
      if (!mounted) return;
      setState(() {
        _bytes = rendered?.bytes;
        _aspect = page.width / page.height;
      });
    } catch (_) {
      if (mounted) setState(() => _failed = true);
    }
  }

  @override
  Widget build(BuildContext context) {
    super.build(context);
    if (_bytes != null && _aspect != null) {
      return Padding(
        padding: const EdgeInsets.only(bottom: 14),
        child: ClipRRect(
          borderRadius: BorderRadius.circular(8),
          child: AspectRatio(
            aspectRatio: _aspect!,
            child: Image.memory(
              _bytes!,
              fit: BoxFit.fill,
              filterQuality: FilterQuality.medium,
              gaplessPlayback: true,
            ),
          ),
        ),
      );
    }
    if (_failed) {
      return Padding(
        padding: const EdgeInsets.only(bottom: 14),
        child: Container(
          height: 120,
          alignment: Alignment.center,
          decoration: BoxDecoration(
            color: context.theme.colors.primaryForeground,
            borderRadius: BorderRadius.circular(8),
            border: Border.all(color: context.theme.colors.border),
          ),
          child: Text(
            'Failed to render page ${widget.pageNumber}',
            style: TextStyle(color: context.theme.colors.mutedForeground),
          ),
        ),
      );
    }
    return Padding(
      padding: const EdgeInsets.only(bottom: 14),
      child: AspectRatio(
        aspectRatio: 1 / 1.414,
        child: Container(
          alignment: Alignment.center,
          decoration: BoxDecoration(
            color: context.theme.colors.primaryForeground,
            borderRadius: BorderRadius.circular(8),
            border: Border.all(color: context.theme.colors.border),
          ),
          child: CircularProgressIndicator(
            color: context.theme.colors.primary,
            strokeWidth: 2.5,
          ),
        ),
      ),
    );
  }
}

class _SingleContent extends HookConsumerWidget {
  final DocWindow doc;
  final bool darkMode;
  final double dpr;
  final TransformationController transform;
  final void Function({double scrollOffset}) persist;
  final String searchQuery;

  const _SingleContent({
    required this.doc,
    required this.darkMode,
    required this.dpr,
    required this.transform,
    required this.persist,
    required this.searchQuery,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final body = useFuture(
      useMemoized(() async {
        final repo = await ref.read(docsRepositoryProvider.future);
        return repo.storedFilePathOf(doc);
      }, [doc.fileName]),
    );

    final path = body.data;
    final textBody = useFuture(
      useMemoized(() async {
        if (path == null || doc.kind == DocKind.image) return null;
        return File(path).readAsString();
      }, [path, doc.kind]),
    );

    if (body.connectionState != ConnectionState.done ||
        (doc.kind != DocKind.image &&
            textBody.connectionState != ConnectionState.done)) {
      return Center(
        child: CircularProgressIndicator(
          color: context.theme.colors.primary,
          strokeWidth: 3,
        ),
      );
    }
    if (path == null) {
      return Center(
        child: Text(
          'File missing.',
          style: TextStyle(color: context.theme.colors.mutedForeground),
        ),
      );
    }
    final fullText = textBody.data ?? '';
    final needle = searchQuery.trim().toLowerCase();
    final visibleText = needle.isEmpty
        ? fullText
        : fullText
              .split('\n')
              .where((line) => line.toLowerCase().contains(needle))
              .take(100)
              .join('\n');

    return InteractiveViewer(
      transformationController: transform,
      boundaryMargin: const EdgeInsets.all(double.infinity),
      minScale: 0.5,
      maxScale: 8,
      panEnabled: true,
      clipBehavior: Clip.none,
      onInteractionEnd: (_) => persist(),
      child: Center(
        child: doc.kind == DocKind.image
            ? ConstrainedBox(
                constraints: BoxConstraints(maxWidth: 1200 * dpr * 0.5),
                child: Image.file(
                  File(path),
                  filterQuality: FilterQuality.medium,
                ),
              )
            : Container(
                width: 900,
                margin: const EdgeInsets.all(24),
                padding: const EdgeInsets.all(20),
                decoration: BoxDecoration(
                  color: darkMode
                      ? context.theme.colors.primaryForeground
                      : Colors.white,
                  borderRadius: BorderRadius.circular(12),
                  boxShadow: const [
                    BoxShadow(color: Color(0x1A000000), blurRadius: 12),
                  ],
                ),
                child: SelectableText.rich(
                  TextSpan(
                    children: _searchSpans(
                      visibleText.isEmpty && needle.isNotEmpty
                          ? 'No matches found.'
                          : visibleText,
                      searchQuery,
                      context.theme.colors.primary.withValues(alpha: 0.24),
                    ),
                    style: TextStyle(
                      fontFamily: 'monospace',
                      fontSize: 13,
                      height: 1.45,
                      color: darkMode
                          ? context.theme.colors.primary
                          : const Color(0xFF212121),
                    ),
                  ),
                  style: TextStyle(
                    fontFamily: 'monospace',
                    fontSize: 13,
                    height: 1.45,
                    color: darkMode
                        ? context.theme.colors.primary
                        : const Color(0xFF212121),
                  ),
                ),
              ),
      ),
    );
  }
}

Map<String, List<List<String>>> decodeSpreadsheetIsolate(Uint8List bytes) {
  final decoder = SpreadsheetDecoder.decodeBytes(bytes);
  final out = <String, List<List<String>>>{};
  decoder.tables.forEach((name, table) {
    out[name] = [
      for (final row in table.rows)
        [for (final cell in row) cell?.toString() ?? ''],
    ];
  });
  return out;
}

List<InlineSpan> _searchSpans(String text, String query, Color highlightColor) {
  final needle = query.trim().toLowerCase();
  if (needle.isEmpty) return [TextSpan(text: text)];

  final lowerText = text.toLowerCase();
  final spans = <InlineSpan>[];
  var start = 0;
  while (true) {
    final match = lowerText.indexOf(needle, start);
    if (match == -1) {
      spans.add(TextSpan(text: text.substring(start)));
      return spans;
    }
    if (match > start) {
      spans.add(TextSpan(text: text.substring(start, match)));
    }
    final end = match + needle.length;
    spans.add(
      TextSpan(
        text: text.substring(match, end),
        style: TextStyle(
          backgroundColor: highlightColor,
          fontWeight: FontWeight.w800,
        ),
      ),
    );
    start = end;
  }
}

class _SpreadsheetBody extends HookConsumerWidget {
  final DocWindow doc;
  final bool darkMode;
  final TransformationController transform;
  final void Function({double scrollOffset}) persist;
  final String searchQuery;

  const _SpreadsheetBody({
    required this.doc,
    required this.darkMode,
    required this.transform,
    required this.persist,
    required this.searchQuery,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final selectedSheet = useState(0);
    final attempt = useState(0);

    final sheets = useFuture(
      useMemoized(() async {
        final repo = await ref.read(docsRepositoryProvider.future);
        final path = await repo.storedFilePathOf(doc);
        if (path == null) return null;
        final bytes = await File(path).readAsBytes();
        return compute(decodeSpreadsheetIsolate, bytes);
      }, [doc.fileName, attempt.value]),
    );

    final data = sheets.data;

    if (sheets.connectionState != ConnectionState.done) {
      return Center(
        child: CircularProgressIndicator(
          color: context.theme.colors.primary,
          strokeWidth: 3,
        ),
      );
    }
    if (sheets.hasError) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                'Couldn\u2019t read this file.\n'
                'If it\u2019s a legacy .xls, save it as .xlsx and re-add it.',
                textAlign: TextAlign.center,
                style: TextStyle(
                  color: context.theme.colors.mutedForeground,
                  height: 1.5,
                ),
              ),
              const SizedBox(height: 8),
              Text('${sheets.error}', textAlign: TextAlign.center),
              const SizedBox(height: 12),
              FTappable(
                onPress: () => attempt.value++,
                child: Text(
                  'Retry',
                  style: TextStyle(
                    fontWeight: FontWeight.w700,
                    color: context.theme.colors.primary,
                  ),
                ),
              ),
            ],
          ),
        ),
      );
    }
    if (data == null || data.isEmpty) {
      return Center(
        child: Text(
          'This spreadsheet has no readable sheets.',
          style: TextStyle(color: context.theme.colors.mutedForeground),
        ),
      );
    }

    final names = data.keys.toList();
    final safeIndex = selectedSheet.value.clamp(0, names.length - 1);
    final rows = data[names[safeIndex]] ?? const [];
    final inner = Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (names.length > 1)
          Padding(
            padding: const EdgeInsets.only(left: 24, bottom: 10),
            child: SizedBox(
              width: 640,
              child: SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                child: Row(
                  children: [
                    for (var i = 0; i < names.length; i++)
                      Padding(
                        padding: const EdgeInsets.only(right: 8),
                        child: FTappable(
                          onPress: () => selectedSheet.value = i,
                          child: AnimatedContainer(
                            duration: const Duration(milliseconds: 180),
                            curve: Curves.easeOutCubic,
                            padding: const EdgeInsets.symmetric(
                              horizontal: 12,
                              vertical: 7,
                            ),
                            decoration: BoxDecoration(
                              color: i == safeIndex
                                  ? context.theme.colors.primary
                                  : context.theme.colors.background,
                              borderRadius: BorderRadius.circular(999),
                              border: Border.all(
                                color: i == safeIndex
                                    ? context.theme.colors.primary
                                    : context.theme.colors.border,
                              ),
                            ),
                            child: Text(
                              names[i],
                              style: TextStyle(
                                fontSize: 12.5,
                                fontWeight: FontWeight.w600,
                                color: i == safeIndex
                                    ? context.theme.colors.primaryForeground
                                    : context.theme.colors.mutedForeground,
                              ),
                            ),
                          ),
                        ),
                      ),
                  ],
                ),
              ),
            ),
          ),
        _SheetTable(rows: rows, darkMode: darkMode, searchQuery: searchQuery),
      ],
    );

    return LayoutBuilder(
      builder: (context, constraints) => InteractiveViewer(
        transformationController: transform,
        boundaryMargin: const EdgeInsets.all(double.infinity),
        minScale: 0.5,
        maxScale: 6,
        panEnabled: true,
        constrained: false,
        clipBehavior: Clip.none,
        onInteractionEnd: (_) => persist(),
        child: ConstrainedBox(
          constraints: BoxConstraints(
            minWidth: constraints.maxWidth,
            minHeight: constraints.maxHeight,
          ),
          child: Center(child: inner),
        ),
      ),
    );
  }
}

class _SheetTable extends StatelessWidget {
  static const _maxRows = 400;
  static const _maxCols = 60;

  final List<List<String>> rows;
  final bool darkMode;
  final String searchQuery;

  const _SheetTable({
    required this.rows,
    required this.darkMode,
    required this.searchQuery,
  });

  String _columnLabel(int index) {
    var value = index + 1;
    var label = '';
    while (value > 0) {
      value--;
      label = String.fromCharCode(65 + value % 26) + label;
      value ~/= 26;
    }
    return label;
  }

  Widget _axisCell(BuildContext context, String label) {
    return Container(
      alignment: Alignment.center,
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 7),
      color: context.theme.colors.primary.withValues(alpha: 0.09),
      child: Text(
        label,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
        style: TextStyle(
          fontSize: 11,
          fontWeight: FontWeight.w800,
          color: context.theme.colors.mutedForeground,
        ),
      ),
    );
  }

  Widget _dataCell(BuildContext context, String value, {required bool header}) {
    final query = searchQuery.trim();
    final match =
        query.isNotEmpty && value.toLowerCase().contains(query.toLowerCase());
    return Container(
      color: match
          ? context.theme.colors.primary.withValues(alpha: 0.24)
          : null,
      padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 7),
      child: Text(
        value,
        maxLines: 3,
        overflow: TextOverflow.ellipsis,
        style: TextStyle(
          fontSize: 12.5,
          height: 1.25,
          fontWeight: match || header ? FontWeight.w700 : FontWeight.w500,
          color: header
              ? context.theme.colors.primary
              : darkMode
              ? context.theme.colors.mutedForeground
              : const Color(0xFF212121),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final query = searchQuery.trim().toLowerCase();
    final sourceRows = rows.take(_maxRows).toList();
    final visibleRows = <({int index, List<String> cells})>[
      for (var i = 0; i < sourceRows.length; i++)
        if (query.isEmpty ||
            sourceRows[i].any((cell) => cell.toLowerCase().contains(query)))
          (index: i, cells: sourceRows[i]),
    ];
    var colCount = 0;
    for (final r in visibleRows) {
      colCount = colCount < r.cells.length ? r.cells.length : colCount;
    }
    colCount = colCount.clamp(1, _maxCols);

    final widths = List<double>.filled(colCount, 72);
    for (var c = 0; c < colCount; c++) {
      var maxLen = 0;
      for (final r in visibleRows) {
        if (c < r.cells.length && r.cells[c].length > maxLen) {
          maxLen = r.cells[c].length;
        }
      }
      widths[c] = (maxLen * 7.5 + 20).clamp(56.0, 170.0);
    }

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 24),
      child: Material(
        color: darkMode ? context.theme.colors.primaryForeground : Colors.white,
        borderRadius: BorderRadius.circular(12),
        elevation: darkMode ? 0 : 2,
        shadowColor: const Color(0x22000000),
        child: ClipRRect(
          borderRadius: BorderRadius.circular(12),
          child: Table(
            border: TableBorder.all(
              color: context.theme.colors.border,
              width: 0.6,
            ),
            defaultVerticalAlignment: TableCellVerticalAlignment.middle,
            columnWidths: {
              0: const FixedColumnWidth(44),
              for (var c = 0; c < colCount; c++)
                c + 1: FixedColumnWidth(widths[c]),
            },
            children: [
              TableRow(
                children: [
                  _axisCell(context, ''),
                  for (var c = 0; c < colCount; c++)
                    _axisCell(context, _columnLabel(c)),
                ],
              ),
              if (visibleRows.isEmpty)
                TableRow(
                  children: [
                    _axisCell(context, ''),
                    _dataCell(context, 'No matches found.', header: false),
                  ],
                ),
              for (var r = 0; r < visibleRows.length; r++)
                TableRow(
                  decoration: BoxDecoration(
                    color: visibleRows[r].index == 0
                        ? context.theme.colors.primary.withValues(alpha: 0.12)
                        : visibleRows[r].index.isOdd && !darkMode
                        ? const Color(0xFFFAFAFA)
                        : null,
                  ),
                  children: [
                    _axisCell(context, '${visibleRows[r].index + 1}'),
                    for (var c = 0; c < colCount; c++)
                      _dataCell(
                        context,
                        c < visibleRows[r].cells.length
                            ? visibleRows[r].cells[c]
                            : '',
                        header: visibleRows[r].index == 0,
                      ),
                  ],
                ),
            ],
          ),
        ),
      ),
    );
  }
}
