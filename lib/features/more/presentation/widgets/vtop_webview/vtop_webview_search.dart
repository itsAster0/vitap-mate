import 'dart:math';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/core/utils/vtop_webview_pages.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';

Future<void> showVtopPageSearch(
  BuildContext context, {
  required Future<List<VtopPage>> pages,
  required List<VtopPage> recent,
  required ValueChanged<VtopPage> onSelect,
}) {
  return showFSheet(
    context: context,
    side: FLayout.btt,
    useRootNavigator: true,
    mainAxisMaxRatio: 0.88,
    builder: (context) => SizedBox(
      // The sheet rises above the keyboard, so it must not also grow into
      // the status bar.
      height: min(
        MediaQuery.sizeOf(context).height * 0.88,
        MediaQuery.sizeOf(context).height -
            MediaQuery.viewInsetsOf(context).bottom -
            MediaQuery.paddingOf(context).top -
            Space.sm,
      ),
      child: VtopPageSearchSheet(
        pages: pages,
        recent: recent,
        onSelect: (page) {
          Navigator.of(context).pop();
          onSelect(page);
        },
      ),
    ),
  );
}

/// Every VTOP sidebar page, searchable, with recently opened pages first.
class VtopPageSearchSheet extends HookWidget {
  const VtopPageSearchSheet({
    required this.pages,
    required this.recent,
    required this.onSelect,
    super.key,
  });

  final Future<List<VtopPage>> pages;
  final List<VtopPage> recent;
  final ValueChanged<VtopPage> onSelect;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final search = useTextEditingController();
    final query = useValueListenable(search).text.trim().toLowerCase();
    final snapshot = useFuture(useMemoized(() => pages));
    final all = snapshot.data ?? const <VtopPage>[];

    Widget caption(String text) => Padding(
      padding: const EdgeInsets.fromLTRB(4, Space.lg, 4, Space.sm),
      child: Text(
        text.toUpperCase(),
        style: typography.body.xs.copyWith(
          color: colors.mutedForeground,
          fontWeight: FontWeight.w600,
          letterSpacing: 0.6,
        ),
      ),
    );

    Widget group(List<VtopPage> pages, {bool showSection = false}) =>
        FItemGroup(
          children: [
            for (final page in pages)
              FItem(
                title: Text(page.title),
                subtitle: showSection && page.section.isNotEmpty
                    ? Text(page.section)
                    : null,
                suffix: const Icon(FLucideIcons.chevronRight),
                onPress: () => onSelect(page),
              ),
          ],
        );

    final children = <Widget>[];
    if (query.isNotEmpty) {
      final words = query.split(RegExp(r'\s+'));
      final matches = all.where((page) {
        final text = '${page.title} ${page.section}'.toLowerCase();
        return words.every(text.contains);
      }).toList();
      // Title matches first; section-only matches after.
      matches.sort((a, b) {
        final aTitle = a.title.toLowerCase().contains(words.first) ? 0 : 1;
        final bTitle = b.title.toLowerCase().contains(words.first) ? 0 : 1;
        return aTitle - bTitle;
      });
      if (matches.isEmpty) {
        children.add(_Message(text: 'No VTOP page matches "$query".'));
      } else {
        children
          ..add(caption('${matches.length} pages'))
          ..add(group(matches, showSection: true));
      }
    } else {
      if (recent.isNotEmpty) {
        children
          ..add(caption('Recent'))
          ..add(group(recent, showSection: true));
      }
      final sections = <String, List<VtopPage>>{};
      for (final page in all) {
        sections.putIfAbsent(page.section, () => []).add(page);
      }
      // Links outside any sidebar section go last.
      final other = sections.remove('');
      if (other != null) sections['Other'] = other;
      for (final MapEntry(key: section, value: pages) in sections.entries) {
        children
          ..add(caption(section))
          ..add(group(pages));
      }
      if (all.isEmpty) {
        children.add(
          snapshot.connectionState == ConnectionState.done
              ? const _Message(
                  text: 'VTOP pages appear once VTOP has finished loading.',
                )
              : const Padding(
                  padding: EdgeInsets.all(Space.xl),
                  child: Center(
                    child: FCircularProgress(
                      semanticsLabel: 'Loading VTOP pages',
                    ),
                  ),
                ),
        );
      }
    }

    return DecoratedBox(
      decoration: BoxDecoration(
        color: colors.background,
        borderRadius: const BorderRadius.vertical(
          top: Radius.circular(Radii.lg + 4),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const SheetHandle(),
          Padding(
            padding: const EdgeInsets.fromLTRB(
              Space.page,
              Space.sm,
              Space.page,
              0,
            ),
            child: FTextField(
              control: FTextFieldControl.managed(controller: search),
              hint: 'Search VTOP pages',
              autofocus: true,
              textInputAction: TextInputAction.search,
              prefixBuilder: (_, _, _) => Padding(
                padding: const EdgeInsets.only(left: 12, right: 8),
                child: Icon(
                  FLucideIcons.search,
                  size: 18,
                  color: colors.mutedForeground,
                ),
              ),
              suffixBuilder: (_, _, _) => query.isEmpty
                  ? const SizedBox.shrink()
                  : IconButton(
                      tooltip: 'Clear search',
                      onPressed: search.clear,
                      icon: Icon(
                        FLucideIcons.x,
                        size: 18,
                        color: colors.mutedForeground,
                      ),
                    ),
            ),
          ),
          Expanded(
            child: ListView(
              keyboardDismissBehavior: ScrollViewKeyboardDismissBehavior.onDrag,
              padding: EdgeInsets.fromLTRB(Space.page, 0, Space.page, Space.xl),
              children: children,
            ),
          ),
        ],
      ),
    );
  }
}

class _Message extends StatelessWidget {
  const _Message({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(Space.xl),
      child: Text(
        text,
        textAlign: TextAlign.center,
        style: context.theme.typography.body.sm.copyWith(
          color: context.theme.colors.mutedForeground,
        ),
      ),
    );
  }
}
