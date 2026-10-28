import 'package:flutter/widgets.dart';
import 'package:vitapmate/ui/app_palette.dart';
import 'package:vitapmate/ui/text_nums.dart';

/// A number that counts up to [value] when first shown or when it changes.
class CountUp extends StatelessWidget {
  const CountUp({
    super.key,
    required this.value,
    required this.style,
    this.decimals = 0,
    this.suffix = '',
  });

  final double value;
  final TextStyle style;
  final int decimals;
  final String suffix;

  @override
  Widget build(BuildContext context) => TweenAnimationBuilder<double>(
    tween: Tween(begin: 0, end: value),
    duration: Motion.reduced(context) ? .zero : Motion.slow * 2,
    curve: Curves.easeOutCubic,
    builder: (context, v, _) =>
        Text('${v.toStringAsFixed(decimals)}$suffix', style: style.nums()),
  );
}
