import 'dart:convert';
import 'dart:developer' show log;

import 'package:flutter/material.dart' show RefreshIndicator;
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/platform/files/general_utils.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/ui/data_updated_footer.dart';
import 'package:vitapmate/app/widgets/screen_refresh.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/profile/presentation/providers/profile_providers.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

enum ProfileTab { profile, mentor }

/// The student's VTOP profile, and their mentor with the mentor's messages.
class ProfilePage extends HookConsumerWidget {
  const ProfilePage({super.key, this.initialTab = ProfileTab.profile});

  final ProfileTab initialTab;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tab = useState(initialTab);
    final colors = context.theme.colors;

    Future<void> refresh() async {
      try {
        await switch (tab.value) {
          ProfileTab.profile => ref.read(profileProvider.notifier).refresh(),
          ProfileTab.mentor => ref.read(mentorProvider.notifier).refresh(),
        };
      } catch (e) {
        log('$e');
        if (context.mounted) disCommonToast(context, e);
      }
    }

    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) async {
        if (!await isAutoRefreshEnabled(ref)) return;
        for (final notifier in [
          ref.read(profileProvider.notifier).refresh,
          ref.read(mentorProvider.notifier).refresh,
        ]) {
          notifier().catchError((e, st) {
            log('auto refresh failed: $e', stackTrace: st);
          });
        }
      });
      return null;
    }, const []);

    return ScreenRefresh(
      onRefresh: refresh,
      tasks: const ['vtop_student_profile', 'vtop_mentor'],
      child: RefreshIndicator(
        onRefresh: refresh,
        backgroundColor: colors.background,
        color: colors.foreground,
        child: ListView(
          padding: const EdgeInsets.fromLTRB(
            Space.sm,
            Space.sm,
            Space.sm,
            Space.lg,
          ),
          physics: const AlwaysScrollableScrollPhysics(),
          children: [
            Segmented<ProfileTab>(
              segments: const [
                (ProfileTab.profile, 'Profile'),
                (ProfileTab.mentor, 'Mentor'),
              ],
              value: tab.value,
              onChanged: (value) => tab.value = value,
            ),
            const SizedBox(height: Space.md),
            AnimatedSwitcher(
              duration: Motion.medium,
              child: switch (tab.value) {
                ProfileTab.profile => const _ProfileTab(
                  key: ValueKey(ProfileTab.profile),
                ),
                ProfileTab.mentor => const _MentorTab(
                  key: ValueKey(ProfileTab.mentor),
                ),
              },
            ),
          ],
        ),
      ),
    );
  }
}

/// Loading, error and data states shared by both tabs.
class _Loaded<T> extends ConsumerWidget {
  const _Loaded({
    required this.value,
    required this.what,
    required this.onRetry,
    required this.builder,
  });

  final AsyncValue<T> value;
  final String what;
  final Future<void> Function() onRetry;
  final Widget Function(T data) builder;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return value.when(
      skipLoadingOnRefresh: true,
      skipLoadingOnReload: true,
      loading: () => const Column(
        children: [
          Skeleton(height: 150, radius: Radii.lg),
          SizedBox(height: Space.lg),
          SkeletonList(count: 4, height: 72),
        ],
      ),
      error: (e, _) => EmptyState(
        icon: FLucideIcons.cloudAlert,
        title: "Couldn't load $what",
        message: commonErrorMessage(e),
        action: FButton(
          variant: FButtonVariant.outline,
          mainAxisSize: MainAxisSize.min,
          onPress: () async {
            try {
              await onRetry();
            } catch (e) {
              if (context.mounted) disCommonToast(context, e);
            }
          },
          child: const Text('Try again'),
        ),
      ),
      data: builder,
    );
  }
}

// ---------------------------------------------------------------------------
// Profile tab
// ---------------------------------------------------------------------------

/// The profile's sections sorted for display. The cards at the top (hostel,
/// contact, addresses, parents) pick VTOP's fields by label; every field
/// they do not show stays in the collapsed sections below, so nothing VTOP
/// sends is lost.
class _ProfileLayout {
  _ProfileLayout(StudentProfile profile) {
    for (final section in profile.sections) {
      final title = section.title.toUpperCase();
      if (title.contains('PROCTOR')) continue; // On the Mentor tab.
      if (title.contains('HOSTEL')) {
        hostel = _fields(section.groups);
        continue;
      }
      final rest = <ProfileGroup>[];
      for (final group in section.groups) {
        final groupTitle = group.title.toUpperCase();
        if (title.contains('PERSONAL') && groupTitle.contains('ADDRESS')) {
          addresses.add(group);
        } else if (title.contains('FAMILY') && groupTitle.contains('DETAILS')) {
          parents.add(group);
        } else {
          rest.add(group);
        }
      }
      if (title.contains('PERSONAL')) personal = rest;
      details.add((
        section.title,
        [
          for (final group in rest)
            ProfileGroup(
              title: group.title,
              fields: [
                for (final field in group.fields)
                  if (!_shownAbove(field.label)) field,
              ],
            ),
        ],
      ));
    }
    contacts = [
      if (_shown(profile.vitEmail)) ('VIT email', profile.vitEmail),
      if (_value(personal, 'MOBILE NUMBER') case final v?) ('Mobile', v),
      if (_value([...personal, ...addresses], 'PARENT WHATSAPP NUMBER')
          case final v?)
        ('Parent WhatsApp', v),
    ];
    for (final group in addresses) {
      final email = _value([group], 'EMAIL');
      if (email != null && !contacts.any((c) => c.$2 == email)) {
        contacts.add(('Personal email', email));
      }
    }
  }

  List<ProfileField> hostel = const [];
  List<ProfileGroup> personal = const [];
  final addresses = <ProfileGroup>[];
  final parents = <ProfileGroup>[];
  late final List<(String, String)> contacts;

  /// Section title and its groups, without what the cards show.
  final details = <(String, List<ProfileGroup>)>[];

  /// The name is in the header and the contacts are in their own card.
  static bool _shownAbove(String label) => const {
    'STUDENT NAME',
    'MOBILE NUMBER',
    'PARENT WHATSAPP NUMBER',
  }.contains(label.toUpperCase());
}

class _ProfileTab extends ConsumerWidget {
  const _ProfileTab({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return _Loaded(
      value: ref.watch(profileProvider),
      what: 'your profile',
      onRetry: () => ref.read(profileProvider.notifier).refresh(),
      builder: (profile) {
        final layout = _ProfileLayout(profile);
        final details = [
          for (final (title, groups) in layout.details)
            if (groups.any((g) => g.fields.any((f) => _shown(f.value))))
              (title, groups),
        ];
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            _ProfileHero(profile: profile),
            if (_HostelCard.hasRoom(layout.hostel)) ...[
              const SizedBox(height: Space.md),
              _HostelCard(fields: layout.hostel),
            ],
            if (layout.contacts.isNotEmpty) ...[
              const SectionHeader(title: 'Contact'),
              Surface(
                padding: const EdgeInsets.symmetric(
                  horizontal: Space.lg,
                  vertical: Space.sm,
                ),
                child: Column(
                  children: [
                    for (final (label, value) in layout.contacts)
                      _ContactRow.of(label, value),
                  ],
                ),
              ),
            ],
            if (layout.addresses.isNotEmpty) ...[
              const SectionHeader(title: 'Addresses'),
              _AddressCard(groups: layout.addresses),
            ],
            if (layout.parents.isNotEmpty) ...[
              const SectionHeader(title: 'Parents'),
              for (final group in layout.parents) ...[
                _ParentCard(group: group),
                const SizedBox(height: Space.sm),
              ],
            ],
            if (details.isNotEmpty) ...[
              const SectionHeader(title: 'More details'),
              for (final (title, groups) in details) ...[
                _Collapsible(title: _detailsTitle(title), groups: groups),
                const SizedBox(height: Space.sm),
              ],
            ],
            DataUpdatedFooter(updateTime: profile.updateTime.toInt()),
          ],
        );
      },
    );
  }
}

class _ProfileHero extends HookWidget {
  const _ProfileHero({required this.profile});

  final StudentProfile profile;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final photo = useMemoized(() => _decodePhoto(profile.photo), [
      profile.photo,
    ]);
    final name = _niceCase(profile.name);

    return Surface(
      child: Row(
        children: [
          ClipRRect(
            borderRadius: BorderRadius.circular(Radii.md),
            child: Container(
              width: 76,
              height: 92,
              color: colors.app.accentTone.subtle,
              alignment: Alignment.center,
              child: photo == null
                  ? Text(
                      _initials(name),
                      style: typography.body.xl.copyWith(
                        fontWeight: FontWeight.w700,
                        color: colors.app.accentTone.onSubtle,
                      ),
                    )
                  : Image.memory(
                      photo,
                      width: 76,
                      height: 92,
                      fit: BoxFit.cover,
                      gaplessPlayback: true,
                      errorBuilder: (_, _, _) => Icon(
                        FLucideIcons.user,
                        color: colors.app.accentTone.onSubtle,
                      ),
                    ),
            ),
          ),
          const SizedBox(width: Space.md),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  name,
                  style: typography.body.lg.copyWith(
                    fontWeight: FontWeight.w700,
                    color: colors.foreground,
                    height: 1.2,
                  ),
                ),
                const SizedBox(height: 6),
                if (_shown(profile.registrationNumber))
                  _Copyable(
                    value: profile.registrationNumber,
                    child: ToneBadge(
                      label: profile.registrationNumber,
                      tone: colors.app.accentTone,
                    ),
                  ),
                const SizedBox(height: Space.sm),
                if (_shown(profile.program))
                  Text(
                    profile.program,
                    style: typography.body.sm.copyWith(
                      color: colors.foreground,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                if (_shown(profile.school))
                  Text(
                    profile.school,
                    style: typography.body.xs.copyWith(
                      color: colors.mutedForeground,
                    ),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// Block and room up front, bed and mess underneath.
class _HostelCard extends StatelessWidget {
  const _HostelCard({required this.fields});

  final List<ProfileField> fields;

  static bool hasRoom(List<ProfileField> fields) =>
      _value([ProfileGroup(title: '', fields: fields)], 'ROOM NO.') != null;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final group = [ProfileGroup(title: '', fields: fields)];
    // "MH-2 (MH-2) (MH-2 - Block )" → "MH-2".
    final block = _value(group, 'BLOCK NAME')?.split('(').first.trim();
    final room = _value(group, 'ROOM NO.');
    final bed = _value(group, 'BED TYPE');
    // "Non-Veg Mess Fee - " → "Non-Veg Mess".
    final mess = _value(
      group,
      'MESS INFORMATION',
    )?.replaceAll(RegExp(r'\s*Fee[\s-]*$', caseSensitive: false), '');
    final tone = colors.app.lab;
    return Surface(
      child: Row(
        children: [
          Container(
            width: 44,
            height: 44,
            decoration: BoxDecoration(
              color: tone.subtle,
              borderRadius: BorderRadius.circular(Radii.md),
            ),
            child: Icon(FLucideIcons.bedDouble, size: 20, color: tone.onSubtle),
          ),
          const SizedBox(width: Space.md),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  [?block, if (room != null) 'Room $room'].join(' · '),
                  style: typography.body.md.copyWith(
                    fontWeight: FontWeight.w700,
                    color: colors.foreground,
                  ),
                ),
                if (bed != null || (mess != null && _shown(mess)))
                  Text(
                    [
                      if (bed != null) _bedType(bed),
                      if (mess != null && _shown(mess)) mess,
                    ].join(' · '),
                    style: typography.body.xs.copyWith(
                      color: colors.mutedForeground,
                    ),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// Each address as one block of text.
class _AddressCard extends StatelessWidget {
  const _AddressCard({required this.groups});

  final List<ProfileGroup> groups;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final addresses = [
      for (final group in groups)
        if (_address(group) case final text when text.isNotEmpty)
          (_niceCase(group.title.replaceAll(RegExp(r'\s*ADDRESS$'), '')), text),
    ];
    return Surface(
      padding: const EdgeInsets.symmetric(
        horizontal: Space.lg,
        vertical: Space.sm,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          for (final (i, (title, text)) in addresses.indexed) ...[
            if (i > 0) Container(height: 1, color: colors.border),
            _Copyable(
              value: text,
              child: Padding(
                padding: const EdgeInsets.symmetric(vertical: Space.sm + 2),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Padding(
                      padding: const EdgeInsets.only(top: 2),
                      child: Icon(
                        i == 0 ? FLucideIcons.mapPin : FLucideIcons.house,
                        size: 16,
                        color: colors.app.accentTone.base,
                      ),
                    ),
                    const SizedBox(width: Space.sm + 2),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            title,
                            style: typography.body.xs.copyWith(
                              color: colors.mutedForeground,
                            ),
                          ),
                          const SizedBox(height: 2),
                          Text(
                            text,
                            style: typography.body.sm.copyWith(
                              color: colors.foreground,
                              height: 1.4,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ],
      ),
    );
  }

  /// "3rd Floor, 7 Hills Homes, SVN Colony, Guntur, Andhra Pradesh 522006,
  /// India": the address lines in VTOP's order, the pincode after the state.
  static String _address(ProfileGroup group) {
    final parts = <String>[];
    String? pincode;
    for (final field in group.fields) {
      final label = field.label.toUpperCase();
      if (!_shown(field.value) ||
          label.contains('PHONE') ||
          label.contains('EMAIL') ||
          label.contains('WHATSAPP')) {
        continue;
      }
      if (label.contains('PIN')) {
        pincode = field.value.trim();
        continue;
      }
      parts.add(_niceCase(field.value).replaceAll(RegExp(r'\s*,\s*'), ', '));
    }
    final country = parts.length > 1 && parts.last.toUpperCase() == 'INDIA'
        ? parts.removeLast()
        : null;
    if (pincode != null && parts.isNotEmpty) {
      parts.last = '${parts.last} $pincode';
    }
    return [...parts, ?country].join(', ');
  }
}

/// A parent or guardian: name and occupation, with call and email.
class _ParentCard extends StatelessWidget {
  const _ParentCard({required this.group});

  final ProfileGroup group;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final groups = [group];
    final relation = _niceCase(
      group.title.replaceAll(RegExp(r'\s*DETAILS$'), ''),
    );
    final name = group.fields
        .where((f) => f.label.toUpperCase().endsWith('NAME') && _shown(f.value))
        .map((f) => f.value)
        .firstOrNull;
    final occupation = _value(groups, 'OCCUPATION');
    final mobile = _value(groups, 'MOBILE NUMBER');
    final email = _value(groups, 'EMAIL');
    final shown = {name, occupation, mobile, email};
    final rest = [
      for (final f in group.fields)
        if (!shown.contains(f.value)) f,
    ];
    return Surface(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Container(
                width: 44,
                height: 44,
                alignment: Alignment.center,
                decoration: BoxDecoration(
                  color: colors.app.warning.subtle,
                  shape: BoxShape.circle,
                ),
                child: Text(
                  _initials(_niceCase(name ?? relation)),
                  style: typography.body.sm.copyWith(
                    fontWeight: FontWeight.w700,
                    color: colors.app.warning.onSubtle,
                  ),
                ),
              ),
              const SizedBox(width: Space.md),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      name == null ? relation : _niceCase(name),
                      style: typography.body.md.copyWith(
                        fontWeight: FontWeight.w700,
                        color: colors.foreground,
                      ),
                    ),
                    Text(
                      [
                        relation,
                        if (occupation != null) _niceCase(occupation),
                      ].join(' · '),
                      style: typography.body.xs.copyWith(
                        color: colors.mutedForeground,
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
          if (mobile != null || email != null) ...[
            const SizedBox(height: Space.sm),
            if (mobile != null) _ContactRow.of('Mobile', mobile),
            if (email != null) _ContactRow.of('Email', email),
          ],
          if (rest.any((f) => _shown(f.value)))
            _Expandable(
              label: 'More',
              child: _Fields(
                groups: [ProfileGroup(title: '', fields: rest)],
              ),
            ),
        ],
      ),
    );
  }
}

/// A section that starts closed, with how many fields it holds.
class _Collapsible extends StatelessWidget {
  const _Collapsible({required this.title, required this.groups});

  final String title;
  final List<ProfileGroup> groups;

  @override
  Widget build(BuildContext context) {
    final count = groups.fold(
      0,
      (n, g) => n + g.fields.where((f) => _shown(f.value)).length,
    );
    return Surface(
      padding: const EdgeInsets.symmetric(horizontal: Space.lg),
      child: _Expandable(
        label: title,
        count: count,
        header: true,
        child: Padding(
          padding: const EdgeInsets.only(bottom: Space.sm),
          child: _Fields(groups: groups),
        ),
      ),
    );
  }
}

/// A tappable row that shows or hides [child].
class _Expandable extends HookWidget {
  const _Expandable({
    required this.label,
    required this.child,
    this.count,
    this.header = false,
  });

  final String label;
  final Widget child;
  final int? count;

  /// A section title, rather than a small "More" link inside a card.
  final bool header;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final open = useState(false);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        FTappable(
          semanticsLabel: label,
          onPress: () {
            HapticFeedback.selectionClick();
            open.value = !open.value;
          },
          child: Padding(
            padding: EdgeInsets.symmetric(vertical: header ? Space.md : 6),
            child: Row(
              children: [
                Expanded(
                  child: Text(
                    label,
                    style: header
                        ? typography.body.md.copyWith(
                            fontWeight: FontWeight.w600,
                            color: colors.foreground,
                          )
                        : typography.body.xs.copyWith(
                            fontWeight: FontWeight.w600,
                            color: colors.app.accentTone.base,
                          ),
                  ),
                ),
                if (count != null && !open.value)
                  Text(
                    '$count',
                    style: typography.body.xs.copyWith(
                      color: colors.mutedForeground,
                    ),
                  ),
                const SizedBox(width: Space.sm),
                AnimatedRotation(
                  turns: open.value ? 0.5 : 0,
                  duration: Motion.fast,
                  child: Icon(
                    FLucideIcons.chevronDown,
                    size: 16,
                    color: colors.mutedForeground,
                  ),
                ),
              ],
            ),
          ),
        ),
        AnimatedSize(
          duration: Motion.medium,
          curve: Curves.easeOutCubic,
          alignment: Alignment.topCenter,
          child: open.value ? child : const SizedBox(width: double.infinity),
        ),
      ],
    );
  }
}

/// Label/value rows by group; empty values are left out.
class _Fields extends StatelessWidget {
  const _Fields({required this.groups});

  final List<ProfileGroup> groups;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final shown = [
      for (final group in groups)
        if (group.fields.any((f) => _shown(f.value))) group,
    ];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (final (i, group) in shown.indexed) ...[
          if (group.title.isNotEmpty)
            Padding(
              padding: EdgeInsets.only(top: i == 0 ? 0 : Space.md, bottom: 2),
              child: Text(
                group.title.toUpperCase(),
                style: typography.body.xs.copyWith(
                  fontWeight: FontWeight.w700,
                  letterSpacing: 0.6,
                  color: colors.app.accentTone.base,
                ),
              ),
            ),
          for (final field in group.fields)
            if (_shown(field.value)) _FieldRow(field: field),
        ],
      ],
    );
  }
}

/// A label and its value side by side, or the value under the label when it
/// is too long to share the line.
class _FieldRow extends HookWidget {
  const _FieldRow({required this.field});

  final ProfileField field;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final secret = _isSecret(field.label);
    final revealed = useState(!secret);
    final value = revealed.value
        ? _niceCase(field.value)
        : _masked(field.value);
    final labelText = Text(
      _label(field.label),
      style: typography.body.sm.copyWith(color: colors.mutedForeground),
    );
    final valueText = Text.rich(
      TextSpan(
        text: value,
        children: [
          if (secret)
            WidgetSpan(
              alignment: PlaceholderAlignment.middle,
              child: Padding(
                padding: const EdgeInsets.only(left: 6),
                child: Icon(
                  revealed.value ? FLucideIcons.eyeOff : FLucideIcons.eye,
                  size: 14,
                  color: colors.mutedForeground,
                ),
              ),
            ),
        ],
      ),
      style: typography.body.sm.copyWith(
        color: colors.foreground,
        fontWeight: FontWeight.w500,
      ),
    );
    final stacked = value.length > 22;

    return _Copyable(
      value: field.value,
      onTap: secret ? () => revealed.value = !revealed.value : null,
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 7),
        child: stacked
            ? Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [labelText, const SizedBox(height: 2), valueText],
              )
            : Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Expanded(child: labelText),
                  const SizedBox(width: Space.md),
                  valueText,
                ],
              ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Mentor tab
// ---------------------------------------------------------------------------

class _MentorTab extends ConsumerWidget {
  const _MentorTab({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    return _Loaded(
      value: ref.watch(mentorProvider),
      what: 'your mentor',
      onRetry: () => ref.read(mentorProvider.notifier).refresh(),
      builder: (mentor) {
        String? detail(String label) {
          for (final field in mentor.details) {
            if (field.label.toLowerCase().contains(label) &&
                _shown(field.value)) {
              return field.value;
            }
          }
          return null;
        }

        final name = detail('name');
        final email = detail('email');
        final mobile = detail('mobile');
        final cabin = detail('cabin');
        final department = detail('department');
        final headline = {
          'name',
          'designation',
          'email',
          'mobile',
          'cabin',
          'department',
        };
        final rest = [
          for (final field in mentor.details)
            if (!headline.any(field.label.toLowerCase().contains)) field,
        ];
        final messages = [
          SectionHeader(
            title: 'Messages',
            trailing: mentor.messages.isEmpty
                ? null
                : Text('${mentor.messages.length}'),
          ),
          if (mentor.messages.isEmpty)
            Surface(
              child: Row(
                children: [
                  Icon(
                    FLucideIcons.inbox,
                    size: 18,
                    color: colors.mutedForeground,
                  ),
                  const SizedBox(width: Space.md),
                  Expanded(
                    child: Text(
                      'No messages from your mentor. '
                      'VTOP removes them after a week.',
                      style: typography.body.sm.copyWith(
                        color: colors.mutedForeground,
                      ),
                    ),
                  ),
                ],
              ),
            )
          else
            for (final message in mentor.messages) ...[
              _MessageCard(message: message),
              const SizedBox(height: Space.sm),
            ],
        ];

        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // New messages matter more than details that rarely change.
            if (mentor.messages.isNotEmpty) ...messages,
            if (name == null)
              const EmptyState(
                icon: FLucideIcons.userX,
                title: 'No mentor assigned',
              )
            else ...[
              if (mentor.messages.isNotEmpty)
                const SectionHeader(title: 'Mentor'),
              Surface(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Row(
                      children: [
                        Container(
                          width: 52,
                          height: 52,
                          alignment: Alignment.center,
                          decoration: BoxDecoration(
                            color: colors.app.lab.subtle,
                            shape: BoxShape.circle,
                          ),
                          child: Text(
                            _initials(name),
                            style: typography.body.md.copyWith(
                              fontWeight: FontWeight.w700,
                              color: colors.app.lab.onSubtle,
                            ),
                          ),
                        ),
                        const SizedBox(width: Space.md),
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                name,
                                style: typography.body.md.copyWith(
                                  fontWeight: FontWeight.w700,
                                  color: colors.foreground,
                                ),
                              ),
                              if (detail('designation') case final d?)
                                Text(
                                  d,
                                  style: typography.body.xs.copyWith(
                                    color: colors.mutedForeground,
                                  ),
                                ),
                              if (department != null)
                                Text(
                                  department,
                                  style: typography.body.xs.copyWith(
                                    color: colors.mutedForeground,
                                  ),
                                ),
                            ],
                          ),
                        ),
                      ],
                    ),
                    if (cabin != null) ...[
                      const SizedBox(height: Space.md),
                      _Copyable(
                        value: cabin,
                        child: Row(
                          children: [
                            Icon(
                              FLucideIcons.mapPin,
                              size: 16,
                              color: colors.app.accentTone.base,
                            ),
                            const SizedBox(width: Space.sm + 2),
                            Text(
                              'Cabin $cabin',
                              style: typography.body.sm.copyWith(
                                color: colors.foreground,
                                fontWeight: FontWeight.w500,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ],
                    if (email != null || mobile != null) ...[
                      const SizedBox(height: Space.xs),
                      if (mobile != null) _ContactRow.of('Mobile', mobile),
                      if (email != null) _ContactRow.of('Email', email),
                    ],
                    if (rest.any((f) => _shown(f.value)))
                      _Expandable(
                        label: 'More',
                        child: _Fields(
                          groups: [ProfileGroup(title: '', fields: rest)],
                        ),
                      ),
                  ],
                ),
              ),
            ],
            if (mentor.messages.isEmpty) ...messages,
            DataUpdatedFooter(updateTime: mentor.updateTime.toInt()),
          ],
        );
      },
    );
  }
}

/// A message's text, with its other columns (date, sender) underneath.
/// VTOP's columns are not fixed, so the longest cell is taken as the text
/// unless one is headed "message".
class _MessageCard extends StatelessWidget {
  const _MessageCard({required this.message});

  final MentorMessage message;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final fields = [
      for (final field in message.fields)
        if (_shown(field.value) && !_isSerial(field.label)) field,
    ];
    if (fields.isEmpty) return const SizedBox.shrink();
    final body = fields.firstWhere(
      (f) => f.label.toLowerCase().contains('message'),
      orElse: () =>
          fields.reduce((a, b) => b.value.length > a.value.length ? b : a),
    );
    final meta = [
      for (final f in fields)
        if (f != body) f,
    ];
    return Surface(
      child: _Copyable(
        value: body.value,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              body.value,
              style: typography.body.sm.copyWith(color: colors.foreground),
            ),
            if (meta.isNotEmpty) ...[
              const SizedBox(height: Space.sm),
              Wrap(
                spacing: Space.sm,
                runSpacing: 4,
                children: [
                  for (final f in meta)
                    Text(
                      f.label.isEmpty ? f.value : '${f.label}: ${f.value}',
                      style: typography.body.xs.copyWith(
                        color: colors.mutedForeground,
                      ),
                    ),
                ],
              ),
            ],
          ],
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Shared pieces
// ---------------------------------------------------------------------------

/// A phone number or email with what it is; a tap calls, mails or opens
/// WhatsApp, a long-press copies it.
class _ContactRow extends StatelessWidget {
  const _ContactRow({
    required this.icon,
    required this.label,
    required this.value,
    required this.uri,
  });

  factory _ContactRow.of(String label, String value) {
    final digits = value.replaceAll(RegExp(r'\D'), '');
    if (value.contains('@')) {
      return _ContactRow(
        icon: FLucideIcons.mail,
        label: label,
        value: value,
        uri: Uri(scheme: 'mailto', path: value),
      );
    }
    if (label.toLowerCase().contains('whatsapp')) {
      return _ContactRow(
        icon: FLucideIcons.messageCircle,
        label: label,
        value: value,
        // wa.me takes the number with its country code.
        uri: Uri.https('wa.me', digits.length == 10 ? '91$digits' : digits),
      );
    }
    return _ContactRow(
      icon: FLucideIcons.phone,
      label: label,
      value: value,
      uri: Uri(scheme: 'tel', path: digits),
    );
  }

  final IconData icon;
  final String label;
  final String value;
  final Uri uri;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    return _Copyable(
      value: value,
      onTap: () => launchUrl(uri, mode: LaunchMode.externalApplication),
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 7),
        child: Row(
          children: [
            Icon(icon, size: 16, color: colors.app.accentTone.base),
            const SizedBox(width: Space.sm + 2),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    value,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: typography.body.sm.copyWith(
                      color: colors.foreground,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                  Text(
                    label,
                    style: typography.body.xs.copyWith(
                      color: colors.mutedForeground,
                    ),
                  ),
                ],
              ),
            ),
            Icon(
              FLucideIcons.chevronRight,
              size: 16,
              color: colors.mutedForeground,
            ),
          ],
        ),
      ),
    );
  }
}

/// Long-press copies [value]; [onTap], if given, runs on a tap.
class _Copyable extends StatelessWidget {
  const _Copyable({required this.value, required this.child, this.onTap});

  final String value;
  final Widget child;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: onTap,
      onLongPress: () async {
        HapticFeedback.mediumImpact();
        await Clipboard.setData(ClipboardData(text: value));
        if (context.mounted) dispToast(context, 'Copied', value);
      },
      child: child,
    );
  }
}

Uint8List? _decodePhoto(String photo) {
  if (photo.isEmpty) return null;
  try {
    return base64.decode(base64.normalize(photo));
  } on FormatException {
    return null;
  }
}

/// VTOP shows "-", "NA" or nothing for fields that do not apply.
bool _shown(String value) {
  final v = value.trim().toUpperCase();
  return v.isNotEmpty && v != '-' && v != 'NA' && v != 'N/A';
}

/// The first shown value labelled [label] (any case) in [groups].
String? _value(List<ProfileGroup> groups, String label) {
  for (final group in groups) {
    for (final field in group.fields) {
      if (field.label.toUpperCase() == label && _shown(field.value)) {
        return field.value.trim();
      }
    }
  }
  return null;
}

List<ProfileField> _fields(List<ProfileGroup> groups) => [
  for (final group in groups) ...group.fields,
];

bool _isSecret(String label) =>
    label.toUpperCase().contains('AADHAR') ||
    label.toUpperCase().contains('AADHAAR');

bool _isSerial(String label) {
  final l = label.toLowerCase().replaceAll(RegExp(r'[^a-z]'), '');
  return l == 'slno' || l == 'sno' || l == 'no';
}

/// Everything but the last four characters hidden.
String _masked(String value) => value.length <= 4
    ? value
    : '${'•' * (value.length - 4)}${value.substring(value.length - 4)}';

/// "PERSONAL INFORMATION" → "Personal details".
String _detailsTitle(String title) {
  final name = _label(
    title.replaceAll(RegExp(r'\s*INFORMATION\s*$', caseSensitive: false), ''),
  );
  return name == 'Educational' ? 'Education' : '$name details';
}

/// "4- BED -AC" → "4 Bed AC".
String _bedType(String value) =>
    _niceCase(value.replaceAll(RegExp(r'\s*-\s*'), ' ').trim());

/// VTOP's upper-case labels in sentence case, keeping the abbreviations:
/// "EMP.ID(IF EMPLOYED IN VIT)" → "Emp. ID (if employed in VIT)".
String _label(String label) {
  final words = label
      .trim()
      .toLowerCase()
      .replaceAll('no.of.', '')
      .replaceAll('emp.id(', 'emp. id (')
      .split(RegExp(r'\s+'))
      .map(
        (w) => switch (w) {
          'vit' || 'vit)' || 'id' => w.toUpperCase(),
          _ => w,
        },
      )
      .join(' ');
  if (words.isEmpty) return words;
  return '${words[0].toUpperCase()}${words.substring(1)}';
}

/// Words kept lower case inside a value: "Board of Intermediate".
const _minorWords = {'and', 'of', 'the', 'for', 'to', 'in', 'at', 'on', 'by'};

/// Short words that are words, not abbreviations, so they are not kept
/// upper case.
const _shortWords = {
  'YES',
  'NO',
  'MR',
  'MRS',
  'DR',
  'ST',
  'RD',
  'OPP',
  'BED',
  ..._minor,
};
const _minor = {'AND', 'OF', 'THE', 'FOR', 'TO', 'IN', 'AT', 'ON', 'BY'};

/// An all-caps (or all-lowercase) value in title case: "ANDHRA PRADESH" →
/// "Andhra Pradesh". Emails and mixed-case values are left as they are;
/// short abbreviations like OC, MPC or B+ stay upper case.
String _niceCase(String value) {
  final v = value.trim().replaceAllMapped(RegExp(r',(?=\S)'), (_) => ', ');
  if (v.contains('@')) return v;
  if (v != v.toUpperCase() && v != v.toLowerCase()) return v;
  final allCaps = v == v.toUpperCase();
  return [
    for (final (i, w) in v.split(' ').indexed)
      _word(w, first: i == 0, allCaps: allCaps),
  ].join(' ');
}

String _word(String w, {required bool first, required bool allCaps}) {
  final upper = w.toUpperCase();
  final letters = upper.replaceAll(RegExp(r'[^A-Z]'), '');
  final lower = w.toLowerCase();
  if (!first && _minorWords.contains(lower)) return lower;
  if (allCaps &&
      letters.isNotEmpty &&
      letters.length <= 3 &&
      !_shortWords.contains(letters) &&
      !RegExp(r'\d').hasMatch(w)) {
    return upper;
  }
  final i = lower.indexOf(RegExp(r'[a-z]'));
  if (i < 0 || RegExp(r'\d').hasMatch(lower.substring(0, i))) return lower;
  return '${lower.substring(0, i)}${lower[i].toUpperCase()}'
      '${lower.substring(i + 1)}';
}

String _initials(String name) => name
    .split(RegExp(r'\s+'))
    .where((p) => p.isNotEmpty)
    .take(2)
    .map((p) => p[0].toUpperCase())
    .join();
