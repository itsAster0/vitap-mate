import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:pocketbase/pocketbase.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/features/social/presentation/providers/pocketbase.dart';

class PbHelper extends ConsumerWidget {
  final Widget Function(PocketBase pb) child;
  const PbHelper({super.key, required this.child});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    var data = ref.watch(pbProvider);
    return data.when(
      data: (pb) {
        if (!pb.authStore.isValid) {
          return FDialog(
            title: Text("Not Logged In"),
            body: Text("Please go to the Social tab to log in."),
            actions: [
              FButton(
                onPress: () {
                  GoRouter.of(context).pop();
                },
                child: Text("ok"),
              ),
            ],
          );
        }
        return child(pb);
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
            child: CircularProgressIndicator(
              color: context.theme.colors.primary,
            ),
          ),
        );
      },
    );
  }
}
