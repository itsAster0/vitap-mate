import 'dart:developer';

import 'package:flutter/material.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/features/more/presentation/providers/marks_provider.dart';
import 'package:vitapmate/features/more/presentation/widgets/marks_card.dart';

class MarksPage extends HookConsumerWidget {
  const MarksPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    Future<void> update() async {
      try {
        await ref.read(vClientProvider.notifier).tryLogin();
        await ref.read(marksProvider.notifier).updatemarks();
      } catch (e) {
        log("$e");
        if (context.mounted) disCommonToast(context, e);
      }
    }

    var marksData = ref.watch(marksProvider);
    return RefreshIndicator(
      onRefresh: () async {
        await update();
      },
      backgroundColor: Colors.white,

      color: Colors.black,
      child: SingleChildScrollView(
        physics: const AlwaysScrollableScrollPhysics(),
        child: ConstrainedBox(
          constraints: BoxConstraints(
            minHeight: MediaQuery.of(context).size.height * 0.8,
          ),
          child: marksData.when(
            data: (data) {
              return Padding(
                padding: const EdgeInsets.all(6),
                child: Column(
                  spacing: 4,
                  children: [
                    for (final i in data.records) MarksCard(record: i),
                    Center(
                      child: Padding(
                        padding: const EdgeInsets.only(top: 12, bottom: 20),
                        child: Text(
                          "Data updated on ${formatUnixTimestamp(data.updateTime.toInt())}",
                          style: TextStyle(
                            fontSize: 14,
                            color: Colors.grey[600],
                          ),
                        ),
                      ),
                    ),
                  ],
                ),
              );
            },
            error: (e, se) {
              disCommonToast(context, e);
              String msg = commonErrorMessage(e);
              return Center(child: Text(msg));
            },
            loading: () {
              return Center(
                child: SizedBox(
                  width: 50,
                  height: 50,
                  child: CircularProgressIndicator(color: Colors.black),
                ),
              );
            },
          ),
        ),
      ),
    );
  }
}
