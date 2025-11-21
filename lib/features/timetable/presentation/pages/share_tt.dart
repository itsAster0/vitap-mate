import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:pocketbase/pocketbase.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';

import 'package:vitapmate/features/settings/presentation/widgets/pb_helper.dart';
import 'package:vitapmate/features/timetable/presentation/providers/pb_links.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';
import 'package:vitapmate/features/timetable/presentation/widgets/info_disp_share.dart';

import 'package:vitapmate/features/timetable/presentation/widgets/share_tt_each.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class ShareTt extends HookConsumerWidget {
  const ShareTt({super.key});
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return FButton.icon(
      onPress: () {
        showAdaptiveDialog(
          context: context,
          builder: (context) {
            return PbHelper(
              child:
                  (pb) => FDialog(
                    title: Sharepage(pb: pb),
                    actions: [
                      // FButton(
                      //   style: FButtonStyle.secondary(),
                      //   onPress: () {
                      //     GoRouter.of(context).pop();
                      //   },
                      //   child: Text("Done"),
                      // ),
                    ],
                  ),
            );
          },
        );
      },
      child: Icon(Icons.share),
    );
  }
}

class Sharepage extends HookConsumerWidget {
  final PocketBase pb;
  const Sharepage({super.key, required this.pb});
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final valS = ref.watch(sharePageProvider);
    final useLoadingTT = useState(false);

    if (valS.hasValue) {
      var val = valS.value!;

      String Function(String) getSemname(SemesterData? val) {
        return (String k) {
          for (var i in val?.semesters ?? []) {
            if (i.id == k) {
              return i.name;
            }
          }
          return k;
        };
      }

      final namer = getSemname(val.$2);
      return Column(
        spacing: 10,
        children: [
          RichText(
            text: TextSpan(
              style: TextStyle(
                fontSize: 16,
                color: context.theme.colors.primary,
              ),
              children: [
                TextSpan(
                  text: "Current Timetable: ",
                  style: TextStyle(fontWeight: FontWeight.bold),
                ),
                TextSpan(text: namer(val.$1.semesterId)),
              ],
            ),
          ),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceAround,
            children: [
              FButton(
                style: FButtonStyle.primary(),
                onPress: () async {
                  if (useLoadingTT.value) return;
                  useLoadingTT.value = true;
                  try {
                    val = (
                      await ref.read(timetableProvider.future),
                      val.$2,
                      val.$3,
                      val.$4,
                    );

                    await ref.read(createTimetabledbProvider(val.$1).future);
                  } finally {
                    useLoadingTT.value = false;
                  }
                },
                child: Center(
                  child:
                      useLoadingTT.value
                          ? FCircularProgress.pinwheel()
                          : Text("Add / Update timetable"),
                ),
              ),
              InfoDispShare(),
            ],
          ),

          ConstrainedBox(
            constraints: BoxConstraints(
              maxHeight: MediaQuery.of(context).size.width * 0.9,
            ),
            child: SingleChildScrollView(
              child: Column(
                spacing: 4,
                children: [
                  for (var i in sortModelsForTT(val.$4))
                    ShareTtEach(
                      selectedTt:
                          val.$1.semesterId == i.getStringValue("tt_id"),
                      url: val.$3,
                      name: namer(i.getStringValue("tt_id")),
                      updateTime: i.getIntValue("last_update"),
                      timetableId: i.id,
                    ),
                ],
              ),
            ),
          ),
        ],
      );
    } else if (valS.hasError) {
      final e = valS.error!;
      disCommonToast(context, e);
      String msg = commonErrorMessage(e);
      return Center(child: Text(msg));
    } else {
      return FCircularProgress.pinwheel();
    }
  }
}

int semIDtoInt(String k) {
  final cleaned = k.toUpperCase().replaceAll("AP", "").trim();

  return int.tryParse(cleaned) ?? 0;
}

List<RecordModel> sortModelsForTT(List<RecordModel> val) {
  val.sort(
    (a, b) => semIDtoInt(
      b.getStringValue("tt_id"),
    ).compareTo(semIDtoInt(a.getStringValue("tt_id"))),
  );
  return val;
}
