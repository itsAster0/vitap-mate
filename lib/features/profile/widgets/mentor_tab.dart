import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/ui/data_updated_footer.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/profile/profile.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/features/profile/logic/profile_text.dart';
import 'package:vitapmate/features/profile/widgets/profile_fields.dart';
import 'package:vitapmate/features/profile/widgets/profile_parts.dart';

/// The mentor tab: the mentor's details and messages.
class MentorTab extends ConsumerWidget {
  const MentorTab({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    return Loaded(
      value: ref.watch(mentorProvider),
      what: 'your mentor',
      onRetry: () => ref.read(mentorProvider.notifier).refresh(),
      builder: (mentor) {
        String? detail(String label) => mentor.details
            .where((f) => f.label.toLowerCase().contains(label))
            .where((f) => hasValue(f.value))
            .firstOrNull
            ?.value;

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
          crossAxisAlignment: .stretch,
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
                  crossAxisAlignment: .stretch,
                  children: [
                    Headline(
                      tone: colors.app.lab,
                      initials: initials(name),
                      large: true,
                      title: name,
                      lines: [?detail('designation'), ?department],
                    ),
                    if (cabin != null) ...[
                      const SizedBox(height: Space.md),
                      Copyable(
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
                                fontWeight: .w500,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ],
                    ...contactsAndMore(Space.xs, mobile, email, rest),
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
        if (hasValue(field.value) && !isSerial(field.label)) field,
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
      child: Copyable(
        value: body.value,
        child: Column(
          crossAxisAlignment: .stretch,
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
                    Small(f.label.isEmpty ? f.value : '${f.label}: ${f.value}'),
                ],
              ),
            ],
          ],
        ),
      ),
    );
  }
}
