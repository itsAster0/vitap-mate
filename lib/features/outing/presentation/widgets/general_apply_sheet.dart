import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:intl/intl.dart';
import 'package:vitapmate/core/utils/vtop_client_call.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/outing/data/outing_service.dart';
import 'package:vitapmate/features/outing/domain/outing_rules.dart';
import 'package:vitapmate/features/outing/presentation/widgets/apply_sheet_frame.dart';
import 'package:vitapmate/features/outing/presentation/widgets/outing_widgets.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

/// Opens the general outing form. Returns VTOP's result once a request is
/// sent, or null when the sheet is closed without applying.
Future<OutingApplyResult?> showGeneralApplySheet(
  BuildContext context,
  GeneralOutingData form,
) => showFSheet<OutingApplyResult>(
  context: context,
  side: FLayout.btt,
  // Over the bottom navigation bar, so the keyboard inset is counted once.
  useRootNavigator: true,
  mainAxisMaxRatio: 0.92,
  builder: (context) => _GeneralApplySheet(form: form),
);

/// A default leaving time VTOP will accept: just over 24 hours from now, on
/// the next half hour, moved into the offered hours.
DateTime earliestLeaving(GeneralOutingData form, DateTime now) {
  // Ten minutes of slack so the default is still valid while the form is
  // being filled in.
  final notice = now.add(generalOutingNotice + const Duration(minutes: 10));
  final halfHours = (notice.minute + 29) ~/ 30;
  var candidate = DateTime(
    notice.year,
    notice.month,
    notice.day,
    notice.hour,
  ).add(Duration(minutes: halfHours * 30));
  final window = offeredWindow(form.outHours);
  if (window == null) return candidate;
  final minutes = dayMinutes(candidate.hour, candidate.minute);
  if (minutes < window.earliest) {
    candidate = dateOnly(candidate).add(Duration(minutes: window.earliest));
  } else if (minutes > window.latest) {
    candidate = dateOnly(
      candidate,
    ).add(Duration(days: 1, minutes: window.earliest));
  }
  return candidate;
}

/// The latest return VTOP offers on [leaving]'s day, else the next day.
DateTime defaultReturn(GeneralOutingData form, DateTime leaving) {
  final latest = offeredWindow(form.inHours)?.latest ?? dayMinutes(20, 0);
  final sameDay = dateOnly(leaving).add(Duration(minutes: latest));
  return sameDay.isAfter(leaving)
      ? sameDay
      : sameDay.add(const Duration(days: 1));
}

class _GeneralApplySheet extends ConsumerStatefulWidget {
  const _GeneralApplySheet({required this.form});

  final GeneralOutingData form;

  @override
  ConsumerState<_GeneralApplySheet> createState() => _GeneralApplySheetState();
}

class _GeneralApplySheetState extends ConsumerState<_GeneralApplySheet> {
  final _place = TextEditingController();
  final _purpose = TextEditingController();
  late DateTime _leaving;
  late DateTime _returning;
  bool _showErrors = false;
  bool _sending = false;
  String? _failure;

  GeneralOutingData get form => widget.form;

  @override
  void initState() {
    super.initState();
    _leaving = earliestLeaving(form, DateTime.now());
    _returning = defaultReturn(form, _leaving);
    _place.addListener(_changed);
    _purpose.addListener(_changed);
  }

  void _changed() => setState(() {});

  @override
  void dispose() {
    _place.dispose();
    _purpose.dispose();
    super.dispose();
  }

  GeneralOutingCheck get _check => checkGeneralOuting(
    form: form,
    now: DateTime.now(),
    place: _place.text,
    purpose: _purpose.text,
    leaving: _leaving,
    returning: _returning,
  );

  Future<void> _pickDate({required bool leaving}) async {
    final now = DateTime.now();
    final range = leaving
        ? generalLeavingRange(form, now)
        : generalReturnRange(_leaving);
    final current = leaving ? _leaving : _returning;
    final picked = await pickOutingDate(
      context,
      title: leaving ? 'Leaving on' : 'Back on',
      first: range.first,
      last: range.last,
      initial: dateOnly(current),
    );
    if (picked == null || !mounted) return;
    setState(() {
      final moved = DateTime(
        picked.year,
        picked.month,
        picked.day,
        current.hour,
        current.minute,
      );
      if (leaving) {
        final gap = _returning.difference(_leaving);
        _leaving = moved;
        // Keep the trip's length when the leaving day moves.
        _returning = _leaving.add(gap);
      } else {
        _returning = moved;
      }
    });
  }

  Future<void> _pickTime({required bool leaving}) async {
    final hours = leaving ? form.outHours : form.inHours;
    final window = offeredWindow(hours);
    final current = leaving ? _leaving : _returning;
    final picked = await pickOutingTime(
      context,
      title: leaving ? 'Leaving at' : 'Back by',
      initial: FTime(current.hour, current.minute),
      hint: window == null
          ? null
          : '${clockLabel(window.earliest)} to ${clockLabel(window.latest)}',
    );
    if (picked == null || !mounted) return;
    setState(() {
      final moved = DateTime(
        current.year,
        current.month,
        current.day,
        picked.hour,
        picked.minute,
      );
      if (leaving) {
        _leaving = moved;
      } else {
        _returning = moved;
      }
    });
  }

  String _tripLength() {
    final span = _returning.difference(_leaving);
    if (span.isNegative || span == Duration.zero) return '';
    final days = span.inDays;
    final hours = span.inHours % 24;
    final minutes = span.inMinutes % 60;
    return [
      if (days > 0) '$days d',
      if (hours > 0) '$hours h',
      if (days == 0 && minutes > 0) '$minutes min',
    ].join(' ');
  }

  Future<void> _review() async {
    final check = _check;
    if (!check.isValid) {
      setState(() {
        _showErrors = true;
        _failure = 'Check the fields marked in red.';
      });
      return;
    }
    final format = DateFormat('EEE d MMM, h:mm a', 'en_US');
    final confirmed = await confirmApplication(
      context,
      title: 'Apply for general outing?',
      rows: [
        ('Place', tidyText(_place.text)),
        ('Purpose', tidyText(_purpose.text)),
        ('Leaving', format.format(_leaving)),
        ('Back by', format.format(_returning)),
        if (form.student case final student?)
          ('Room', '${student.hostelBlock} · ${student.roomNumber}'),
      ],
      footnote:
          'Goes for approval. You can cancel it from your requests until '
          'it is approved.',
    );
    if (!confirmed || !mounted) return;
    setState(() {
      _sending = true;
      _failure = null;
    });
    try {
      final result = await ref
          .read(outingServiceProvider)
          .applyGeneral(
            place: tidyText(_place.text),
            purpose: tidyText(_purpose.text),
            leaving: _leaving,
            returning: _returning,
            formatDate: vtopOutingDate,
          );
      if (!mounted) return;
      if (result.applied) {
        Navigator.of(context).pop(result);
      } else {
        setState(() => _failure = result.message);
      }
    } catch (error) {
      if (mounted) setState(() => _failure = vtopErrorMessage(error));
    } finally {
      if (mounted) setState(() => _sending = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final check = _check;
    final now = DateTime.now();
    final outWindow = offeredWindow(form.outHours);
    final inWindow = offeredWindow(form.inHours);
    String? shown(String? error) => _showErrors ? error : null;

    return ApplySheetFrame(
      title: 'General outing',
      subtitle: switch (form.student) {
        final student? => '${student.hostelBlock} · Room ${student.roomNumber}',
        null => null,
      },
      failure: _failure,
      sending: _sending,
      onSubmit: _review,
      children: [
        KeepVisibleOnFocus(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              FieldLabel(
                'Place of visit',
                trailing: Text('${_place.text.length}/${form.placeMaxLength}'),
              ),
              FTextField(
                control: FTextFieldControl.managed(controller: _place),
                hint: 'Where are you going?',
                maxLength: form.placeMaxLength,
                textCapitalization: TextCapitalization.words,
                textInputAction: TextInputAction.next,
                counterBuilder: (_, _, _, _) => null,
              ),
              FieldError(shown(check.place)),
            ],
          ),
        ),
        const SizedBox(height: Space.md),
        KeepVisibleOnFocus(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              FieldLabel(
                'Purpose',
                trailing: Text(
                  '${_purpose.text.length}/${form.purposeMaxLength}',
                ),
              ),
              FTextField(
                control: FTextFieldControl.managed(controller: _purpose),
                hint: 'Why are you going?',
                maxLength: form.purposeMaxLength,
                textCapitalization: TextCapitalization.sentences,
                textInputAction: TextInputAction.done,
                counterBuilder: (_, _, _, _) => null,
              ),
              FieldError(shown(check.purpose)),
            ],
          ),
        ),
        const SizedBox(height: Space.md),
        FieldLabel(
          'Leaving',
          trailing: outWindow == null
              ? null
              : Text(
                  '${clockLabel(outWindow.earliest)} – '
                  '${clockLabel(outWindow.latest)}',
                ),
        ),
        _MomentRow(
          moment: _leaving,
          now: now,
          error: check.leaving != null,
          onDate: () => _pickDate(leaving: true),
          onTime: () => _pickTime(leaving: true),
        ),
        // Timing problems show at once: they come from the pickers, not typing.
        FieldError(check.leaving),
        const SizedBox(height: Space.md),
        FieldLabel(
          'Back by',
          trailing: inWindow == null
              ? null
              : Text(
                  '${clockLabel(inWindow.earliest)} – '
                  '${clockLabel(inWindow.latest)}',
                ),
        ),
        _MomentRow(
          moment: _returning,
          now: now,
          error: check.returning != null,
          onDate: () => _pickDate(leaving: false),
          onTime: () => _pickTime(leaving: false),
        ),
        FieldError(check.returning),
        if (check.leaving == null && check.returning == null) ...[
          const SizedBox(height: Space.sm),
          Text(
            'Away for ${_tripLength()}',
            style: context.theme.typography.body.xs.copyWith(
              color: context.theme.colors.mutedForeground,
            ),
          ),
        ],
      ],
    );
  }
}

class _MomentRow extends StatelessWidget {
  const _MomentRow({
    required this.moment,
    required this.now,
    required this.error,
    required this.onDate,
    required this.onTime,
  });

  final DateTime moment;
  final DateTime now;
  final bool error;
  final VoidCallback onDate;
  final VoidCallback onTime;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        Expanded(
          flex: 3,
          child: PickerField(
            icon: FLucideIcons.calendar,
            text: shortDate(moment, now),
            placeholder: 'Date',
            error: error,
            onPress: onDate,
          ),
        ),
        const SizedBox(width: Space.sm),
        Expanded(
          flex: 2,
          child: PickerField(
            icon: FLucideIcons.clock,
            text: clockLabel(dayMinutes(moment.hour, moment.minute)),
            placeholder: 'Time',
            error: error,
            onPress: onTime,
          ),
        ),
      ],
    );
  }
}
