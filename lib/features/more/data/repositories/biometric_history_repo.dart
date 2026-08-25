import 'package:vitapmate/core/utils/cached_repository.dart';
import 'package:vitapmate/features/more/data/datasources/data_source.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class BiometricHistoryRepository extends CachedRepository<BiometricData> {
  BiometricHistoryRepository({
    required this.date,
    required BiometricHistoryDataSource dataSource,
  }) : _dataSource = dataSource;

  final String date;
  final BiometricHistoryDataSource _dataSource;

  @override
  Future<BiometricData?> loadCache() {
    return _dataSource.getBiometricHistory(date);
  }

  @override
  Future<void> saveCache(BiometricData data) {
    return _dataSource.saveBiometricHistory(data, date);
  }

  @override
  Future<BiometricData> fetchRemote() {
    return _dataSource.fetchBiometricHistory(date);
  }
}
