// Single-image and spreadsheet documents, and search highlighting.
part of '../document_viewer_page.dart';

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
      return const Spinner();
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
        return compute(decodeSpreadsheetTables, bytes);
      }, [doc.fileName, attempt.value]),
    );

    final data = sheets.data;

    if (sheets.connectionState != ConnectionState.done) {
      return const Spinner();
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
