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
        showFSheet(
          mainAxisMaxRatio: 1 / 4,
          context: context,
          builder: (context) {
            return Container(
              height: double.infinity,
              width: double.infinity,
              decoration: BoxDecoration(
                color: context.theme.colors.background,
                borderRadius: const BorderRadius.vertical(
                  top: Radius.circular(10),
                ),
              ),
              child: PbHelper(child: (pb) => Sharepage(pb: pb)),
            );
          },
          side: FLayout.btt,
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

      Future<String?> handelSync() async {
        if (useLoadingTT.value) return null;
        useLoadingTT.value = true;
        String? out;
        try {
          val = (
            await ref.read(timetableProvider.future),
            val.$2,
            val.$3,
            val.$4,
          );

          out = await ref.read(createTimetabledbProvider(val.$1).future);
        } finally {
          useLoadingTT.value = false;
        }
        return out;
      }

      var found = false;
      RecordModel? foundRecord;
      for (var i in val.$4) {
        if (val.$1.semesterId == i.getStringValue("tt_id")) {
          foundRecord = i;
          found = true;
        }
      }

      final namer = getSemname(val.$2);

      return Padding(
        padding: const EdgeInsets.all(8.0),
        child: Column(
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
                    text: "Current Semester: ",
                    style: TextStyle(fontWeight: FontWeight.bold),
                  ),
                  TextSpan(text: namer(val.$1.semesterId)),
                ],
              ),
            ),
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceAround,
              children: [
                FButton.icon(
                  onPress: handelSync,
                  child: Center(
                    child:
                        useLoadingTT.value
                            ? FCircularProgress.pinwheel()
                            : Icon(Icons.sync),
                  ),
                ),
                InfoDispShare(),
              ],
            ),

            if (found)
              Padding(
                padding: const EdgeInsets.all(20.0),
                child: ShareTtEach(
                  url: val.$3,
                  name: namer(foundRecord!.getStringValue("tt_id")),
                  updateTime: foundRecord.getIntValue("last_update"),
                  timetableId: foundRecord.id,
                ),
              ),
            if (!found)
              FButton(
                onPress: () async {
                  final timetableId = await handelSync();
                  if (timetableId != null) {
                    useLoadingTT.value = true;
                    try {
                      await ref.read(
                        createTimetablelinkdbProvider(timetableId).future,
                      );
                      var _ = await ref.refresh(sharePageProvider.future);
                    } finally {
                      useLoadingTT.value = false;
                    }
                  }
                },
                child:
                    useLoadingTT.value
                        ? FCircularProgress.pinwheel()
                        : Text("Create a link"),
              ),
          ],
        ),
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

// int semIDtoInt(String k) {
//   final cleaned = k.toUpperCase().replaceAll("AP", "").trim();

//   return int.tryParse(cleaned) ?? 0;
// }

// List<RecordModel> sortModelsForTT(List<RecordModel> val) {
//   val.sort(
//     (a, b) => semIDtoInt(
//       b.getStringValue("tt_id"),
//     ).compareTo(semIDtoInt(a.getStringValue("tt_id"))),
//   );
//   return val;
// }
