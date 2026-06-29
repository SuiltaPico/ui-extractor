import 'extract_timings.dart';

/// UI extract JSON plus native stage timings.
class UiExtractOutput {
  const UiExtractOutput({
    required this.resultJson,
    required this.timings,
  });

  /// Tree payload (`width` / `height` / `root`) without the `timings` field.
  final Map<String, dynamic> resultJson;
  final UiExtractTimings timings;

  factory UiExtractOutput.fromJson(Map<String, dynamic> json) {
    final timings = UiExtractTimings.fromJson(json['timings']);
    final result = Map<String, dynamic>.from(json)..remove('timings');
    return UiExtractOutput(resultJson: result, timings: timings);
  }
}
