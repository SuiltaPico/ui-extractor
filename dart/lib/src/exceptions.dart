/// Thrown when the native library returns an error.
class UiExtractorException implements Exception {
  UiExtractorException(this.message);

  final String message;

  @override
  String toString() => 'UiExtractorException: $message';
}
