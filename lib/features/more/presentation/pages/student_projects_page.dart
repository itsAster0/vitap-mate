import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:http/http.dart' as http;
import 'package:url_launcher/url_launcher.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/features/more/presentation/providers/student_projects_provider.dart';

class StudentProjectsPage extends ConsumerWidget {
  const StudentProjectsPage({super.key});

  static final Map<String, String?> _previewCache = {};

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final data =
        ref.watch(studentProjectsProvider).value ?? defaultStudentProjectsData;
    final pinnedOnly = ref.watch(studentProjectsPinnedOnlySessionProvider);
    final pinnedIds = ref.watch(studentProjectPinnedIdsProvider);
    final controller = ref.read(studentProjectsSettingsControllerProvider);

    final categories = data.categories.isEmpty
        ? StudentProjectCategory.defaultCategories
        : data.categories;
    final grouped = <String, List<StudentProject>>{
      for (final c in categories) c.id: <StudentProject>[],
    };
    for (final project in data.projects) {
      if (pinnedOnly && !pinnedIds.contains(project.id)) continue;
      grouped
          .putIfAbsent(project.category, () => <StudentProject>[])
          .add(project);
    }

    return DefaultTabController(
      length: categories.length,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Expanded(
                child: Text(
                  "Projects by Students",
                  style: context.theme.typography.lg.copyWith(
                    color: context.theme.colors.foreground,
                    fontWeight: FontWeight.w700,
                  ),
                ),
              ),
              FTappable(
                onPress: () => ref
                    .read(studentProjectsPinnedOnlySessionProvider.notifier)
                    .setValue(!pinnedOnly),
                child: FBadge(
                  variant: FBadgeVariant.outline,
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
                        style: context.theme.typography.xs,
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            "Explore useful student-built tools and apps.",
            style: context.theme.typography.sm.copyWith(
              color: context.theme.colors.mutedForeground,
            ),
          ),
          const SizedBox(height: 8),
          Theme(
            data: Theme.of(context).copyWith(
              tabBarTheme: TabBarThemeData(
                labelColor: context.theme.colors.foreground,
                unselectedLabelColor: context.theme.colors.mutedForeground,
                indicatorColor: context.theme.colors.primary,
                labelStyle: context.theme.typography.sm.copyWith(
                  fontWeight: FontWeight.w600,
                ),
                unselectedLabelStyle: context.theme.typography.sm,
              ),
            ),
            child: TabBar(
              isScrollable: true,
              tabs: [
                for (final category in categories)
                  Tab(
                    text:
                        "${category.label} (${(grouped[category.id] ?? const []).length})",
                  ),
              ],
            ),
          ),
          const SizedBox(height: 8),
          Expanded(
            child: TabBarView(
              children: [
                for (final category in categories)
                  _CategoryTabView(
                    category: category,
                    projects: grouped[category.id] ?? const [],
                    pinnedIds: pinnedIds,
                    controller: controller,
                    cta: data.cta,
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _CategoryTabView extends ConsumerWidget {
  const _CategoryTabView({
    required this.category,
    required this.projects,
    required this.pinnedIds,
    required this.controller,
    required this.cta,
  });

  final StudentProjectCategory category;
  final List<StudentProject> projects;
  final Set<int> pinnedIds;
  final StudentProjectsSettingsController controller;
  final StudentProjectsCta cta;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return ListView(
      padding: const EdgeInsets.only(bottom: 8),
      children: [
        Container(
          margin: const EdgeInsets.only(bottom: 10),
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: context.theme.colors.secondary.withValues(alpha: .08),
            borderRadius: BorderRadius.circular(10),
            border: Border.all(color: context.theme.colors.border),
          ),
          child: Text(category.description),
        ),
        if (projects.isEmpty)
          Container(
            margin: const EdgeInsets.only(bottom: 12),
            padding: const EdgeInsets.all(12),
            child: const Text("No projects in this category."),
          ),
        for (final p in projects)
          Padding(
            padding: const EdgeInsets.only(bottom: 10),
            child: _ProjectCard(
              project: p,
              isPinned: pinnedIds.contains(p.id),
              onTogglePin: () => controller.togglePinned(p.id),
            ),
          ),
        FTappable(
          onPress: () async {
            final uri = Uri.tryParse(cta.url);
            if (uri == null) return;
            await launchUrl(uri, mode: LaunchMode.externalApplication);
          },
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 6),
            child: Text(
              cta.label,
              style: TextStyle(
                color: context.theme.colors.primary,
                decoration: TextDecoration.underline,
              ),
            ),
          ),
        ),
      ],
    );
  }
}

class _ProjectCard extends StatelessWidget {
  const _ProjectCard({
    required this.project,
    required this.isPinned,
    required this.onTogglePin,
  });

  final StudentProject project;
  final bool isPinned;
  final VoidCallback onTogglePin;

  @override
  Widget build(BuildContext context) {
    return FCard(
      image: _projectPreview(project, context),
      title: Row(
        children: [
          Expanded(
            child: Text(
              project.name,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
            ),
          ),
          IconButton(
            visualDensity: VisualDensity.compact,
            tooltip: isPinned ? "Unpin Project" : "Pin Project",
            onPressed: onTogglePin,
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
            project.type,
            style: TextStyle(
              fontSize: context.theme.typography.xs.fontSize,
              color: context.theme.colors.mutedForeground,
              fontWeight: FontWeight.w600,
            ),
          ),
          Text(project.summary, maxLines: 3, overflow: TextOverflow.ellipsis),
        ],
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            "Made by: ${project.madeBy}",
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: TextStyle(
              fontSize: context.theme.typography.xs.fontSize,
              color: context.theme.colors.mutedForeground,
            ),
          ),
          const SizedBox(height: 6),
          if (project.tags.isNotEmpty)
            Wrap(
              spacing: 6,
              runSpacing: 6,
              children: [
                for (final tag in project.tags.take(4))
                  FBadge(
                    variant: FBadgeVariant.outline,
                    child: Text(
                      tag,
                      style: context.theme.typography.xs.copyWith(
                        fontSize:
                            (context.theme.typography.xs.fontSize ?? 12) - 1,
                      ),
                    ),
                  ),
              ],
            ),
          const SizedBox(height: 10),
          Row(
            children: [
              Expanded(
                child: FButton(
                  variant: FButtonVariant.outline,
                  onPress: () async {
                    await _openExternal(project.url);
                  },
                  child: const Text("Open"),
                ),
              ),
              if (project.sourceUrl != null)
                Padding(
                  padding: const EdgeInsets.only(left: 8),
                  child: FTappable(
                    onPress: () async {
                      await _openExternal(project.sourceUrl!);
                    },
                    child: Container(
                      padding: const EdgeInsets.all(8),
                      decoration: BoxDecoration(
                        borderRadius: BorderRadius.circular(8),
                        border: Border.all(color: context.theme.colors.border),
                      ),
                      child: const Icon(Icons.code, size: 16),
                    ),
                  ),
                ),
            ],
          ),
        ],
      ),
    );
  }

  Future<void> _openExternal(String url) async {
    final uri = Uri.tryParse(url);
    if (uri == null) return;
    await launchUrl(uri, mode: LaunchMode.externalApplication);
  }

  Widget _projectPreview(StudentProject project, BuildContext context) {
    const height = 180.0;
    if (project.previewImage != null && project.previewImage!.isNotEmpty) {
      return _networkPreview(project.previewImage!, height, context);
    }
    final cached = StudentProjectsPage._previewCache[project.url];
    if (cached != null) {
      return _networkPreview(cached, height, context);
    }
    return FutureBuilder<String?>(
      future: _fetchPreviewImage(project.url),
      builder: (context, snapshot) {
        final preview = snapshot.data;
        if (preview != null && preview.isNotEmpty) {
          return _networkPreview(preview, height, context);
        }
        return _previewFallback(context, height);
      },
    );
  }

  Widget _networkPreview(String imageUrl, double height, BuildContext context) {
    return SizedBox(
      height: height,
      child: Center(
        child: Padding(
          padding: const EdgeInsets.all(8),
          child: CachedNetworkImage(
            imageUrl: imageUrl,
            fit: BoxFit.contain,
            alignment: Alignment.center,
            width: double.infinity,
            height: double.infinity,
            errorWidget: (_, _, _) => _previewFallback(context, height),
            placeholder: (_, _) => _previewFallback(context, height),
          ),
        ),
      ),
    );
  }

  Widget _previewFallback(BuildContext context, double height) {
    return Container(
      height: height,
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(8),
        gradient: LinearGradient(
          colors: [
            context.theme.colors.primary,
            context.theme.colors.secondary,
          ],
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
        ),
      ),
      child: Center(
        child: Icon(
          Icons.rocket_launch_rounded,
          color: context.theme.colors.primaryForeground,
          size: 40,
        ),
      ),
    );
  }

  Future<String?> _fetchPreviewImage(String url) async {
    try {
      final cached = StudentProjectsPage._previewCache[url];
      if (cached != null) return cached;
      final uri = Uri.parse(url);
      final response = await http
          .get(uri, headers: {"User-Agent": "vitapmate/1.0"})
          .timeout(const Duration(seconds: 6));
      if (response.statusCode < 200 || response.statusCode >= 300) {
        StudentProjectsPage._previewCache[url] = null;
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
            r'<meta[^>]+property=["'
            "'"
            r']og:image["'
            "'"
            r'][^>]+content=["'
            "'"
            r']([^"'
            "'"
            r']+)["'
            "'"
            r']',
          ) ??
          extract(
            r'<meta[^>]+name=["'
            "'"
            r']twitter:image["'
            "'"
            r'][^>]+content=["'
            "'"
            r']([^"'
            "'"
            r']+)["'
            "'"
            r']',
          ) ??
          extract(
            r'<link[^>]+rel=["'
            "'"
            r']image_src["'
            "'"
            r'][^>]+href=["'
            "'"
            r']([^"'
            "'"
            r']+)["'
            "'"
            r']',
          );

      if (raw == null || raw.isEmpty) {
        StudentProjectsPage._previewCache[url] = null;
        return null;
      }
      final parsed = Uri.tryParse(raw.trim());
      if (parsed == null) {
        StudentProjectsPage._previewCache[url] = null;
        return null;
      }
      final resolved = parsed.hasScheme ? parsed : uri.resolveUri(parsed);
      final out = resolved.toString();
      StudentProjectsPage._previewCache[url] = out;
      return out;
    } catch (_) {
      StudentProjectsPage._previewCache[url] = null;
      return null;
    }
  }
}
