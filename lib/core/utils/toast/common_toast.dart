import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/core/utils/app_error.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';

Widget _dismissToastButton(BuildContext context, dynamic entry) {
  return IntrinsicHeight(
    child: FButton(
      size: FButtonSizeVariant.xs,
      onPress: entry.dismiss,
      child: const Text('Aye'),
    ),
  );
}

void disCommonToast(BuildContext context, Object e) {
  if (!context.mounted) return;
  if (e == VtopError.invalidCredentials()) {
    showFToast(
      context: context,
      alignment: FToastAlignment.bottomCenter,
      title: const Text('Password Changed'),
      description: const Text(
        'It looks like you changed your VTOP password. Please update it in Settings.',
      ),
      suffixBuilder: _dismissToastButton,
    );
  } else if (e == VtopError.networkError()) {
    showFToast(
      context: context,
      alignment: FToastAlignment.bottomCenter,
      title: const Text('No Internet Connection'),
      description: const Text(
        "You're offline. Please check your connection and try again",
      ),
      suffixBuilder: _dismissToastButton,
    );
  } else if (e is FeatureDisabledException) {
    showFToast(
      context: context,
      alignment: FToastAlignment.bottomCenter,
      title: const Text('Feature Disabled'),

      description: const Text('Please try again in a while Or Update the app'),
      suffixBuilder: _dismissToastButton,
    );
  } else {
    dispToast(context, 'Error', appErrorMessage(e));
  }
}

void disOnbardingCommonToast(BuildContext context, Object e) {
  if (e == VtopError.invalidCredentials()) {
    showFToast(
      context: context,
      alignment: FToastAlignment.bottomCenter,
      title: const Text('Login Failed'),
      description: const Text(
        'The username or password you entered is incorrect.',
      ),
      suffixBuilder: _dismissToastButton,
    );
  } else if (e == VtopError.networkError()) {
    showFToast(
      context: context,
      alignment: FToastAlignment.bottomCenter,
      title: const Text('No Internet Connection'),
      description: const Text(
        "You're offline. Please check your connection and try again",
      ),
      suffixBuilder: _dismissToastButton,
    );
  } else if (e is FeatureDisabledException) {
    showFToast(
      context: context,
      alignment: FToastAlignment.bottomCenter,
      title: const Text('Feature Disabled'),
      //description: const Text('Visit this page for more information.'),
      description: const Text('Please try again in a while'),
      suffixBuilder: _dismissToastButton,
    );
  } else {
    dispToast(context, 'Login Failed', appErrorMessage(e));
  }
}

void dispToast(BuildContext context, String title, String des) {
  showFToast(
    context: context,
    alignment: FToastAlignment.bottomCenter,
    title: Text(title),
    //description: const Text('Visit this page for more information.'),
    description: Text(des),
    suffixBuilder: _dismissToastButton,
  );
}
