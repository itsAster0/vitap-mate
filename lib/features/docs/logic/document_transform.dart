import 'package:flutter/widgets.dart';

Matrix4 resetHorizontalOffset(Matrix4 current) {
  final translation = current.getTranslation();
  return Matrix4.copy(current)
    ..setTranslationRaw(0, translation.y, translation.z);
}

/// Brings a PDF back into view. For PDFs the list scroll carries the reading
/// position, so vertical translation is only drag travel past the first/last
/// page; a stale value can leave every page outside the viewport.
Matrix4 recenterPdf(Matrix4 current) =>
    Matrix4.copy(current)..setTranslationRaw(0, 0, current.getTranslation().z);
