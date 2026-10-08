import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:intl/intl.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/core/utils/vtop_client_call.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/outing/data/outing_service.dart';
import 'package:vitapmate/features/outing/domain/outing_rules.dart';
import 'package:vitapmate/features/outing/presentation/widgets/apply_sheet_frame.dart';
import 'package:vitapmate/features/outing/presentation/widgets/outing_widgets.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

/// The last contact number used, so it does not need typing every week.
const _contactKey = 'outing_contact_number';

Future<OutingApplyResult?> showWeekendApplySheet(
  BuildContext context,
  WeekendOutingData form,
) => showFSheet<OutingApplyResult>(
  context: context,
  side: FLayout.btt,
  // Over the bottom navigation bar, so the keyboard inset is counted once.
  useRootNavigator: true,
  mainAxisMaxRatio: 0.92,
  builder: (context) => _WeekendApplySheet(form: form),
);

class _WeekendApplySheet extends ConsumerStatefulWidget {
  const _WeekendApplySheet({required this.form});

  final WeekendOutingData form;

  @override
  ConsumerState<_WeekendApplySheet> createState() => _WeekendApplySheetState();
}

class _WeekendApplySheetState extends ConsumerState<_WeekendApplySheet> {
  final _purpose = TextEditingController();
  final _contact = TextEditingController();
  late final List<DateTime> _dates;
  DateTime? _date;
  String? _slot;
  String? _place;
  bool _showErrors = false;
  bool _sending = false;
  String? _failure;

  WeekendOutingData get form => widget.form;

  @override
  void initState() {
    super.initState();
    _dates = weekendOutingDates(form, DateTime.now());
    _date = _dates.firstOrNull;
    _slot = form.timeSlots.firstOrNull?.value;
    _place = form.places.firstOrNull?.value;
    _purpose.addListener(_changed);
    _contact.addListener(_changed);
    SharedPreferences.getInstance().then((prefs) {
      final saved = prefs.getString(_contactKey);
      if (saved != null && _contact.text.isEmpty && mounted) {
        _contact.text = saved;
      }
    });
  }

  void _changed() => setState(() {});

  @override
  void dispose() {
    _purpose.dispose();
    _contact.dispose();
    super.dispose();
  }

  WeekendOutingCheck get _check => checkWeekendOuting(
    form: form,
    now: DateTime.now(),
    purpose: _purpose.text,
    date: _date,
    contact: _contact.text,
  );

  Future<void> _review() async {
    final check = _check;
    if (!check.isValid || _slot == null || _place == null) {
      setState(() {
        _showErrors = true;
        _failure = 'Check the fields marked in red.';
      });
      return;
    }
    final confirmed = await confirmApplication(
      context,
      title: 'Apply for weekend outing?',
      rows: [
        ('Day', DateFormat('EEEE d MMM', 'en_US').format(_date!)),
        ('Slot', slotLabel(_slot!)),
        ('Place', _place!),
        ('Purpose', tidyText(_purpose.text)),
        ('Contact', _contact.text.trim()),
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
      final contact = _contact.text.trim();
      final result = await ref
          .read(outingServiceProvider)
          .applyWeekend(
            place: _place!,
            purpose: tidyText(_purpose.text),
            date: vtopOutingDate(_date!),
            timeSlot: _slot!,
            contactNumber: contact,
          );
      if (!mounted) return;
      if (result.applied) {
        await (await SharedPreferences.getInstance()).setString(
          _contactKey,
          contact,
        );
        if (mounted) Navigator.of(context).pop(result);
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
    String? shown(String? error) => _showErrors ? error : null;

    return ApplySheetFrame(
      title: 'Weekend outing',
      subtitle: switch (form.student) {
        final student? => '${student.hostelBlock} · Room ${student.roomNumber}',
        null => null,
      },
      failure: _failure,
      sending: _sending,
      onSubmit: _review,
      children: [
        FieldLabel('Day', trailing: Text(weekdayNames(form.weekdays))),
        if (_dates.isEmpty)
          const FieldError('VTOP offers no day in the coming week.')
        else
          Wrap(
            spacing: Space.sm,
            runSpacing: Space.sm,
            children: [
              for (final date in _dates)
                ChoicePill(
                  caption: DateFormat(
                    'EEE',
                    'en_US',
                  ).format(date).toUpperCase(),
                  label: DateFormat('d MMM', 'en_US').format(date),
                  selected: date == _date,
                  onPress: () => setState(() => _date = date),
                ),
            ],
          ),
        FieldError(shown(check.date)),
        const SizedBox(height: Space.md),
        const FieldLabel('Time slot'),
        GridView.count(
          crossAxisCount: 2,
          shrinkWrap: true,
          physics: const NeverScrollableScrollPhysics(),
          mainAxisSpacing: Space.sm,
          crossAxisSpacing: Space.sm,
          mainAxisExtent: 44,
          padding: EdgeInsets.zero,
          children: [
            for (final slot in form.timeSlots)
              ChoicePill(
                label: slotLabel(slot.value),
                selected: slot.value == _slot,
                onPress: () => setState(() => _slot = slot.value),
              ),
          ],
        ),
        const SizedBox(height: Space.md),
        const FieldLabel('Place'),
        Wrap(
          spacing: Space.sm,
          runSpacing: Space.sm,
          children: [
            for (final place in form.places)
              ChoicePill(
                label: place.label,
                selected: place.value == _place,
                onPress: () => setState(() => _place = place.value),
              ),
          ],
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
                textInputAction: TextInputAction.next,
                counterBuilder: (_, _, _, _) => null,
              ),
              FieldError(shown(check.purpose)),
            ],
          ),
        ),
        const SizedBox(height: Space.md),
        KeepVisibleOnFocus(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const FieldLabel('Your contact number'),
              FTextField(
                control: FTextFieldControl.managed(controller: _contact),
                hint: '10-digit mobile',
                keyboardType: TextInputType.phone,
                textInputAction: TextInputAction.done,
                inputFormatters: [
                  FilteringTextInputFormatter.digitsOnly,
                  LengthLimitingTextInputFormatter(10),
                ],
              ),
              FieldError(shown(check.contact)),
            ],
          ),
        ),
        if (form.student case final student?) ...[
          const SizedBox(height: Space.sm),
          Text(
            'Parent contact on file: ${student.parentContactNumber}',
            style: context.theme.typography.body.xs.copyWith(
              color: context.theme.colors.mutedForeground,
            ),
          ),
        ],
      ],
    );
  }
}
