import 'dart:typed_data';

import 'package:spreadsheet_decoder/spreadsheet_decoder.dart';

/// Sheet name → rows of cell text; empty cells (including the covered part of
/// a merged range) are ''.
Map<String, List<List<String>>> decodeSpreadsheetTables(Uint8List bytes) {
  final decoder = SpreadsheetDecoder.decodeBytes(bytes);
  return {
    for (final MapEntry(key: name, value: table) in decoder.tables.entries)
      name: [
        for (final row in table.rows)
          [for (final cell in row) cell?.toString() ?? ''],
      ],
  };
}
