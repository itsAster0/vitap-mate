import 'package:flutter/widgets.dart';
import 'package:vitapmate/ui/app_palette.dart';
import 'package:vitapmate/ui/skeleton.dart';
import 'package:vitapmate/ui/surface.dart';

/// A card-shaped skeleton, repeated [count] times, for list screens.
class SkeletonList extends StatelessWidget {
  const SkeletonList({super.key, this.count = 5, this.height = 96});

  final int count;
  final double height;

  @override
  Widget build(BuildContext context) => Column(
    children: [
      for (var i = 0; i < count; i++)
        Padding(
          padding: const .only(bottom: Space.md),
          child: Surface(
            child: ConstrainedBox(
              constraints: BoxConstraints(minHeight: height - Space.lg * 2),
              child: const Row(
                children: [
                  Skeleton(width: 44, height: 44, radius: Radii.md),
                  SizedBox(width: Space.md),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: .start,
                      mainAxisAlignment: .center,
                      children: [
                        Skeleton(width: 180, height: 14),
                        SizedBox(height: Space.sm),
                        Skeleton(width: 110, height: 11),
                      ],
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
    ],
  );
}
