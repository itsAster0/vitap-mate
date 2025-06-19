import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/features/more/presentation/widgets/marks_table.dart';

import 'package:vitapmate/src/api/vtop/types.dart';

class MarksCard extends HookConsumerWidget {
  final MarksRecord record;

  const MarksCard({super.key, required this.record});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final controller = useAnimationController(
      duration: Duration(microseconds: 300),
    );
    final animation = useMemoized(
      () => CurvedAnimation(parent: controller, curve: Curves.easeInOut),
      [controller],
    );

    final expanded = useState(false);
    void toggle() {
      expanded.value = !expanded.value;
      controller.toggle();
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        FTappable(
          onPress: () {
            toggle();
          },
          child: Container(
            decoration: BoxDecoration(
              color:
                  record.coursetype.toLowerCase().endsWith("lab")
                      ? Color(0xFFD4F6FF)
                      : Color(0xFFC6E7FF),
              borderRadius: BorderRadius.circular(8),
              boxShadow: [
                BoxShadow(
                  color: Colors.black12,
                  blurRadius: 10,
                  offset: Offset(0, 4),
                ),
              ],
            ),
            child: Padding(
              padding: const EdgeInsets.all(8.0),

              child: Column(
                spacing: 8,
                children: [
                  Row(
                    spacing: 2,
                    children: [
                      Icon(
                        record.coursetype.toLowerCase().endsWith("lab")
                            ? FIcons.flaskConical
                            : FIcons.libraryBig,
                      ),
                      Expanded(
                        child: SizedBox(
                          width: MediaQuery.of(context).size.width * 0.8,
                          child: Text(
                            record.coursetitle,
                            maxLines: 1,
                            overflow: TextOverflow.fade,
                            softWrap: false,
                            style: TextStyle(fontWeight: FontWeight.w500),
                          ),
                        ),
                      ),
                      SizedBox(width: 8),
                      SizedBox(
                        child: Text(
                          record.coursecode,
                          style: TextStyle(fontSize: 12),
                        ),
                      ),
                    ],
                  ),
                  Row(
                    children: [
                      Text(
                        "by ${record.faculity}",
                        style: TextStyle(fontSize: 14),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
        ),
        const SizedBox(height: 6),
        AnimatedBuilder(
          animation: animation,
          builder:
              (context, child) =>
                  FCollapsible(value: animation.value, child: child!),
          child: FCard(child: MarksTable(marks: record.marks)),
        ),
      ],
    );
  }
}
