import 'dart:convert';

import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/vtop/vtop_user_provider.dart';
import 'package:vitapmate/app/settings.dart';

/// A VTOP sidebar page. An empty [url] is VTOP's Home dashboard.
class VtopPage {
  const VtopPage({required this.url, required this.title, this.section = ''});

  static const home = VtopPage(url: '', title: '');

  final String url;
  final String title;
  final String section;

  bool get isHome => url.isEmpty;

  static VtopPage? fromJson(Object? json) {
    if (json case {
      'url': final String url,
      'title': final String title,
    } when url.isNotEmpty && title.isNotEmpty) {
      final section = json['section'];
      return VtopPage(
        url: url,
        title: title,
        section: section is String ? section : '',
      );
    }
    return null;
  }

  Map<String, String> toJson() => {
    'url': url,
    'title': title,
    'section': section,
  };
}

const _maxRecentPages = 6;

/// Recent pages belong to one account; another account's list reads as empty.
List<VtopPage> _readRecentVtopPages(
  SharedPreferencesWithCache prefs,
  String username,
) {
  try {
    final stored = jsonDecode(prefs.getString(vtopRecentPagesSettingKey) ?? '');
    if (stored case {
      'user': final user,
      'pages': final List pages,
    } when user == username.toUpperCase()) {
      return pages.map(VtopPage.fromJson).whereType<VtopPage>().toList();
    }
    return [];
  } catch (_) {
    return [];
  }
}

/// Recently opened VTOP pages for the signed-in account, newest first.
class VtopRecentPages extends Notifier<List<VtopPage>> {
  Future<void> _writes = Future.value();

  @override
  List<VtopPage> build() {
    final prefs = ref.watch(settingsProvider).value;
    final username = ref.watch(vtopUserProvider).value?.username;
    if (prefs == null || username == null) return const [];
    return _readRecentVtopPages(prefs, username);
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
      final pages = [
        page,
        ..._readRecentVtopPages(
          prefs,
          username,
        ).where((recent) => recent.url != page.url),
      ].take(_maxRecentPages);
      await prefs.setString(
        vtopRecentPagesSettingKey,
        jsonEncode({
          'user': username.toUpperCase(),
          'pages': [for (final page in pages) page.toJson()],
        }),
      );
    });
    // Recents are a convenience; a failed write only loses the ordering.
    _writes = write.catchError((Object _) {});
    return _writes;
  }
}

final vtopRecentPagesProvider =
    NotifierProvider<VtopRecentPages, List<VtopPage>>(VtopRecentPages.new);
