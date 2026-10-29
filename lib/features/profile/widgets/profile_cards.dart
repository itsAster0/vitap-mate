import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/features/profile/logic/profile_text.dart';
import 'package:vitapmate/features/profile/widgets/profile_fields.dart';
import 'package:vitapmate/features/profile/widgets/profile_parts.dart';

/// Block and room up front, bed and mess underneath.
class HostelCard extends StatelessWidget {
  const HostelCard({super.key, required this.groups});

  final List<ProfileGroup> groups;

  @override
  Widget build(BuildContext context) {
    // "MH-2 (MH-2) (MH-2 - Block )" → "MH-2".
    final block = fieldValue(groups, 'BLOCK NAME')?.split('(').first.trim();
    final room = fieldValue(groups, 'ROOM NO.');
    final bed = fieldValue(groups, 'BED TYPE');
    // "Non-Veg Mess Fee - " → "Non-Veg Mess".
    final mess = fieldValue(
      groups,
      'MESS INFORMATION',
    )?.replaceAll(RegExp(r'\s*Fee[\s-]*$', caseSensitive: false), '');
    final shownMess = mess != null && hasValue(mess) ? mess : null;
    return Surface(
      child: Headline(
        tone: context.theme.colors.app.lab,
        icon: FLucideIcons.bedDouble,
        title: [?block, if (room != null) 'Room $room'].join(' · '),
        lines: [
          if (bed != null || shownMess != null)
            [if (bed != null) bedType(bed), ?shownMess].join(' · '),
        ],
      ),
    );
  }
}

/// Each address as one block of text.
class AddressCard extends StatelessWidget {
  const AddressCard({super.key, required this.groups});

  final List<ProfileGroup> groups;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final addresses = [
      for (final group in groups)
        if (_address(group) case final text when text.isNotEmpty)
          (niceCase(group.title.replaceAll(RegExp(r'\s*ADDRESS$'), '')), text),
    ];
    return Surface(
      padding: const .symmetric(horizontal: Space.lg, vertical: Space.sm),
      child: Column(
        crossAxisAlignment: .stretch,
        children: [
          for (final (i, (title, text)) in addresses.indexed) ...[
            if (i > 0) Container(height: 1, color: colors.border),
            Copyable(
              value: text,
              child: Padding(
                padding: const .symmetric(vertical: Space.sm + 2),
                child: Row(
                  crossAxisAlignment: .start,
                  children: [
                    Padding(
                      padding: const .only(top: 2),
                      child: Icon(
                        i == 0 ? FLucideIcons.mapPin : FLucideIcons.house,
                        size: 16,
                        color: colors.app.accentTone.base,
                      ),
                    ),
                    const SizedBox(width: Space.sm + 2),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: .start,
                        children: [
                          Small(title),
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
      if (!hasValue(field.value) ||
          label.contains('PHONE') ||
          label.contains('EMAIL') ||
          label.contains('WHATSAPP')) {
        continue;
      }
      if (label.contains('PIN')) {
        pincode = field.value.trim();
        continue;
      }
      parts.add(niceCase(field.value).replaceAll(RegExp(r'\s*,\s*'), ', '));
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
class ParentCard extends StatelessWidget {
  const ParentCard({super.key, required this.group});

  final ProfileGroup group;

  @override
  Widget build(BuildContext context) {
    final groups = [group];
    final relation = niceCase(
      group.title.replaceAll(RegExp(r'\s*DETAILS$'), ''),
    );
    final name = group.fields
        .where(
          (f) => f.label.toUpperCase().endsWith('NAME') && hasValue(f.value),
        )
        .map((f) => f.value)
        .firstOrNull;
    final occupation = fieldValue(groups, 'OCCUPATION');
    final mobile = fieldValue(groups, 'MOBILE NUMBER');
    final email = fieldValue(groups, 'EMAIL');
    final used = {name, occupation, mobile, email};
    final rest = [
      for (final f in group.fields)
        if (!used.contains(f.value)) f,
    ];
    return Surface(
      child: Column(
        crossAxisAlignment: .stretch,
        children: [
          Headline(
            tone: context.theme.colors.app.warning,
            initials: initials(niceCase(name ?? relation)),
            title: name == null ? relation : niceCase(name),
            lines: [
              [
                relation,
                if (occupation != null) niceCase(occupation),
              ].join(' · '),
            ],
          ),
          ...contactsAndMore(Space.sm, mobile, email, rest),
        ],
      ),
    );
  }
}
