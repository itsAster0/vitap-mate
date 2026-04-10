import 'package:vitapmate/src/api/vtop/types.dart';

class StudentIdentity {
  const StudentIdentity({
    this.email,
    required this.registrationNumber,
    required this.selectedSemesterId,
  });

  final String? email;
  final String registrationNumber;
  final String selectedSemesterId;

  Map<String, dynamic> toJson() {
    return {
      'email': email,
      'registrationNumber': registrationNumber,
      'selectedSemesterId': selectedSemesterId,
    };
  }

  factory StudentIdentity.fromJson(Map<String, dynamic> json) {
    return StudentIdentity(
      email: json['email'] as String?,
      registrationNumber: json['registrationNumber'] as String? ?? '',
      selectedSemesterId: json['selectedSemesterId'] as String? ?? '',
    );
  }
}

class ActiveAccount {
  const ActiveAccount({required this.identity, this.isValid = true});

  final StudentIdentity identity;
  final bool isValid;

  ActiveAccount copyWith({StudentIdentity? identity, bool? isValid}) {
    return ActiveAccount(
      identity: identity ?? this.identity,
      isValid: isValid ?? this.isValid,
    );
  }

  Map<String, dynamic> toJson() {
    return {'identity': identity.toJson(), 'isValid': isValid};
  }

  factory ActiveAccount.fromJson(Map<String, dynamic> json) {
    return ActiveAccount(
      identity: StudentIdentity.fromJson(
        json['identity'] as Map<String, dynamic>? ?? const <String, dynamic>{},
      ),
      isValid: json['isValid'] as bool? ?? true,
    );
  }
}

class PendingSignInResult {
  const PendingSignInResult({
    required this.email,
    required this.registrationNumber,
    required this.password,
    required this.semesters,
    required this.cookie,
  });

  final String? email;
  final String registrationNumber;
  final String password;
  final SemesterData semesters;
  final String cookie;
}
