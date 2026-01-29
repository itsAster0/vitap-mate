import 'package:riverpod_annotation/riverpod_annotation.dart';
part 'filter.g.dart';

enum AttendanceFilter { all, classes, labs }

@Riverpod(keepAlive: true)
class AttendanceFilterNotifier extends _$AttendanceFilterNotifier {
  @override
  AttendanceFilter build() {
    return AttendanceFilter.all;
  }

  void setFilter(AttendanceFilter filter) {
    state = filter;
  }
}
