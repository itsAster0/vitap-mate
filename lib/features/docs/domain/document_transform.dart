import 'package:flutter/widgets.dart';

Matrix4 resetHorizontalOffset(Matrix4 current) {
  final updated = Matrix4.copy(current);
  final translation = updated.getTranslation();
  updated.setTranslationRaw(0, translation.y, translation.z);
  return updated;
}
