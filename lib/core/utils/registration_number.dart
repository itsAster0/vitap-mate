String deriveRegistrationNumber(String email) {
  final trimmedEmail = email.trim().toLowerCase();
  final localPart = trimmedEmail.split('@').first;
  final localSegments =
      RegExp(
        r'[a-z0-9]+',
      ).allMatches(localPart).map((match) => match.group(0)!).toList();
  final registrationSegment = localSegments.lastWhere(
    (segment) => RegExp(r'^\d{2}[a-z]{2,6}\d{4}$').hasMatch(segment),
    orElse: () => localSegments.isNotEmpty ? localSegments.last : '',
  );
  final normalized = registrationSegment.replaceAll(RegExp(r'[^a-z0-9]'), '');
  if (normalized.isEmpty) {
    throw const FormatException(
      'Could not derive a registration number from the selected email address.',
    );
  }
  return normalized;
}
