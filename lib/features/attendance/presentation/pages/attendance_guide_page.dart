import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';

/// Opens [AttendanceGuidePage] inside the current tab, so the timetable and
/// attendance tabs each keep their own back stack.
void openAttendanceGuide(BuildContext context) {
  final inTimetable = GoRouterState.of(
    context,
  ).matchedLocation.startsWith('/timetable');
  GoRouter.of(context).pushNamed(
    inTimetable ? Paths.timetableAttendanceGuide : Paths.attendanceGuide,
  );
}

/// Explains every attendance number in the app against one worked example,
/// and that they are estimates to check against VTOP.
class AttendanceGuidePage extends HookWidget {
  const AttendanceGuidePage({super.key, this.showTimetable = false});

  /// Opens scrolled to the "In the timetable" section.
  final bool showTimetable;

  @override
  Widget build(BuildContext context) {
    final timetableKey = useMemoized(GlobalKey.new);
    useEffect(() {
      if (!showTimetable) return null;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        final target = timetableKey.currentContext;
        if (target == null || !target.mounted) return;
        Scrollable.ensureVisible(
          target,
          duration: MediaQuery.disableAnimationsOf(context)
              ? Duration.zero
              : Motion.medium,
          curve: Curves.easeOutCubic,
        );
      });
      return null;
    }, const []);
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final palette = colors.app;
    // Built in full (it is short) so the timetable section can be scrolled
    // to before it is on screen.
    return SingleChildScrollView(
      padding: EdgeInsets.fromLTRB(
        Space.sm,
        Space.md,
        Space.sm,
        Space.xl + MediaQuery.paddingOf(context).bottom,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 2),
            child: Text(
              'What each number means, where it comes from, and when to '
              'double-check.',
              style: typography.body.sm.copyWith(
                height: 1.45,
                color: colors.mutedForeground,
              ),
            ),
          ),
          const SizedBox(height: Space.lg),
          const _BestEffortNote(),

          const SectionHeader(title: 'On each course'),
          const _SampleCard(),
          const SizedBox(height: Space.sm + 2),
          const Surface(
            padding: EdgeInsets.symmetric(horizontal: Space.lg),
            child: Column(
              children: [
                _Term(
                  mark: '1',
                  title: '92%',
                  body:
                      'VTOP\'s own percentage. VTOP rounds up, so 74.5% shows '
                      'as 75%. The app judges 75% on the exact figure, so a '
                      'course showing 75% can still say "Attend 1".',
                ),
                _Term(
                  mark: '2',
                  title: '22/24 attended',
                  body:
                      'Classes VTOP has posted: attended of held. On duty '
                      'counts as attended. An amber dot beside it means the '
                      'class-by-class history doesn\'t match yet; pull to '
                      'refresh.',
                ),
                _Term(
                  mark: '3',
                  title: 'Can skip 5',
                  body:
                      'Classes you can miss in a row and stay at 75% or above.',
                  math: '22 ÷ 29 = 75.9%   ·   22 ÷ 30 = 73.3%',
                  variants:
                      'Below 75% it reads "Attend 2": classes in a row to get '
                      'back. "Don\'t skip" means the next miss drops you below.',
                ),
                _Term(
                  mark: '4',
                  title: '1+',
                  body:
                      'Classes already held that VTOP hasn\'t posted yet. The '
                      'app can\'t know if you went, so they still count as open.',
                ),
                _Term(
                  mark: '5',
                  title: '16 left',
                  body:
                      'Classes still to come, from your timetable and the '
                      'academic calendar. Holidays, exam days and Sundays are '
                      'skipped; today\'s classes count until they start.',
                ),
                _Term(
                  mark: '6',
                  title: '8 to spare',
                  body:
                      'Of all 17 open classes, how many you can miss and still '
                      'finish at 75% or above.',
                  math: '(22 + 9) ÷ (24 + 17) = 75.6%',
                  variants:
                      'Below 75% it reads "3 needed": classes to attend to '
                      'finish at 75%. "Short of 75%" means even all of them '
                      'won\'t get you there.',
                ),
                _Term(
                  mark: '7',
                  title: '95% max',
                  body: 'Where you finish if you attend every open class.',
                  math: '(22 + 17) ÷ (24 + 17) = 95.1%',
                  last: true,
                ),
              ],
            ),
          ),

          SectionHeader(key: timetableKey, title: 'In the timetable'),
          Surface(
            padding: const EdgeInsets.symmetric(horizontal: Space.lg),
            child: Column(
              children: [
                _Term(
                  lead: Icon(
                    FLucideIcons.percent,
                    size: 16,
                    color: colors.mutedForeground,
                  ),
                  title: 'Beside each class',
                  body:
                      'The same percentage and skip count as the course card. '
                      '"need 2" is "Attend 2"; tap it for the full details.',
                  // Full size below the text, so it never squeezes the title.
                  footer: Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: Space.sm + 2,
                      vertical: Space.xs + 2,
                    ),
                    decoration: BoxDecoration(
                      color: colors.secondary,
                      borderRadius: BorderRadius.circular(Radii.sm),
                    ),
                    child: const _SampleGutter(),
                  ),
                ),
                _Term(
                  lead: Icon(
                    FLucideIcons.calendarDays,
                    size: 16,
                    color: colors.mutedForeground,
                  ),
                  title: 'Week recap',
                  body:
                      'On a day without classes, one dot per class of the '
                      'week, from the class-by-class history.',
                  footer: Wrap(
                    spacing: Space.md,
                    runSpacing: Space.xs + 2,
                    children: [
                      _DotKey(fill: palette.success.base, label: 'Present'),
                      _DotKey(fill: palette.accent, label: 'On duty'),
                      _DotKey(fill: palette.danger.base, label: 'Absent'),
                      _DotKey(
                        ring: colors.mutedForeground,
                        label: 'Not posted',
                      ),
                      _DotKey(fill: colors.border, label: 'To come'),
                    ],
                  ),
                ),
                _Term(
                  lead: Icon(
                    FLucideIcons.pencilLine,
                    size: 16,
                    color: palette.warning.base,
                  ),
                  title: 'Exams and holidays',
                  body:
                      'Exam times and venues come straight from VTOP\'s exam '
                      'schedule. Holidays and exam days come from the academic '
                      'calendar, so a change announced later shows only once '
                      'VTOP has it.',
                  last: true,
                ),
              ],
            ),
          ),

          const SectionHeader(title: 'Good to know'),
          Surface(
            padding: const EdgeInsets.symmetric(horizontal: Space.lg),
            child: Column(
              children: [
                _Term(
                  lead: Icon(
                    FLucideIcons.flaskConical,
                    size: 16,
                    color: palette.lab.base,
                  ),
                  title: 'Labs',
                  body:
                      'VTOP counts each two-period session as 2 classes. The '
                      'percentage, counts and "Can skip" on the card use '
                      'VTOP\'s classes; "left" and "to spare" are whole '
                      'sessions. Labs stop at the lab FAT.',
                ),
                _Term(
                  lead: Icon(
                    FLucideIcons.flag,
                    size: 16,
                    color: colors.mutedForeground,
                  ),
                  title: 'Counting to the next exam',
                  body:
                      'Settings → Classes Left can count to the next CAT or FAT '
                      'instead of the semester end. The line then reads "left '
                      'till CAT-II".',
                ),
                _Term(
                  lead: Icon(
                    FLucideIcons.slidersHorizontal,
                    size: 16,
                    color: colors.mutedForeground,
                  ),
                  title: 'Planner',
                  body:
                      'Open a course to try a plan: attend or skip a few '
                      'classes and see where you land. You can correct the '
                      'counts there if you know VTOP is behind.',
                  last: true,
                ),
              ],
            ),
          ),

          const SectionHeader(title: 'Where it can be off'),
          Surface(
            padding: const EdgeInsets.fromLTRB(
              Space.lg,
              Space.md,
              Space.lg,
              Space.md,
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                for (final item in const [
                  'Data on your phone older than VTOP: check "Updated" at '
                      'the bottom and pull to refresh',
                  'Holidays announced after the academic calendar',
                  'Cancelled, extra or rescheduled classes',
                  'A day that follows another day\'s timetable',
                  'Attendance posted late or corrected by faculty',
                  'On duty or medical leave approved afterwards',
                  'Timetable changes not refreshed yet',
                ])
                  _Bullet(text: item, color: palette.warning.base),
              ],
            ),
          ),
          const SizedBox(height: Space.lg),
          Text(
            'VTOP and your faculty have the final word.',
            textAlign: TextAlign.center,
            style: typography.body.xs.copyWith(color: colors.mutedForeground),
          ),
        ],
      ),
    );
  }
}

/// The warning up top: these are estimates, check VTOP before skipping.
class _BestEffortNote extends StatelessWidget {
  const _BestEffortNote();

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final tone = colors.app.warning;
    return Surface(
      color: tone.subtle,
      borderColor: tone.base.withValues(alpha: 0.35),
      padding: const EdgeInsets.all(Space.md + 2),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.only(top: 2),
            child: Icon(FLucideIcons.triangleAlert, size: 16, color: tone.base),
          ),
          const SizedBox(width: Space.sm + 2),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'Best effort, not official',
                  style: typography.body.sm.copyWith(
                    fontWeight: FontWeight.w600,
                    color: tone.onSubtle,
                  ),
                ),
                const SizedBox(height: 3),
                Text(
                  'These are worked out from VTOP\'s data and can miss edge '
                  'cases. Treat them as a guide and check VTOP before you '
                  'skip a class.',
                  style: typography.body.sm.copyWith(
                    height: 1.45,
                    color: colors.foreground,
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

/// An attendance card for the example course, each number tagged with the
/// marker of the term that explains it.
class _SampleCard extends StatelessWidget {
  const _SampleCard();

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final tone = colors.app.success;
    final muted = typography.body.sm.copyWith(
      color: colors.mutedForeground,
      fontFeatures: const [FontFeature.tabularFigures()],
    );
    final small = typography.body.xs.copyWith(
      color: colors.mutedForeground,
      fontFeatures: const [FontFeature.tabularFigures()],
    );
    final strong = small.copyWith(
      fontWeight: FontWeight.w600,
      color: colors.foreground,
    );
    InlineSpan mark(String n) => WidgetSpan(
      alignment: PlaceholderAlignment.middle,
      child: Padding(
        padding: const EdgeInsets.only(left: 3, right: 1),
        child: _Marker(n, size: 14),
      ),
    );

    return Surface(
      padding: EdgeInsets.zero,
      child: ClipRRect(
        borderRadius: BorderRadius.circular(Radii.lg - 1),
        child: IntrinsicHeight(
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Container(width: 4, color: tone.base),
              Expanded(
                child: Padding(
                  padding: const EdgeInsets.fromLTRB(
                    Space.md + 2,
                    Space.md + 2,
                    Space.md + 2,
                    Space.md,
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Expanded(
                            child: Text(
                              'Example Course',
                              style: typography.body.md.copyWith(
                                fontWeight: FontWeight.w600,
                                color: colors.foreground,
                              ),
                            ),
                          ),
                          Text.rich(
                            TextSpan(
                              children: [
                                TextSpan(
                                  text: '92%',
                                  style: typography.body.lg.copyWith(
                                    height: 1.2,
                                    fontWeight: FontWeight.w500,
                                    color: tone.base,
                                  ),
                                ),
                                mark('1'),
                              ],
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: Space.md),
                      SkipMeter(percent: 92, tone: tone),
                      const SizedBox(height: Space.md),
                      Row(
                        children: [
                          Text.rich(
                            TextSpan(
                              children: [
                                TextSpan(text: '22/24 attended', style: muted),
                                mark('2'),
                              ],
                            ),
                          ),
                          const Spacer(),
                          Text.rich(
                            TextSpan(
                              children: [
                                TextSpan(
                                  text: 'Can skip 5',
                                  style: muted.copyWith(
                                    fontWeight: FontWeight.w500,
                                  ),
                                ),
                                mark('3'),
                              ],
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: Space.md),
                      Container(height: 1, color: colors.border),
                      const SizedBox(height: Space.sm),
                      Center(
                        child: Text.rich(
                          TextSpan(
                            children: [
                              TextSpan(text: '1+', style: small),
                              mark('4'),
                              TextSpan(text: ' 16', style: strong),
                              TextSpan(text: ' left', style: small),
                              mark('5'),
                              TextSpan(text: '  ·  ', style: small),
                              TextSpan(text: '8', style: strong),
                              TextSpan(text: ' to spare', style: small),
                              mark('6'),
                              TextSpan(text: '  ·  ', style: small),
                              TextSpan(text: '95%', style: strong),
                              TextSpan(text: ' max', style: small),
                              mark('7'),
                            ],
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// The timetable's "84% · skip 3" attendance beside a class.
class _SampleGutter extends StatelessWidget {
  const _SampleGutter();

  @override
  Widget build(BuildContext context) {
    final tone = context.theme.colors.app.success;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.end,
      children: [
        Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            ProgressRing(value: 0.84, color: tone.base, size: 11, stroke: 2),
            const SizedBox(width: 4),
            Text(
              '84%',
              style: TextStyle(
                fontSize: 11,
                fontWeight: FontWeight.w700,
                color: tone.onSubtle,
              ),
            ),
          ],
        ),
        Text(
          'skip 3',
          style: TextStyle(
            fontSize: 10,
            fontWeight: FontWeight.w500,
            color: tone.onSubtle.withValues(alpha: 0.8),
          ),
        ),
      ],
    );
  }
}

/// Numbered circle tying a sample number to its explanation.
class _Marker extends StatelessWidget {
  const _Marker(this.number, {this.size = 20});

  final String number;
  final double size;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    return Container(
      width: size,
      height: size,
      alignment: Alignment.center,
      decoration: BoxDecoration(
        color: colors.secondary,
        shape: BoxShape.circle,
      ),
      child: Text(
        number,
        style: TextStyle(
          fontSize: size * 0.6,
          height: 1,
          fontWeight: FontWeight.w700,
          color: colors.mutedForeground,
        ),
      ),
    );
  }
}

/// One term: a marker (or [lead] icon), its name, what it means, and
/// optionally the example's [math], the [variants] it can read as, and a
/// [footer]. A hairline sits below unless [last].
class _Term extends StatelessWidget {
  const _Term({
    this.mark,
    this.lead,
    required this.title,
    required this.body,
    this.math,
    this.variants,
    this.footer,
    this.last = false,
  });

  final String? mark;
  final Widget? lead;
  final String title;
  final String body;
  final String? math;
  final String? variants;
  final Widget? footer;
  final bool last;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final bodyStyle = typography.body.sm.copyWith(
      height: 1.45,
      color: colors.mutedForeground,
    );
    return DecoratedBox(
      decoration: BoxDecoration(
        border: last ? null : Border(bottom: BorderSide(color: colors.border)),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: Space.md + 2),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            SizedBox(
              width: 40,
              child: Align(
                alignment: Alignment.topLeft,
                child: Padding(
                  padding: const EdgeInsets.only(top: 1),
                  child: lead ?? _Marker(mark ?? ''),
                ),
              ),
            ),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    title,
                    style: typography.body.md.copyWith(
                      height: 1.3,
                      fontWeight: FontWeight.w600,
                      color: colors.foreground,
                      fontFeatures: const [FontFeature.tabularFigures()],
                    ),
                  ),
                  const SizedBox(height: 3),
                  Text(body, style: bodyStyle),
                  if (math != null) ...[
                    const SizedBox(height: Space.sm),
                    Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: Space.sm + 2,
                        vertical: Space.xs + 1,
                      ),
                      decoration: BoxDecoration(
                        color: colors.secondary,
                        borderRadius: BorderRadius.circular(Radii.sm),
                      ),
                      child: Text(
                        math!,
                        style: typography.body.xs.copyWith(
                          fontWeight: FontWeight.w500,
                          color: colors.foreground,
                          fontFeatures: const [FontFeature.tabularFigures()],
                        ),
                      ),
                    ),
                  ],
                  if (variants != null) ...[
                    const SizedBox(height: Space.sm),
                    Text(
                      variants!,
                      style: typography.body.xs.copyWith(
                        height: 1.45,
                        color: colors.mutedForeground,
                      ),
                    ),
                  ],
                  if (footer != null) ...[
                    const SizedBox(height: Space.sm + 2),
                    footer!,
                  ],
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// A week-recap dot and what it means.
class _DotKey extends StatelessWidget {
  const _DotKey({this.fill, this.ring, required this.label});

  final Color? fill;
  final Color? ring;
  final String label;

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Container(
          width: 7,
          height: 7,
          decoration: BoxDecoration(
            color: fill,
            shape: BoxShape.circle,
            border: ring == null ? null : Border.all(color: ring!, width: 1.2),
          ),
        ),
        const SizedBox(width: 5),
        Text(
          label,
          style: context.theme.typography.body.xs.copyWith(
            color: context.theme.colors.foreground,
          ),
        ),
      ],
    );
  }
}

class _Bullet extends StatelessWidget {
  const _Bullet({required this.text, required this.color});

  final String text;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: Space.xs + 1),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // On the first line's centre, not the item's.
          Padding(
            padding: const EdgeInsets.only(top: 8),
            child: Container(
              width: 5,
              height: 5,
              decoration: BoxDecoration(color: color, shape: BoxShape.circle),
            ),
          ),
          const SizedBox(width: Space.sm + 2),
          Expanded(
            child: Text(
              text,
              style: context.theme.typography.body.sm.copyWith(
                color: context.theme.colors.foreground,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
