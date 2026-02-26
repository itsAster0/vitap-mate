import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:flutter_inappwebview/flutter_inappwebview.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';

import 'package:vitapmate/core/providers/theme_provider.dart';

import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/features/more/presentation/widgets/more_color.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';
import 'package:webview_flutter/webview_flutter.dart';

var cookieManagerAndroid = WebViewCookieManager();

({String name, String value})? parseCookiePair(String cookieHeader) {
  final firstCookiePart = cookieHeader.split(";").first.trim();
  final separatorIndex = firstCookiePart.indexOf("=");
  if (separatorIndex <= 0 || separatorIndex >= firstCookiePart.length - 1) {
    return null;
  }

  final name = firstCookiePart.substring(0, separatorIndex).trim();
  final value = firstCookiePart.substring(separatorIndex + 1).trim();
  if (name.isEmpty || value.isEmpty) {
    return null;
  }

  return (name: name, value: value);
}

class VtopWebview extends HookConsumerWidget {
  const VtopWebview({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final envState = useState(true);
    final loading = useState(false);
    final cookieName = useState<String?>(null);
    final cookieValue = useState<String?>(null);
    final webController = useState<InAppWebViewController?>(null);
    final showHeading = useState("VTOP");
    final isRemoveSpacing = useState(true);
    final url = WebUri("https://vtop.vitap.ac.in");
    final initUrl = WebUri("https://vtop.vitap.ac.in/vtop/content?");
    final isDesktopMode = useState(false);
    final isdark = useState(ref.read(themeProvider).index == 2);
    final String darkModeScript = '''
    
  (function() {
    const existingStyle = document.getElementById('dark-mode-style');
    if (existingStyle) existingStyle.remove();
    
    const style = document.createElement('style');
    style.id = 'dark-mode-style';
    style.textContent = `
      html {
        filter: invert(1) hue-rotate(180deg) !important;
        background-color: #000 !important;
      }
      img, video, [style*="background-image"] {
        filter: invert(1) hue-rotate(180deg) !important;
      }
    `;
    document.head.appendChild(style);
  })();
''';

    final String removeDarkModeScript = '''
  (function() {
    const style = document.getElementById('dark-mode-style');
    if (style) style.remove();
  })();
''';

    useEffect(() {
      final controller = webController.value;
      if (controller != null) {
        if (isdark.value) {
          controller.evaluateJavascript(source: darkModeScript);
        } else {
          controller.evaluateJavascript(source: removeDarkModeScript);
        }
      }
      return null;
    }, [isdark.value]);
    void removeSpacing({int padding = 2}) {
      final controller = webController.value;
      if (controller != null) {
        controller.evaluateJavascript(
          source: '''
      (function() {
      // Wait a bit for content to load
      setTimeout(function() {
        const style = document.createElement('style');
        style.id = 'custom-spacing-style';
        style.textContent = `
          * {
            margin: 0 !important;
            padding: 0 !important;
            box-sizing: border-box !important;
          }
          
          body {
            padding: ${padding}px !important;
            margin: 0 !important;
          }
          
          table {
            border-spacing: 0 !important;
            border-collapse: collapse !important;
            width: 100% !important;
            margin: ${padding}px !important;
          }
          
          td, th {
            padding: ${padding + 4}px !important;
          }
          
          .card {
            margin: ${padding}px !important;
            padding: ${padding}px !important;
            border-radius: 0 !important;
          }
          
          .container, .container-fluid {
            padding: ${padding}px !important;
            margin: ${padding}px !important;
            width: 100% !important;
            max-width: 100% !important;
          }
          
          /* Target the CGPA Details specifically */
          div[style*="padding"], div[style*="margin"] {
            padding: ${padding}px !important;
            margin: ${padding}px !important;
          }
  button[data-bs-target="#expandedSideBar"] {
    padding: 10px 14px !important;
    border: none !important;
    border-radius: 6px !important;
    transition: all 0.3s ease !important;
    cursor: pointer !important;
  }
    
  `;
        
        // Remove existing style if present
        const oldStyle = document.getElementById('custom-spacing-style');
        if (oldStyle) oldStyle.remove();
        
        document.head.appendChild(style);
        
        // Force layout recalculation
        document.body.style.display = 'none';
        document.body.offsetHeight;
        document.body.style.display = 'block';
      }, 500);
    })();
    ''',
        );
      }
    }

    void restDesktopview() {
      final controller = webController.value;
      if (controller != null) {
        controller.evaluateJavascript(
          source: '''
(function () {
  var viewport = document.querySelector('meta[name=viewport]');
  if (viewport) {
    viewport.setAttribute(
      'content',
      'width=device-width, initial-scale=1'
    );
  }
})();
      ''',
        );
      }
    }

    void setDesktopview() {
      final controller = webController.value;
      if (controller != null) {
        controller.evaluateJavascript(
          source: '''
        (function() {
    var viewportWidth = 1024;
    var viewport = document.querySelector("meta[name=viewport]");
    
    if (viewport) {
      viewport.setAttribute('content', 'width=' + viewportWidth + ', user-scalable=yes');
    } else {
      var meta = document.createElement('meta');
      meta.name = "viewport";
      meta.content = 'width=' + viewportWidth + ', user-scalable=yes';
      document.head.appendChild(meta);
    }
  })();
      ''',
        );
      }
    }

    useEffect(() {
      if (isDesktopMode.value) {
        setDesktopview();
      } else {
        restDesktopview();
      }
      return null;
    }, [isDesktopMode.value]);
    void resetSpacing() {
      final controller = webController.value;
      if (controller != null) {
        controller.evaluateJavascript(
          source: '''
        (function() {
          // Remove the custom style element
          const customStyle = document.getElementById('custom-spacing-style');
          if (customStyle) {
            customStyle.remove();
          }
          
          // Force layout recalculation
          document.body.style.display = 'none';
          document.body.offsetHeight;
          document.body.style.display = 'block';
          
          // Optionally reload the page to fully restore original styles
          // window.location.reload();
        })();
      ''',
        );
      }
    }

    Future<void> loadenv() async {
      final cookieManager = CookieManager.instance();
      final expiresDate =
          DateTime.now()
              .add(const Duration(minutes: 30))
              .millisecondsSinceEpoch;

      final client = await ref.read(vClientProvider.future);
      final raw = await fetchCookies(client: client);
      final cookieString = String.fromCharCodes(raw);

      final parsedCookie = parseCookiePair(cookieString);
      if (parsedCookie == null) return;

      final name = parsedCookie.name;
      final value = parsedCookie.value;
      cookieName.value = name;
      cookieValue.value = value;
      await cookieManager.deleteAllCookies();
      await cookieManagerAndroid.clearCookies();

      if (Platform.isAndroid) {
        await cookieManagerAndroid.setCookie(
          WebViewCookie(
            name: name,
            value: value,
            domain: '.vitap.ac.in',
            path: '/',
          ),
        );
      } else {
        await cookieManager.setCookie(
          url: url,
          name: name,
          value: value,
          expiresDate: expiresDate,
          isSecure: true,
          domain: ".vitap.ac.in",
          path: '/',
        );
      }

      envState.value = false;
    }

    useEffect(() {
      Future(() async {
        await loadenv();
      });

      return null;
    }, []);

    if (envState.value) {
      return FScaffold(
        childPad: false,
        header: FHeader.nested(
          title: Text("VTOP"),
          prefixes: [
            FHeaderAction.back(onPress: () => GoRouter.of(context).pop()),
          ],
        ),
        child: Container(
          color: context.theme.colors.primaryForeground,
          child: Center(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                SizedBox(
                  width: 50,
                  height: 50,
                  child: CircularProgressIndicator(
                    color: MoreColors.infoBorder,
                    strokeWidth: 3,
                  ),
                ),
                const SizedBox(height: 16),
                Text(
                  "Setting up VTOP...",
                  style: TextStyle(
                    fontSize: 16,
                    color: MoreColors.secondaryText,
                    fontWeight: FontWeight.w500,
                  ),
                ),
              ],
            ),
          ),
        ),
      );
    }
    void gorto(String url) async {
      final controller = webController.value;
      if (controller != null) {
        await controller.evaluateJavascript(
          source: '''
  (function() {
    const link = document.querySelector('a[data-url="$url"]');
    if (link) {
      link.click();
      return true;
    }
    return false;
  })();
''',
        );
      }
    }

    useEffect(() {
      if (isRemoveSpacing.value) {
        removeSpacing(padding: 1);
      } else {
        resetSpacing();
      }
      return null;
    }, [isRemoveSpacing.value]);
    return FScaffold(
      childPad: false,
      header: FHeader.nested(
        title: Text(
          showHeading.value,
          style: TextStyle(fontSize: context.theme.typography.sm.fontSize),
        ),
        prefixes: [
          FHeaderAction.back(onPress: () => GoRouter.of(context).pop()),
        ],
        suffixes: [
          FHeaderAction(
            icon: Icon(
              isdark.value ? Icons.light_mode : Icons.dark_mode,
              size: 20,
            ),
            onPress: () {
              isdark.value = !isdark.value;
            },
          ),
          SizedBox(width: 10),
          FPopoverMenu(
            autofocus: true,
            menuAnchor: Alignment.topRight,
            childAnchor: Alignment.bottomRight,
            menu: [
              FItemGroup(
                children: [
                  FItem(
                    prefix: const Icon(FIcons.calendar),
                    title: const Text('Timetable'),
                    onPress: () => gorto("academics/common/StudentTimeTable"),
                  ),
                  FItem(
                    prefix: const Icon(FIcons.paperclip),
                    title: const Text('Attendance'),
                    onPress: () => gorto("academics/common/StudentAttendance"),
                  ),
                  FItem(
                    prefix: const Icon(FIcons.book),
                    title: const Text('CoursePage'),
                    onPress: () => gorto("academics/common/StudentCoursePage"),
                  ),
                  FItem(
                    prefix: const Icon(FIcons.university),
                    title: const Text('Academic Calendar'),
                    onPress: () => gorto("academics/common/CalendarPreview"),
                  ),
                  FItem(
                    prefix: const Icon(FIcons.workflow),
                    title: const Text('Digital Assignment'),
                    onPress: () => gorto("examinations/StudentDA"),
                  ),
                ],
              ),

              FItemGroup(
                children: [
                  FItem(
                    prefix: const Icon(FIcons.graduationCap),
                    title: const Text('Grades'),
                    onPress:
                        () => gorto(
                          "examinations/examGradeView/StudentGradeView",
                        ),
                  ),
                  FItem(
                    prefix: const Icon(FIcons.history),
                    title: const Text('Grades History'),
                    onPress:
                        () => gorto(
                          "examinations/examGradeView/StudentGradeHistory",
                        ),
                  ),
                ],
              ),
              FItemGroup(
                children: [
                  FItem(
                    prefix: const Icon(FIcons.amphora),
                    title: const Text('Weekend Outing'),
                    onPress: () => gorto("hostel/StudentWeekendOuting"),
                  ),
                  FItem(
                    prefix: const Icon(FIcons.anchor),
                    title: const Text('General Outing'),
                    onPress: () => gorto("hostel/StudentGeneralOuting"),
                  ),
                ],
              ),
              FItemGroup(
                children: [
                  FItem(
                    prefix: FCheckbox(value: isRemoveSpacing.value),
                    title: Text('Compact View'),
                    onPress: () {
                      isRemoveSpacing.value = !isRemoveSpacing.value;
                    },
                  ),
                  FItem(
                    prefix: FCheckbox(value: isDesktopMode.value),
                    title: Text('Desktop Mode'),
                    onPress: () {
                      isDesktopMode.value = !isDesktopMode.value;
                    },
                  ),
                  FItem(
                    prefix: const Icon(FIcons.logIn),
                    title: const Text('Force Login'),
                    onPress: () async {
                      loading.value = true;
                      try {
                        await ref
                            .read(vClientProvider.notifier)
                            .tryLogin(force: true);

                        await loadenv();
                        final controller = webController.value;
                        if (controller != null) {
                          await controller.loadUrl(
                            urlRequest: URLRequest(url: initUrl),
                          );
                        }
                      } finally {
                        loading.value = false;
                      }
                    },
                  ),
                ],
              ),
            ],
            builder:
                (_, controller, _) => FHeaderAction(
                  icon: const Icon(FIcons.ellipsis),
                  onPress: controller.toggle,
                ),
          ),
        ],
      ),
      child: SafeArea(
        bottom: true,
        top: false,
        left: false,
        right: false,
        child: Container(
          width: MediaQuery.of(context).size.width,
          height: MediaQuery.of(context).size.height * 0.8,
          decoration: BoxDecoration(
            color: MoreColors.tableBackground,
            boxShadow: [
              BoxShadow(
                color: MoreColors.cardShadow,
                blurRadius: 8,
                offset: const Offset(0, 2),
              ),
            ],
          ),
          child: Stack(
            children: [
              InAppWebView(
                initialSettings: InAppWebViewSettings(
                  isInspectable: kDebugMode,
                ),
                initialUrlRequest: URLRequest(url: initUrl),
                onWebViewCreated: (controller) {
                  webController.value = controller;
                },

                shouldOverrideUrlLoading: (controller, navigationAction) async {
                  final uri = navigationAction.request.url;
                  // removeSpacing();
                  if (uri.toString().toLowerCase().contains("download")) {
                    String cookie = "${cookieName.value}=${cookieValue.value};";
                    downloadFile(uri.toString(), cookie);
                    return NavigationActionPolicy.CANCEL;
                  } else if (uri.toString().toLowerCase().startsWith(
                    "https://vtop.vitap.ac.in",
                  )) {
                    return NavigationActionPolicy.ALLOW;
                  }

                  return NavigationActionPolicy.CANCEL;
                },
                onReceivedServerTrustAuthRequest: (_, _) async {
                  return ServerTrustAuthResponse(
                    action: ServerTrustAuthResponseAction.PROCEED,
                  );
                },
                onUpdateVisitedHistory: (controller, url, androidIsReload) {
                  final u = url.toString().toLowerCase();
                  if (!u.startsWith("https://vtop.vitap.ac.in")) {
                    controller.stopLoading();
                    controller.goBack();
                    dispToast(
                      context,
                      "Open in Chrome",
                      "Please continue in Chrome.",
                    );
                  }
                },
                onLoadStart: (controller, url) async {
                  loading.value = true;
                  //removeSpacing(padding: 1);
                  final k = ref.read(themeProvider);

                  if (isdark.value) {
                    controller.evaluateJavascript(source: darkModeScript);
                  }
                },
                onLoadStop: (controller, url) async {
                  loading.value = false;
                  removeSpacing(padding: 1);
                  final k = ref.read(themeProvider);

                  if (isdark.value) {
                    controller.evaluateJavascript(source: darkModeScript);
                  }
                },
              ),
              if (loading.value)
                Positioned(left: 0, right: 0, top: 0, child: FProgress()),
            ],
          ),
        ),
      ),
    );
  }
}
