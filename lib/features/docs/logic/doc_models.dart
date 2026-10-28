enum DocKind { pdf, image, spreadsheet, text, file, none }

class DocWindow {
  final String id;
  final String name;
  final String? fileName;
  final DocKind kind;
  final bool isPreset;

  /// Read a spreadsheet as the mess menu instead of a plain table. Only the
  /// view changes; the stored file stays as imported.
  final bool asMessMenu;
  final int addedAt;
  final int? lastOpenedAt;
  final double scale;
  final double offsetX;
  final double offsetY;
  final double scrollOffset;

  const DocWindow({
    required this.id,
    required this.name,
    required this.kind,
    this.fileName,
    this.isPreset = false,
    this.asMessMenu = false,
    required this.addedAt,
    this.lastOpenedAt,
    this.scale = 1.0,
    this.offsetX = 0.0,
    this.offsetY = 0.0,
    this.scrollOffset = 0.0,
  });

  bool get hasFile => fileName != null && (kind != .none);

  DocWindow copyWith({
    String? name,
    String? fileName,
    DocKind? kind,
    bool? asMessMenu,
    int? addedAt,
    int? lastOpenedAt,
    double? scale,
    double? offsetX,
    double? offsetY,
    double? scrollOffset,
  }) => DocWindow(
    id: id,
    name: name ?? this.name,
    kind: kind ?? this.kind,
    fileName: fileName ?? this.fileName,
    isPreset: isPreset,
    asMessMenu: asMessMenu ?? this.asMessMenu,
    addedAt: addedAt ?? this.addedAt,
    lastOpenedAt: lastOpenedAt ?? this.lastOpenedAt,
    scale: scale ?? this.scale,
    offsetX: offsetX ?? this.offsetX,
    offsetY: offsetY ?? this.offsetY,
    scrollOffset: scrollOffset ?? this.scrollOffset,
  );

  Map<String, dynamic> toJson() => {
    'id': id,
    'name': name,
    if (fileName != null) 'fileName': fileName,
    'kind': kind.name,
    'isPreset': isPreset,
    if (asMessMenu) 'asMessMenu': true,
    'addedAt': addedAt,
    if (lastOpenedAt != null) 'lastOpenedAt': lastOpenedAt,
    'scale': scale,
    'offsetX': offsetX,
    'offsetY': offsetY,
    'scrollOffset': scrollOffset,
  };

  factory DocWindow.fromJson(Map<String, dynamic> json) => DocWindow(
    id: json['id'] as String,
    name: json['name'] as String,
    kind: DocKind.values.asNameMap()[json['kind']] ?? .none,
    fileName: json['fileName'] as String?,
    isPreset: json['isPreset'] as bool? ?? false,
    asMessMenu: json['asMessMenu'] as bool? ?? false,
    addedAt: json['addedAt'] as int? ?? 0,
    lastOpenedAt: json['lastOpenedAt'] as int?,
    scale: (json['scale'] as num?)?.toDouble() ?? 1.0,
    offsetX: (json['offsetX'] as num?)?.toDouble() ?? 0.0,
    offsetY: (json['offsetY'] as num?)?.toDouble() ?? 0.0,
    scrollOffset: (json['scrollOffset'] as num?)?.toDouble() ?? 0.0,
  );
}

/// The extensions Docs reads, by kind; anything else is a plain file.
const docExtensions = {
  DocKind.pdf: ['pdf'],
  DocKind.image: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'bmp'],
  DocKind.spreadsheet: ['xlsx', 'xlsm', 'xls', 'ods'],
  DocKind.text: ['xml', 'html', 'htm', 'txt', 'json', 'md', 'csv'],
};

DocKind docKindFromExtension(String path) {
  final ext = path.split('.').last.toLowerCase();
  for (final MapEntry(key: kind, value: extensions) in docExtensions.entries) {
    if (extensions.contains(ext)) return kind;
  }
  return .file;
}
