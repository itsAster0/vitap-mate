import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:pocketbase/pocketbase.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/features/settings/presentation/providers/semester_id_provider.dart';
import 'package:vitapmate/features/social/presentation/providers/pocketbase.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
part 'pb_links.g.dart';

@riverpod
Future<List> getTimetableSharesLinks(Ref ref) async {
  var pb = await ref.read(pbProvider.future);
  List out = [];
  try {
    var data = await pb.send("/timetable/links", method: "GET");
    out = data["links"];
  } catch (e) {
    ();
  }
  return out;
}

@riverpod
Future<void> createTimetabledb(Ref ref, TimetableData ttData) async {
  var pb = await ref.read(pbProvider.future);
  final payload = {...ttData.toJson(), "updateTime": ttData.updateTime.toInt()};
  var temp = await pb.send("/timetable/create", method: "POST", body: payload);
  var _ = await ref.refresh(getTimetabledbProvider.future);
}

@riverpod
Future<void> createTimetablelinkdb(Ref ref, String id) async {
  var pb = await ref.read(pbProvider.future);
  var payload = {"timetableID": id};
  var temp = await pb.send(
    "/timetable/link/create",
    method: "POST",
    body: payload,
  );
  var _ = await ref.refresh(getTimetableSharesLinksProvider.future);
  var _ = await ref.read(sharePageProvider.future);
}

@riverpod
Future<List<RecordModel>> getTimetabledb(Ref ref) async {
  var pb = await ref.read(pbProvider.future);
  var temp = await pb
      .collection("timetable")
      .getFullList(filter: 'user="${pb.authStore.record!.id}"');
  return temp;
}

@riverpod
Future<void> delTimetablelinkdb(Ref ref, String id) async {
  var pb = await ref.read(pbProvider.future);

  await pb.collection("timetable_links").delete(id);
  var _ = await ref.refresh(getTimetableSharesLinksProvider.future);
  var _ = await ref.read(sharePageProvider.future);
}

@riverpod
Future<(TimetableData, SemesterData, List, List<RecordModel>)> sharePage(
  Ref ref,
) async {
  var sett = await ref.watch(timetableProvider.future);
  var sem = await ref.watch(semesterIdProvider.future);
  var url = await ref.watch(getTimetableSharesLinksProvider.future);
  var uploaded = await ref.watch(getTimetabledbProvider.future);
  return (sett, sem, url, uploaded);
}
