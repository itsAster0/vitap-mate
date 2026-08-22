import 'package:flutter_test/flutter_test.dart';
import 'package:flutter/widgets.dart';
import 'package:vitapmate/features/docs/domain/document_transform.dart';

void main() {
  test('horizontal reset preserves vertical position and zoom', () {
    final original = Matrix4.identity()
      ..scaleByDouble(2, 2, 1, 1)
      ..setTranslationRaw(-140, -320, 0);

    final reset = resetHorizontalOffset(original);

    expect(reset.getTranslation().x, 0);
    expect(reset.getTranslation().y, -320);
    expect(reset.getMaxScaleOnAxis(), 2);
    expect(original.getTranslation().x, -140);
  });
}
