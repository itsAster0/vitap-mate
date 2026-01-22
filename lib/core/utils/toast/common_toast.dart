import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';

void disCommonToast(BuildContext context, Object e) {
  if (e == VtopError.invalidCredentials()) {
    showFToast(
      context: context,
      alignment: FToastAlignment.bottomCenter,
      title: const Text('Password Changed'),
      description: const Text(
        'It looks like you changed your VTOP password. Please update it in settings -> Vtop Details .',
      ),
      onDismiss: () {
        GoRouter.of(context).goNamed(Paths.vtopUserManagement);
      },
      suffixBuilder:
          (context, entry) => IntrinsicHeight(
            child: FButton(
              style:
                  context.theme.buttonStyles.primary
                      .copyWith(
                        contentStyle:
                            context.theme.buttonStyles.primary.contentStyle
                                .copyWith(
                                  padding: const EdgeInsets.symmetric(
                                    horizontal: 12,
                                    vertical: 7.5,
                                  ),
                                  textStyle: FWidgetStateMap.all(
                                    context.theme.typography.xs.copyWith(
                                      color:
                                          context
                                              .theme
                                              .colors
                                              .primaryForeground,
                                    ),
                                  ),
                                )
                                .call,
                      )
                      .call,
              onPress: entry.dismiss,
              child: const Text('Aye'),
            ),
          ),
    );
  } else if (e == VtopError.networkError()) {
    showFToast(
      context: context,
      alignment: FToastAlignment.bottomCenter,
      title: const Text('No Internet Connection'),
      description: const Text(
        "You're offline. Please check your connection and try again",
      ),
      suffixBuilder:
          (context, entry) => IntrinsicHeight(
            child: FButton(
              style:
                  context.theme.buttonStyles.primary
                      .copyWith(
                        contentStyle:
                            context.theme.buttonStyles.primary.contentStyle
                                .copyWith(
                                  padding: const EdgeInsets.symmetric(
                                    horizontal: 12,
                                    vertical: 7.5,
                                  ),
                                  textStyle: FWidgetStateMap.all(
                                    context.theme.typography.xs.copyWith(
                                      color:
                                          context
                                              .theme
                                              .colors
                                              .primaryForeground,
                                    ),
                                  ),
                                )
                                .call,
                      )
                      .call,
              onPress: entry.dismiss,
              child: const Text('Aye'),
            ),
          ),
    );
  } else if (e is FeatureDisabledException) {
    showFToast(
      context: context,
      alignment: FToastAlignment.bottomCenter,
      title: const Text('Feature Disabled'),

      description: const Text('Please try again in a while Or Update the app'),
      suffixBuilder:
          (context, entry) => IntrinsicHeight(
            child: FButton(
              style:
                  context.theme.buttonStyles.primary
                      .copyWith(
                        contentStyle:
                            context.theme.buttonStyles.primary.contentStyle
                                .copyWith(
                                  padding: const EdgeInsets.symmetric(
                                    horizontal: 12,
                                    vertical: 7.5,
                                  ),
                                  textStyle: FWidgetStateMap.all(
                                    context.theme.typography.xs.copyWith(
                                      color:
                                          context
                                              .theme
                                              .colors
                                              .primaryForeground,
                                    ),
                                  ),
                                )
                                .call,
                      )
                      .call,
              onPress: entry.dismiss,
              child: const Text('Aye'),
            ),
          ),
    );
  } else {
    showFToast(
      context: context,
      alignment: FToastAlignment.bottomCenter,
      title: const Text('Error oocured'),

      description: const Text('Please try again'),
      suffixBuilder:
          (context, entry) => IntrinsicHeight(
            child: FButton(
              style:
                  context.theme.buttonStyles.primary
                      .copyWith(
                        contentStyle:
                            context.theme.buttonStyles.primary.contentStyle
                                .copyWith(
                                  padding: const EdgeInsets.symmetric(
                                    horizontal: 12,
                                    vertical: 7.5,
                                  ),
                                  textStyle: FWidgetStateMap.all(
                                    context.theme.typography.xs.copyWith(
                                      color:
                                          context
                                              .theme
                                              .colors
                                              .primaryForeground,
                                    ),
                                  ),
                                )
                                .call,
                      )
                      .call,
              onPress: entry.dismiss,
              child: const Text('Aye'),
            ),
          ),
    );
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
      suffixBuilder:
          (context, entry) => IntrinsicHeight(
            child: FButton(
              style:
                  context.theme.buttonStyles.primary
                      .copyWith(
                        contentStyle:
                            context.theme.buttonStyles.primary.contentStyle
                                .copyWith(
                                  padding: const EdgeInsets.symmetric(
                                    horizontal: 12,
                                    vertical: 7.5,
                                  ),
                                  textStyle: FWidgetStateMap.all(
                                    context.theme.typography.xs.copyWith(
                                      color:
                                          context
                                              .theme
                                              .colors
                                              .primaryForeground,
                                    ),
                                  ),
                                )
                                .call,
                      )
                      .call,
              onPress: entry.dismiss,
              child: const Text('Aye'),
            ),
          ),
    );
  } else if (e == VtopError.networkError()) {
    showFToast(
      context: context,
      alignment: FToastAlignment.bottomCenter,
      title: const Text('No Internet Connection'),
      description: const Text(
        "You're offline. Please check your connection and try again",
      ),
      suffixBuilder:
          (context, entry) => IntrinsicHeight(
            child: FButton(
              style:
                  context.theme.buttonStyles.primary
                      .copyWith(
                        contentStyle:
                            context.theme.buttonStyles.primary.contentStyle
                                .copyWith(
                                  padding: const EdgeInsets.symmetric(
                                    horizontal: 12,
                                    vertical: 7.5,
                                  ),
                                  textStyle: FWidgetStateMap.all(
                                    context.theme.typography.xs.copyWith(
                                      color:
                                          context
                                              .theme
                                              .colors
                                              .primaryForeground,
                                    ),
                                  ),
                                )
                                .call,
                      )
                      .call,
              onPress: entry.dismiss,
              child: const Text('Aye'),
            ),
          ),
    );
  } else if (e is FeatureDisabledException) {
    showFToast(
      context: context,
      alignment: FToastAlignment.bottomCenter,
      title: const Text('Feature Disabled'),
      //description: const Text('Visit this page for more information.'),
      description: const Text('Please try again in a while'),
      suffixBuilder:
          (context, entry) => IntrinsicHeight(
            child: FButton(
              style:
                  context.theme.buttonStyles.primary
                      .copyWith(
                        contentStyle:
                            context.theme.buttonStyles.primary.contentStyle
                                .copyWith(
                                  padding: const EdgeInsets.symmetric(
                                    horizontal: 12,
                                    vertical: 7.5,
                                  ),
                                  textStyle: FWidgetStateMap.all(
                                    context.theme.typography.xs.copyWith(
                                      color:
                                          context
                                              .theme
                                              .colors
                                              .primaryForeground,
                                    ),
                                  ),
                                )
                                .call,
                      )
                      .call,
              onPress: entry.dismiss,
              child: const Text('Aye'),
            ),
          ),
    );
  }
}

void dispToast(BuildContext context, String title, String des) {
  showFToast(
    context: context,
    alignment: FToastAlignment.bottomCenter,
    title: Text(title),
    //description: const Text('Visit this page for more information.'),
    description: Text(des),
    suffixBuilder:
        (context, entry) => IntrinsicHeight(
          child: FButton(
            style:
                context.theme.buttonStyles.primary
                    .copyWith(
                      contentStyle:
                          context.theme.buttonStyles.primary.contentStyle
                              .copyWith(
                                padding: const EdgeInsets.symmetric(
                                  horizontal: 12,
                                  vertical: 7.5,
                                ),
                                textStyle: FWidgetStateMap.all(
                                  context.theme.typography.xs.copyWith(
                                    color:
                                        context.theme.colors.primaryForeground,
                                  ),
                                ),
                              )
                              .call,
                    )
                    .call,
            onPress: entry.dismiss,
            child: const Text('Aye'),
          ),
        ),
  );
}
