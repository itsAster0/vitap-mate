import 'dart:developer';
// ignore: depend_on_referenced_packages
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/database/database.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
part 'app_db_provider.g.dart';

@Riverpod(keepAlive: true)
Future<AppDatabase> appDatabase(Ref ref) async {
  String? username = await ref.watch(
    vtopUserProvider.selectAsync((val) => val.username),
  );
  log("finished db build", level: 800);
  return AppDatabase(username: username);
}
