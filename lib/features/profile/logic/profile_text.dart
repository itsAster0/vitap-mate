import 'package:vitapmate/src/api/vtop/types.dart';

/// VTOP shows "-", "NA" or nothing for fields that do not apply.
bool hasValue(String value) {
  final v = value.trim().toUpperCase();
  return v.isNotEmpty && v != '-' && v != 'NA' && v != 'N/A';
}

/// The first shown value labelled [label] (any case) in [groups].
String? fieldValue(List<ProfileGroup> groups, String label) {
  for (final group in groups) {
    for (final field in group.fields) {
      if (field.label.toUpperCase() == label && hasValue(field.value)) {
        return field.value.trim();
      }
    }
  }
  return null;
}

bool isSecret(String label) => label.toUpperCase().contains(RegExp('AADHAA?R'));

bool isSerial(String label) {
  final l = label.toLowerCase().replaceAll(RegExp(r'[^a-z]'), '');
  return l == 'slno' || l == 'sno' || l == 'no';
}

/// Everything but the last four characters hidden.
String masked(String value) => value.length <= 4
    ? value
    : '${'•' * (value.length - 4)}${value.substring(value.length - 4)}';

/// "PERSONAL INFORMATION" → "Personal details".
String detailsTitle(String title) {
  final name = niceLabel(
    title.replaceAll(RegExp(r'\s*INFORMATION\s*$', caseSensitive: false), ''),
  );
  return name == 'Educational' ? 'Education' : '$name details';
}

/// "4- BED -AC" → "4 Bed AC".
String bedType(String value) =>
    niceCase(value.replaceAll(RegExp(r'\s*-\s*'), ' ').trim());

/// VTOP's upper-case labels in sentence case, keeping the abbreviations:
/// "EMP.ID(IF EMPLOYED IN VIT)" → "Emp. ID (if employed in VIT)".
String niceLabel(String label) {
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
const _shortWords = {'YES', 'NO', 'MR', 'MRS', 'DR', 'ST', 'RD', 'OPP', 'BED'};

/// An all-caps (or all-lowercase) value in title case: "ANDHRA PRADESH" →
/// "Andhra Pradesh". Emails and mixed-case values are left as they are;
/// short abbreviations like OC, MPC or B+ stay upper case.
String niceCase(String value) {
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
      !_minorWords.contains(letters.toLowerCase()) &&
      !RegExp(r'\d').hasMatch(w)) {
    return upper;
  }
  final i = lower.indexOf(RegExp(r'[a-z]'));
  if (i < 0 || RegExp(r'\d').hasMatch(lower.substring(0, i))) return lower;
  return '${lower.substring(0, i)}${lower[i].toUpperCase()}'
      '${lower.substring(i + 1)}';
}

String initials(String name) => name
    .split(RegExp(r'\s+'))
    .where((p) => p.isNotEmpty)
    .take(2)
    .map((p) => p[0].toUpperCase())
    .join();
