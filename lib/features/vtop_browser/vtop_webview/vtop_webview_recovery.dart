/// A successful authenticated page starts a new recovery cycle. Completing a
/// login attempt alone does not, since the server may redirect straight back.
class VtopWebviewRecovery {
  bool _attempted = false;

  bool beginAutomaticAttempt() {
    if (_attempted) return false;
    _attempted = true;
    return true;
  }

  void authenticatedPageReady() => _attempted = false;
}
