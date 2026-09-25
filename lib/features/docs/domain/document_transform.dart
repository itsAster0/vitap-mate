import 'package:flutter/widgets.dart';

Matrix4 resetHorizontalOffset(Matrix4 current) {
  final updated = Matrix4.copy(current);
  final translation = updated.getTranslation();
  updated.setTranslationRaw(0, translation.y, translation.z);
  return updated;
}

/// Brings a PDF back into view. For PDFs the list scroll carries the reading
/// position, so vertical translation is only drag travel past the first/last
/// page; a stale value can leave every page outside the viewport.
Matrix4 recenterPdf(Matrix4 current) {
  final updated = Matrix4.copy(current);
  updated.setTranslationRaw(0, 0, updated.getTranslation().z);
  return updated;
}
