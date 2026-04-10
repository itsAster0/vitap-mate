import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/services/service_layer.dart';

class LogsPage extends ConsumerStatefulWidget {
  const LogsPage({super.key});

  @override
  ConsumerState<LogsPage> createState() => _LogsPageState();
}

class _LogsPageState extends ConsumerState<LogsPage> {
  final TextEditingController _searchController = TextEditingController();
  LogLevel? _selectedLevel;
  String? _selectedSource;

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final logsAsync = ref.watch(logsProvider);
    final query = _searchController.text.trim().toLowerCase();

    return Container(
      color: context.theme.colors.background,
      child: logsAsync.when(
        data: (entries) {
          final filtered = entries
              .where((entry) {
                final levelMatches =
                    _selectedLevel == null || entry.level == _selectedLevel;
                final sourceMatches =
                    _selectedSource == null || entry.source == _selectedSource;
                final queryMatches =
                    query.isEmpty ||
                    entry.source.toLowerCase().contains(query) ||
                    entry.message.toLowerCase().contains(query) ||
                    (entry.error?.toLowerCase().contains(query) ?? false);
                return levelMatches && sourceMatches && queryMatches;
              })
              .toList(growable: false);

          return Column(
            children: [
              Padding(
                padding: const EdgeInsets.all(8),
                child: FTextField(
                  controller: _searchController,
                  hint: 'Search logs',
                  onChange: (_) => setState(() {}),
                ),
              ),
              SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                padding: const EdgeInsets.symmetric(horizontal: 8),
                child: Row(
                  children: [
                    _FilterButton(
                      selected: _selectedSource == null,
                      label: 'All Sources',
                      onPress: () => setState(() => _selectedSource = null),
                    ),
                    const SizedBox(width: 8),
                    _FilterButton(
                      selected: _selectedSource == 'bridge',
                      label: 'Bridge',
                      onPress: () => setState(() => _selectedSource = 'bridge'),
                    ),
                    const SizedBox(width: 16),
                    _FilterButton(
                      selected: _selectedLevel == null,
                      label: 'All',
                      onPress: () => setState(() => _selectedLevel = null),
                    ),
                    const SizedBox(width: 8),
                    for (final level in LogLevel.values) ...[
                      _FilterButton(
                        selected: _selectedLevel == level,
                        label: level.name.toUpperCase(),
                        onPress: () => setState(() => _selectedLevel = level),
                      ),
                      const SizedBox(width: 8),
                    ],
                    FButton.icon(
                      onPress: () async {
                        final logger = await ref.read(appLoggerProvider.future);
                        await logger.clear();
                      },
                      child: const Icon(FIcons.trash),
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 8),
              Expanded(
                child: ListView.separated(
                  padding: const EdgeInsets.fromLTRB(8, 0, 8, 24),
                  itemCount: filtered.length,
                  separatorBuilder: (_, _) => const SizedBox(height: 8),
                  itemBuilder: (context, index) {
                    final entry = filtered[index];
                    return FCard(
                      title: Row(
                        children: [
                          Expanded(
                            child: Text(
                              entry.message,
                              style: context.theme.typography.base.copyWith(
                                fontWeight: FontWeight.w700,
                              ),
                            ),
                          ),
                          Text(entry.level.name.toUpperCase()),
                        ],
                      ),
                      subtitle: Text(
                        '${entry.source} - ${entry.timestamp.toLocal()}',
                      ),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          if (entry.error != null) ...[Text(entry.error!)],
                          if (entry.stackTrace != null) ...[
                            const SizedBox(height: 8),
                            Text(
                              entry.stackTrace!,
                              maxLines: 6,
                              overflow: TextOverflow.ellipsis,
                              style: context.theme.typography.xs,
                            ),
                          ],
                        ],
                      ),
                    );
                  },
                ),
              ),
            ],
          );
        },
        loading: () => Center(child: FCircularProgress()),
        error: (error, _) => Center(child: Text(error.toString())),
      ),
    );
  }
}

class _FilterButton extends StatelessWidget {
  const _FilterButton({
    required this.selected,
    required this.label,
    required this.onPress,
  });

  final bool selected;
  final String label;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    if (selected) {
      return FButton(onPress: onPress, child: Text(label));
    }
    return FButton(
      style: FButtonStyle.outline(),
      onPress: onPress,
      child: Text(label),
    );
  }
}
