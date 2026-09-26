import 'dart:convert';

import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/providers/settings.dart';

/// A VTOP sidebar page. An empty [url] is VTOP's Home dashboard.
class VtopPage {
  const VtopPage({required this.url, required this.title, this.section = ''});

  static const home = VtopPage(url: '', title: '');

  final String url;
  final String title;
  final String section;

  bool get isHome => url.isEmpty;

  static VtopPage? fromJson(Object? json) {
    if (json is! Map) return null;
    final url = json['url'];
    final title = json['title'];
    if (url is! String || title is! String || url.isEmpty || title.isEmpty) {
      return null;
    }
    final section = json['section'];
    return VtopPage(
      url: url,
      title: title,
      section: section is String ? section : '',
    );
  }

  Map<String, String> toJson() => {
    'url': url,
    'title': title,
    'section': section,
  };
}

const _maxRecentPages = 6;

/// Recent pages belong to one account; another account's list reads as empty.
List<VtopPage> readRecentVtopPages(
  SharedPreferencesWithCache prefs,
  String username,
) {
  try {
    final stored = jsonDecode(prefs.getString(vtopRecentPagesSettingKey) ?? '');
    if (stored is! Map || stored['user'] != username.toUpperCase()) return [];
    final pages = stored['pages'];
    if (pages is! List) return [];
    return pages.map(VtopPage.fromJson).whereType<VtopPage>().toList();
  } catch (_) {
    return [];
  }
}

Future<void> addRecentVtopPage(
  SharedPreferencesWithCache prefs,
  String username,
  VtopPage page,
) {
  final pages = [
    page,
    ...readRecentVtopPages(
      prefs,
      username,
    ).where((recent) => recent.url != page.url),
  ].take(_maxRecentPages);
  return prefs.setString(
    vtopRecentPagesSettingKey,
    jsonEncode({
      'user': username.toUpperCase(),
      'pages': pages.map((page) => page.toJson()).toList(),
    }),
  );
}

/// Recently opened VTOP pages for the signed-in account, newest first.
class VtopRecentPages extends Notifier<List<VtopPage>> {
  Future<void> _writes = Future.value();

  @override
  List<VtopPage> build() {
    final prefs = ref.watch(settingsProvider).value;
    final username = ref.watch(vtopUserProvider).value?.username;
    if (prefs == null || username == null) return const [];
    return readRecentVtopPages(prefs, username);
  }

  Future<void> add(VtopPage page) {
    final username = ref.read(vtopUserProvider).value?.username;
    if (username == null || page.isHome) return Future.value();
    state = [
      page,
      ...state.where((recent) => recent.url != page.url),
    ].take(_maxRecentPages).toList();
    final write = _writes.then((_) async {
      final prefs = await ref.read(settingsProvider.future);
      await addRecentVtopPage(prefs, username, page);
    });
    // Recents are a convenience; a failed write only loses the ordering.
    _writes = write.catchError((Object _) {});
    return _writes;
  }
}

final vtopRecentPagesProvider =
    NotifierProvider<VtopRecentPages, List<VtopPage>>(VtopRecentPages.new);
