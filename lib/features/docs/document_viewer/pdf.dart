// PDF documents: the file lookup, the paged viewport and its page tiles.
part of '../document_viewer_page.dart';

class _PdfContent extends HookConsumerWidget {
  final DocWindow doc;
  final double dpr;
  final TransformationController transform;
  final void Function({double scrollOffset}) persist;

  const _PdfContent({
    required this.doc,
    required this.dpr,
    required this.transform,
    required this.persist,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final pathState = _useStoredPath(ref, doc);
    if (pathState.connectionState != ConnectionState.done) {
      return const Spinner();
    }
    final path = pathState.data;
    if (path == null) {
      return const _Notice('File missing. Remove and re-import this document.');
    }
    return _PdfLoaded(
      path: path,
      doc: doc,
      dpr: dpr,
      transform: transform,
      persist: persist,
    );
  }
}

class _PdfLoaded extends HookWidget {
  final String path;
  final DocWindow doc;
  final double dpr;
  final TransformationController transform;
  final void Function({double scrollOffset}) persist;

  const _PdfLoaded({
    required this.path,
    required this.doc,
    required this.dpr,
    required this.transform,
    required this.persist,
  });

  @override
  Widget build(BuildContext context) {
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
      return docFuture.hasError
          ? const _Notice('Unable to open this PDF.')
          : const Spinner();
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
          child: _PdfPageTile(document: pdfDoc, pageNumber: i + 1, dpr: dpr),
        ),
      ),
    );
  }
}

class _PdfPageTile extends StatefulWidget {
  final pdfx.PdfDocument document;
  final int pageNumber;
  final double dpr;

  const _PdfPageTile({
    required this.document,
    required this.pageNumber,
    required this.dpr,
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
    final placeholder = BoxDecoration(
      color: context.theme.colors.primaryForeground,
      borderRadius: .circular(8),
      border: Border.all(color: context.theme.colors.border),
    );
    return Padding(
      padding: const .only(bottom: 14),
      child: switch ((_bytes, _aspect)) {
        (final bytes?, final aspect?) => ClipRRect(
          borderRadius: .circular(8),
          child: AspectRatio(
            aspectRatio: aspect,
            child: Image.memory(
              bytes,
              fit: .fill,
              filterQuality: .medium,
              gaplessPlayback: true,
            ),
          ),
        ),
        _ when _failed => Container(
          height: 120,
          alignment: .center,
          decoration: placeholder,
          child: Text(
            'Failed to render page ${widget.pageNumber}',
            style: TextStyle(color: context.theme.colors.mutedForeground),
          ),
        ),
        _ => AspectRatio(
          aspectRatio: 1 / 1.414,
          child: Container(
            alignment: .center,
            decoration: placeholder,
            child: CircularProgressIndicator(
              color: context.theme.colors.primary,
              strokeWidth: 2.5,
            ),
          ),
        ),
      },
    );
  }
}
