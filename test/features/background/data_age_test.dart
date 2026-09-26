import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/features/background/stale_refresh.dart';
import 'package:vitapmate/features/background/sync.dart';

void main() {
  BigInt savedAgo(Duration ago) =>
      BigInt.from(DateTime.now().subtract(ago).millisecondsSinceEpoch ~/ 1000);

  test('the age limit decides whether saved data is fetched again', () {
    final elevenHours = savedAgo(const Duration(hours: 11));
    expect(isVtopDataFresh(elevenHours, openAppRefreshMaxAge), true);
    expect(isVtopDataFresh(elevenHours, backgroundSyncMaxAge), false);
    expect(
      isVtopDataFresh(
        savedAgo(const Duration(hours: 13)),
        openAppRefreshMaxAge,
      ),
      false,
    );
  });

  test('data with no save time is never fresh', () {
    expect(isVtopDataFresh(BigInt.zero, openAppRefreshMaxAge), false);
  });
}
