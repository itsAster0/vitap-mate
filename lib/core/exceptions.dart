class FeatureDisabledException implements Exception {
  final String message;
  FeatureDisabledException([this.message = 'Invalid user input']);
  @override
  String toString() => 'FeatureDisabledException: $message';
}
