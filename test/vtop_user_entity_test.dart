import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/core/utils/entity/vtop_user_entity.dart';

void main() {
  group('VtopUserEntity', () {
    test('cannot configure an account with empty credentials', () {
      expect(
        () => VtopUserEntity.configured(
          username: '',
          password: 'secret',
          semesterId: 'semester',
        ),
        throwsArgumentError,
      );
    });

    test('migrates a valid legacy record', () {
      final user = VtopUserEntity.fromJson(const {
        'username': '23ABC0001',
        'password': 'secret',
        'semid': 'semester',
        'isValid': true,
      });

      expect(user, isA<ConfiguredVtopUser>());
      expect(user.username, '23ABC0001');
      expect(user.isValid, isTrue);
    });

    test('represents rejected credentials as a separate state', () {
      final user = VtopUserEntity.configured(
        username: '23ABC0001',
        password: 'old-secret',
        semesterId: 'semester',
      ).rejectCredentials();

      expect(user, isA<RejectedVtopUser>());
      expect(user.isValid, isFalse);
      expect(
        user.withCredentials('23ABC0001', 'new-secret'),
        isA<ConfiguredVtopUser>(),
      );
    });

    test('does not normalize password contents', () {
      final user = VtopUserEntity.configured(
        username: '23ABC0001',
        password: ' secret with spaces ',
        semesterId: 'semester',
      );

      expect(user.password, ' secret with spaces ');
    });
  });
}
