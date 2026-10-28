import 'package:flutter/material.dart' show CircularProgressIndicator;
import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

/// A centered loading spinner in the primary colour.
class Spinner extends StatelessWidget {
  const Spinner({super.key});

  @override
  Widget build(BuildContext context) => Center(
    child: CircularProgressIndicator(
      color: context.theme.colors.primary,
      strokeWidth: 3,
    ),
  );
}
