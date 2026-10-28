import 'package:flutter/material.dart';

/// Shares one pan/scale recognizer across the document and its lazy page list.
class PdfDocumentViewport extends StatefulWidget {
  final TransformationController transform;
  final ScrollController scroll;
  final int pageCount;
  final IndexedWidgetBuilder pageBuilder;
  final VoidCallback onInteractionEnd;

  const PdfDocumentViewport({
    super.key,
    required this.transform,
    required this.scroll,
    required this.pageCount,
    required this.pageBuilder,
    required this.onInteractionEnd,
  });

  @override
  State<PdfDocumentViewport> createState() => _PdfDocumentViewportState();
}

class _PdfDocumentViewportState extends State<PdfDocumentViewport> {
  bool _updatingScroll = false;

  @override
  void initState() {
    super.initState();
    widget.transform.addListener(_scrollPages);
  }

  @override
  void didUpdateWidget(PdfDocumentViewport oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.transform != widget.transform) {
      oldWidget.transform.removeListener(_scrollPages);
      widget.transform.addListener(_scrollPages);
    }
  }

  @override
  void dispose() {
    widget.transform.removeListener(_scrollPages);
    super.dispose();
  }

  void _scrollPages() {
    if (_updatingScroll || !widget.scroll.hasClients) return;
    final position = widget.scroll.position;
    if (!position.hasContentDimensions) return;
    final matrix = Matrix4.copy(widget.transform.value);
    final scale = matrix.getMaxScaleOnAxis();
    final translation = matrix.getTranslation();
    final offset = (position.pixels - translation.y / scale).clamp(
      position.minScrollExtent,
      position.maxScrollExtent,
    );
    final delta = offset - position.pixels;
    if (delta == 0) return;

    // Move vertical travel into the list so offscreen pages stay lazy. Keep
    // any travel past the first/last page in the viewer's transformation.
    _updatingScroll = true;
    try {
      matrix.setTranslationRaw(
        translation.x,
        translation.y + delta * scale,
        translation.z,
      );
      widget.transform.value = matrix;
      widget.scroll.jumpTo(offset);
    } finally {
      _updatingScroll = false;
    }
  }

  @override
  Widget build(BuildContext context) {
    return InteractiveViewer(
      transformationController: widget.transform,
      boundaryMargin: const EdgeInsets.all(double.infinity),
      minScale: 0.5,
      maxScale: 6,
      onInteractionEnd: (_) => widget.onInteractionEnd(),
      child: ListView.builder(
        controller: widget.scroll,
        // A nested drag recognizer can win before the second finger lands,
        // preventing InteractiveViewer from ever receiving the pinch.
        physics: const NeverScrollableScrollPhysics(),
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 16),
        itemCount: widget.pageCount,
        itemBuilder: widget.pageBuilder,
      ),
    );
  }
}
