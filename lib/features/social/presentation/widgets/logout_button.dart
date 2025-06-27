import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/features/social/presentation/providers/pocketbase.dart';

class LogoutButton extends HookConsumerWidget {
  const LogoutButton({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    var data = ref.watch(pbProvider);
    return data.when(
      data: (pb) {
        if (!pb.authStore.isValid) {
          return SizedBox.shrink();
        }
        return FTappable(
          onPress: () {
            pb.authStore.clear();
            ref.invalidate(pbProvider);
          },
          child: Icon(FIcons.logOut),
        );
      },
      error: (e, se) {
        disCommonToast(context, e);
        String msg = commonErrorMessage(e);
        return Center(child: Text(msg));
      },
      loading: () {
        return SizedBox(
          width: 10,
          height: 10,
          child: CircularProgressIndicator(color: Colors.black),
        );
      },
    );
  }
}
