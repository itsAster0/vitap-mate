enum DocKind { pdf, image, spreadsheet, text, none }

class DocWindow {
  final String id;
  final String name;
  final String? fileName;
  final DocKind kind;
  final bool isPreset;
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
    required this.addedAt,
    this.lastOpenedAt,
    this.scale = 1.0,
    this.offsetX = 0.0,
    this.offsetY = 0.0,
    this.scrollOffset = 0.0,
  });

  bool get hasFile => fileName != null && (kind != DocKind.none);

  DocWindow copyWith({
    String? name,
    String? fileName,
    DocKind? kind,
    int? lastOpenedAt,
    double? scale,
    double? offsetX,
    double? offsetY,
    double? scrollOffset,
  }) {
    return DocWindow(
      id: id,
      name: name ?? this.name,
      kind: kind ?? this.kind,
      fileName: fileName ?? this.fileName,
      isPreset: isPreset,
      addedAt: addedAt,
      lastOpenedAt: lastOpenedAt ?? this.lastOpenedAt,
      scale: scale ?? this.scale,
      offsetX: offsetX ?? this.offsetX,
      offsetY: offsetY ?? this.offsetY,
      scrollOffset: scrollOffset ?? this.scrollOffset,
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'name': name,
    if (fileName != null) 'fileName': fileName,
    'kind': kind.name,
    'isPreset': isPreset,
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
    kind: DocKind.values.firstWhere(
      (k) => k.name == json['kind'],
      orElse: () => DocKind.none,
    ),
    fileName: json['fileName'] as String?,
    isPreset: json['isPreset'] as bool? ?? false,
    addedAt: json['addedAt'] as int? ?? 0,
    lastOpenedAt: json['lastOpenedAt'] as int?,
    scale: (json['scale'] as num?)?.toDouble() ?? 1.0,
    offsetX: (json['offsetX'] as num?)?.toDouble() ?? 0.0,
    offsetY: (json['offsetY'] as num?)?.toDouble() ?? 0.0,
    scrollOffset: (json['scrollOffset'] as num?)?.toDouble() ?? 0.0,
  );
}

DocKind docKindFromExtension(String path) {
  final ext = path.split('.').last.toLowerCase();
  switch (ext) {
    case 'pdf':
      return DocKind.pdf;
    case 'png':
    case 'jpg':
    case 'jpeg':
    case 'webp':
    case 'gif':
    case 'bmp':
      return DocKind.image;
    case 'xlsx':
    case 'xlsm':
    case 'xls':
    case 'ods':
      return DocKind.spreadsheet;
    case 'xml':
    case 'html':
    case 'htm':
    case 'txt':
    case 'json':
    case 'md':
    case 'csv':
      return DocKind.text;
    default:
      return DocKind.none;
  }
}
