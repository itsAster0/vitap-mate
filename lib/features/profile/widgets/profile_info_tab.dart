import 'dart:convert';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/ui/data_updated_footer.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/profile/profile.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/features/profile/logic/profile_layout.dart';
import 'package:vitapmate/features/profile/logic/profile_text.dart';
import 'package:vitapmate/features/profile/widgets/profile_cards.dart';
import 'package:vitapmate/features/profile/widgets/profile_fields.dart';
import 'package:vitapmate/features/profile/widgets/profile_parts.dart';

/// The profile tab: header, hostel, contacts, addresses, parents and details.
class ProfileInfoTab extends ConsumerWidget {
  const ProfileInfoTab({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) => Loaded(
    value: ref.watch(profileProvider),
    what: 'your profile',
    onRetry: () => ref.read(profileProvider.notifier).refresh(),
    builder: (profile) {
      final layout = ProfileLayout(profile);
      return Column(
        crossAxisAlignment: .stretch,
        children: [
          _ProfileHero(profile: profile),
          if (fieldValue(layout.hostel, 'ROOM NO.') != null) ...[
            const SizedBox(height: Space.md),
            HostelCard(groups: layout.hostel),
          ],
          if (layout.contacts.isNotEmpty) ...[
            const SectionHeader(title: 'Contact'),
            Surface(
              padding: const .symmetric(
                horizontal: Space.lg,
                vertical: Space.sm,
              ),
              child: Column(
                children: [
                  for (final (label, value) in layout.contacts)
                    ContactRow(label, value),
                ],
              ),
            ),
          ],
          if (layout.addresses.isNotEmpty) ...[
            const SectionHeader(title: 'Addresses'),
            AddressCard(groups: layout.addresses),
          ],
          if (layout.parents.isNotEmpty) ...[
            const SectionHeader(title: 'Parents'),
            for (final group in layout.parents) ...[
              ParentCard(group: group),
              const SizedBox(height: Space.sm),
            ],
          ],
          if (layout.details.isNotEmpty) ...[
            const SectionHeader(title: 'More details'),
            for (final (title, groups) in layout.details) ...[
              // Closed at first, with how many fields it holds.
              Surface(
                padding: const .symmetric(horizontal: Space.lg),
                child: Expandable(
                  label: detailsTitle(title),
                  count: groups
                      .expand((g) => g.fields)
                      .where((f) => hasValue(f.value))
                      .length,
                  header: true,
                  child: Padding(
                    padding: const .only(bottom: Space.sm),
                    child: Fields(groups: groups),
                  ),
                ),
              ),
              const SizedBox(height: Space.sm),
            ],
          ],
          DataUpdatedFooter(updateTime: profile.updateTime.toInt()),
        ],
      );
    },
  );
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
    final name = niceCase(profile.name);

    return Surface(
      child: Row(
        children: [
          ClipRRect(
            borderRadius: .circular(Radii.md),
            child: Container(
              width: 76,
              height: 92,
              color: colors.app.accentTone.subtle,
              alignment: .center,
              child: photo == null
                  ? Text(
                      initials(name),
                      style: typography.body.xl.copyWith(
                        fontWeight: .w700,
                        color: colors.app.accentTone.onSubtle,
                      ),
                    )
                  : Image.memory(
                      photo,
                      width: 76,
                      height: 92,
                      fit: .cover,
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
              crossAxisAlignment: .start,
              children: [
                Text(
                  name,
                  style: typography.body.lg.copyWith(
                    fontWeight: .w700,
                    color: colors.foreground,
                    height: 1.2,
                  ),
                ),
                const SizedBox(height: 6),
                if (hasValue(profile.registrationNumber))
                  Copyable(
                    value: profile.registrationNumber,
                    child: ToneBadge(
                      label: profile.registrationNumber,
                      tone: colors.app.accentTone,
                    ),
                  ),
                const SizedBox(height: Space.sm),
                if (hasValue(profile.program))
                  Text(
                    profile.program,
                    style: typography.body.sm.copyWith(
                      color: colors.foreground,
                      fontWeight: .w500,
                    ),
                  ),
                if (hasValue(profile.school)) Small(profile.school),
              ],
            ),
          ),
        ],
      ),
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
