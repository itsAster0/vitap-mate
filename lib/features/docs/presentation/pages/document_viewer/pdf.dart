// PDF documents: the file lookup, the paged viewport and its page tiles.
part of '../document_viewer_page.dart';

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
      return const Spinner();
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
      return const Spinner();
    }
    return NotificationListener<ScrollNotification>(
      onNotification: onScrollNotification,
      child: PdfDocumentViewport(
        transform: transform,
        scroll: scroll,
        pageCount: pdfDoc.pagesCount,
        onInteractionEnd: () => persist(scrollOffset: scroll.offset),
        pageBuilder: (context, i) => RepaintBoundary(
          key: ValueKey('page-$i'),
          child: _PdfPageTile(
            document: pdfDoc,
            pageNumber: i + 1,
            dpr: dpr,
            darkMode: darkMode,
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
