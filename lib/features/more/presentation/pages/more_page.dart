import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:http/http.dart' as http;
import 'package:url_launcher/url_launcher.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';

import 'package:vitapmate/features/more/presentation/widgets/more_color.dart';
import 'package:vitapmate/features/more/presentation/widgets/wifi_card.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';

class MorePage extends HookConsumerWidget {
  const MorePage({super.key});
  static final Map<String, String?> _previewCache = {};

  static const _studentProjects =
      <({
        int id,
        String name,
        String type,
        String desc,
        String madeBy,
        String url,
      })>[
    (
      id: 1001,
      name: "GPA Calculator",
      type: "Website",
      desc: "Calculate GPA/CGPA quickly for VIT-AP students.",
      madeBy: "Venkatsubash07",
      url: "https://vitapcgpacalculator.vercel.app/",
    ),
    (
      id: 1002,
      name: "GPA Calculator",
      type: "App",
      desc: "VIT-AP GPA calculator app  for VIT-AP students. ",
      madeBy: "Maddikeri Narendranath Reddy",
      url: "https://play.google.com/store/apps/details?id=com.gcalc.gcalcpro",
    ),
  ];

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(8, 8, 8, 0),
      child: SingleChildScrollView(
        child: Column(
          spacing: 10,
          children: [
            FTileGroup(
              label: const Text("Academic"),
              divider: FItemDivider.indented,
              children: [
                FTile(
                  prefix: const Icon(Icons.grading_outlined),
                  title: const Text('Marks'),
                  subtitle: const Text('View your Marks'),
                  suffix: const Icon(FIcons.chevronRight),
                  onPress: () {
                    GoRouter.of(context).pushNamed(Paths.marks);
                  },
                ),
                FTile(
                  prefix: const Icon(Icons.event_note_outlined),
                  title: const Text('Exam Schedule'),
                  subtitle: const Text('View your Exam Schedule'),
                  suffix: const Icon(FIcons.chevronRight),
                  onPress: () {
                    GoRouter.of(context).pushNamed(Paths.examSchedule);
                  },
                ),
              ],
            ),
            VtopCard(),
            _studentProjectsCard(context, ref),
            if (ref.watch(wificardSettingProvider)) WifiCard(),
          ],
        ),
      ),
    );
  }

  Widget _studentProjectsCard(BuildContext context, WidgetRef ref) {
    final colors = context.theme.colors;
    final pinnedIds = ref.watch(studentProjectPinnedIdsProvider);
    final pinnedOnly = ref.watch(studentProjectsPinnedOnlySessionProvider);
    final controller = ref.read(studentProjectsSettingsControllerProvider);
    final visibleProjects =
        pinnedOnly
            ? _studentProjects.where((p) => pinnedIds.contains(p.id)).toList()
            : _studentProjects;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Expanded(
              child: Text(
                "Projects by Students",
                style: TextStyle(
                  color: colors.primary,
                  fontWeight: FontWeight.w700,
                  fontSize: 18,
                ),
              ),
            ),
            FTappable(
              onPress:
                  () =>
                      ref
                          .read(studentProjectsPinnedOnlySessionProvider.notifier)
                          .state = !pinnedOnly,
              child: FBadge(
                style: FBadgeStyle.outline(),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      pinnedOnly ? Icons.push_pin : Icons.push_pin_outlined,
                      size: 14,
                    ),
                    const SizedBox(width: 6),
                    Text(
                      pinnedOnly ? "Pinned only" : "All projects",
                      style: const TextStyle(fontSize: 12),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
        const SizedBox(height: 4),
        Text(
          "Explore useful student-built tools",
          style: TextStyle(color: MoreColors.secondaryText),
        ),
        const SizedBox(height: 10),
        if (visibleProjects.isEmpty)
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: colors.secondary.withValues(alpha: .08),
              borderRadius: BorderRadius.circular(10),
              border: Border.all(color: colors.border),
            ),
            child: const Text("No pinned projects yet. Pin one to see it here."),
          )
        else
        LayoutBuilder(
          builder: (context, constraints) {
            final cardWidth = (constraints.maxWidth * 0.82).clamp(240.0, 320.0);
            final previewHeight = cardWidth * 9 / 16;
            final listHeight = previewHeight + 205;
            return SizedBox(
              height: listHeight,
              child: ListView.separated(
                scrollDirection: Axis.horizontal,
                itemCount: visibleProjects.length,
                separatorBuilder: (_, _) => const SizedBox(width: 10),
                itemBuilder: (context, index) {
                  final p = visibleProjects[index];
                  final isPinned = pinnedIds.contains(p.id);
                  return SizedBox(
                    width: cardWidth,
                    child: FCard(
                      image: _projectPreview(p.url, previewHeight),
                      title: Row(
                        children: [
                          Expanded(
                            child: Text(
                              p.name,
                              maxLines: 2,
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                          IconButton(
                            visualDensity: VisualDensity.compact,
                            tooltip: isPinned ? "Unpin Project" : "Pin Project",
                            onPressed: () => controller.togglePinned(p.id),
                            icon: Icon(
                              isPinned ? Icons.push_pin : Icons.push_pin_outlined,
                              size: 20,
                            ),
                          ),
                        ],
                      ),
                      subtitle: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            p.type,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: TextStyle(
                              fontSize: 12,
                              color: MoreColors.secondaryText,
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                          Text(
                            p.desc,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ],
                      ),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            "Made by: ${p.madeBy}",
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: TextStyle(
                              fontSize: 12,
                              color: MoreColors.secondaryText,
                            ),
                          ),
                          const SizedBox(height: 8),
                          Align(
                            alignment: Alignment.centerRight,
                            child: FButton(
                              style: FButtonStyle.outline(),
                              onPress: () async {
                                await launchUrl(
                                  Uri.parse(p.url),
                                  mode: LaunchMode.externalApplication,
                                );
                              },
                              child: const Text("View Project"),
                            ),
                          ),
                        ],
                      ),
                    ),
                  );
                },
              ),
            );
          },
        ),
        const SizedBox(height: 4),
        FTappable(
          onPress: () async {
            await launchUrl(
              Uri.parse("https://www.instagram.com/itsKryxen"),
              mode: LaunchMode.externalApplication,
            );
          },
          child: Text(
            "Want your project here? Contact me on Instagram: @itsKryxen",
            style: TextStyle(
              color: colors.primary,
              decoration: TextDecoration.underline,
            ),
          ),
        ),
      ],
    );
  }

  Widget _projectPreview(String url, double height) {
    final cached = _previewCache[url];
    if (cached != null) {
      return _networkPreview(cached, height);
    }
    return FutureBuilder<String?>(
      future: _fetchPreviewImage(url),
      builder: (context, snapshot) {
        final preview = snapshot.data;
        if (preview != null && preview.isNotEmpty) {
          return _networkPreview(preview, height);
        }
        return Container(
          height: height,
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(8),
            gradient: const LinearGradient(
              colors: [Color.fromARGB(255, 6, 71, 212), Color(0xFF2563EB)],
              begin: Alignment.topLeft,
              end: Alignment.bottomRight,
            ),
          ),
          child: const Center(
            child: Icon(
              Icons.rocket_launch_rounded,
              color: Colors.white,
              size: 40,
            ),
          ),
        );
      },
    );
  }

  Widget _networkPreview(String imageUrl, double height) {
    return Container(
      height: height,
      clipBehavior: Clip.antiAlias,
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(8),
        color: Colors.transparent,
      ),
      child: Center(
        child: Padding(
          padding: const EdgeInsets.all(8),
          child: Image.network(
            imageUrl,
            fit: BoxFit.contain,
            alignment: Alignment.center,
            width: double.infinity,
            height: double.infinity,
            errorBuilder: (_, _, _) => Container(
              decoration: const BoxDecoration(
                gradient: LinearGradient(
                  colors: [Color(0xFF06B6D4), Color(0xFF2563EB)],
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                ),
              ),
              child: const Center(
                child: Icon(
                  Icons.rocket_launch_rounded,
                  color: Colors.white,
                  size: 40,
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  Future<String?> _fetchPreviewImage(String url) async {
    try {
      final cached = _previewCache[url];
      if (cached != null) return cached;
      final uri = Uri.parse(url);
      final response = await http
          .get(uri, headers: {"User-Agent": "vitapmate/1.0"})
          .timeout(const Duration(seconds: 6));
      if (response.statusCode < 200 || response.statusCode >= 300) {
        _previewCache[url] = null;
        return null;
      }
      final html = response.body;
      String? extract(String pattern) {
        final reg = RegExp(pattern, caseSensitive: false);
        final match = reg.firstMatch(html);
        return match?.group(1);
      }

      final raw =
          extract(
            r'<meta[^>]+property=["' "'" r']og:image["' "'" r'][^>]+content=["' "'" r']([^"' "'" r']+)["' "'" r']',
          ) ??
          extract(
            r'<meta[^>]+name=["' "'" r']twitter:image["' "'" r'][^>]+content=["' "'" r']([^"' "'" r']+)["' "'" r']',
          ) ??
          extract(
            r'<link[^>]+rel=["' "'" r']image_src["' "'" r'][^>]+href=["' "'" r']([^"' "'" r']+)["' "'" r']',
          );

      if (raw == null || raw.isEmpty) {
        _previewCache[url] = null;
        return null;
      }
      final parsed = Uri.tryParse(raw.trim());
      if (parsed == null) {
        _previewCache[url] = null;
        return null;
      }
      final resolved = parsed.hasScheme ? parsed : uri.resolveUri(parsed);
      final out = resolved.toString();
      _previewCache[url] = out;
      return out;
    } catch (_) {
      _previewCache[url] = null;
      return null;
    }
  }
}

class VtopCard extends HookConsumerWidget {
  const VtopCard({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isLoading = useState(false);
    final colors = context.theme.colors;
    void handelclick() async {
      isLoading.value = true;
      try {
        await ref.read(vClientProvider.notifier).tryLogin();
        var client = await ref.read(vClientProvider.future);
        if (await fetchIsAuth(client: client)) {
          if (context.mounted) {
            GoRouter.of(context).pushNamed(Paths.vtopweb);
          }
        } else {
          if (context.mounted) {
            disCommonToast(context, Error());
          }
        }
      } catch (e) {
        if (context.mounted) {
          disCommonToast(context, e);
        }
      } finally {
        isLoading.value = false;
      }
    }

    return cardConatiner(
      child:
          !isLoading.value
              ? FButton(
                onPress: () async {
                  handelclick();
                },
                child: Text("Open"),
              )
              : Center(
                child: const SizedBox(
                  height: 20,
                  width: 20,
                  child: CircularProgressIndicator(
                    strokeWidth: 2,
                    color: MoreColors.infoBorder,
                  ),
                ),
              ),
      colors: colors,
      title: 'VTOP',
      desc: 'No login requried',
    );
  }
}

Widget cardConatiner({
  Widget? child,
  required FColors colors,
  required String title,
  required String desc,
  
}) {
  return Container(
    decoration: BoxDecoration(
      borderRadius: BorderRadius.circular(12),

      boxShadow: [
        BoxShadow(
          color: MoreColors.cardShadow,
          blurRadius: 8,
          offset: const Offset(0, 2),
        ),
      ],
    ),
    child: FCard(
      title: Center(
        child: Text(
          title,
          style: TextStyle(color: colors.primary, fontWeight: FontWeight.w600),
        ),
      ),
      subtitle: Center(
        child: Text(desc, style: TextStyle(color: colors.primary)),
      ),
      child: child,
    ),
  );
}
