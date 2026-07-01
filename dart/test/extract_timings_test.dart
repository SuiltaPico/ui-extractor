import 'package:flutter_test/flutter_test.dart';
import 'package:ui_extractor/ui_extractor.dart';

void main() {
  test('UiExtractTimings parses native JSON', () {
    final timings = UiExtractTimings.fromJson({
      'gray_ms': 1.2,
      'layout_ms': 45.6,
      'parallel_ms': 210.0,
      'ocr': {'init_ms': 0, 'predict_ms': 180.5, 'det_ms': 70.0, 'rec_ms': 100.0, 'post_ms': 10.5},
      'attach_words_ms': 3.1,
      'icon': {
        'candidates': 12,
        'matched': 4,
        'timings': {'load_ms': 0, 'match_ms': 2200.0},
      },
    });

    expect(timings.grayMs, 1.2);
    expect(timings.layoutMs, 45.6);
    expect(timings.parallelMs, 210.0);
    expect(timings.ocr.predictMs, 180.5);
    expect(timings.ocr.detMs, 70.0);
    expect(timings.ocr.recMs, 100.0);
    expect(timings.icon.matched, 4);
    expect(timings.icon.timings.matchMs, 2200.0);
  });

  test('UiExtractIconTimings parses embed_detail', () {
    final timings = UiExtractIconTimings.fromJson({
      'embed_ms': 1775.0,
      'embed_detail': {
        'resize_ms': 12.0,
        'pack_nchw_ms': 8.0,
        'copy_input_ms': 5.0,
        'run_session_ms': 1600.0,
        'read_output_ms': 120.0,
        'finalize_ms': 3.0,
        'batch_runs': 5,
        'image_count': 36,
      },
    });

    expect(timings.embedMs, 1775.0);
    expect(timings.embedDetail.runSessionMs, 1600.0);
    expect(timings.embedDetail.batchRuns, 5);
    expect(timings.embedDetail.hasBreakdown, isTrue);
  });

  test('UiExtractOutput strips timings from resultJson', () {
    final output = UiExtractOutput.fromJson({
      'width': 100,
      'height': 200,
      'root': {'kind': 'root', 'children': []},
      'timings': {'layout_ms': 10.0},
    });

    expect(output.resultJson.containsKey('timings'), isFalse);
    expect(output.resultJson['width'], 100);
    expect(output.timings.layoutMs, 10.0);
  });
}
