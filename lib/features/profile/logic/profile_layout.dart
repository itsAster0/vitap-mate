import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/features/profile/logic/profile_text.dart';

/// The profile's sections sorted for display. The cards at the top (hostel,
/// contact, addresses, parents) pick VTOP's fields by label; every field
/// they do not show stays in the collapsed sections below, so nothing VTOP
/// sends is lost.
class ProfileLayout {
  ProfileLayout(StudentProfile profile) {
    var personal = const <ProfileGroup>[];
    for (final section in profile.sections) {
      final title = section.title.toUpperCase();
      if (title.contains('PROCTOR')) continue; // On the Mentor tab.
      if (title.contains('HOSTEL')) {
        hostel = section.groups;
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
      final groups = [
        for (final group in rest)
          ProfileGroup(
            title: group.title,
            fields: [
              for (final field in group.fields)
                if (!_shownAbove(field.label)) field,
            ],
          ),
      ];
      if (groups.any((g) => g.fields.any((f) => hasValue(f.value)))) {
        details.add((section.title, groups));
      }
    }
    contacts = [
      if (hasValue(profile.vitEmail)) ('VIT email', profile.vitEmail),
      if (fieldValue(personal, 'MOBILE NUMBER') case final v?) ('Mobile', v),
      if (fieldValue([...personal, ...addresses], 'PARENT WHATSAPP NUMBER')
          case final v?)
        ('Parent WhatsApp', v),
    ];
    for (final group in addresses) {
      final email = fieldValue([group], 'EMAIL');
      if (email != null && !contacts.any((c) => c.$2 == email)) {
        contacts.add(('Personal email', email));
      }
    }
  }

  List<ProfileGroup> hostel = const [];
  final addresses = <ProfileGroup>[];
  final parents = <ProfileGroup>[];
  late final List<(String, String)> contacts;

  /// Section title and its groups, without what the cards show; sections
  /// with nothing left are dropped.
  final details = <(String, List<ProfileGroup>)>[];

  /// The name is in the header and the contacts are in their own card.
  static bool _shownAbove(String label) => const {
    'STUDENT NAME',
    'MOBILE NUMBER',
    'PARENT WHATSAPP NUMBER',
  }.contains(label.toUpperCase());
}
