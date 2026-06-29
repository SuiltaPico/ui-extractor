/// Native extract stage timings from ui-extractor FFI JSON.
class UiExtractTimings {
  const UiExtractTimings({
    this.grayMs = 0,
    this.layoutMs = 0,
    this.pipelineDumpMs = 0,
    this.parallelMs = 0,
    this.ocr = const UiExtractOcrTimings(),
    this.attachWordsMs = 0,
    this.icon = const UiExtractIconStats(),
  });

  final double grayMs;
  final double layoutMs;
  final double pipelineDumpMs;
  final double parallelMs;
  final UiExtractOcrTimings ocr;
  final double attachWordsMs;
  final UiExtractIconStats icon;

  double get ocrTotalMs => ocr.initMs + ocr.predictMs;

  factory UiExtractTimings.fromJson(Object? json) {
    if (json is! Map) return const UiExtractTimings();
    final map = Map<String, dynamic>.from(json);
    return UiExtractTimings(
      grayMs: _readMs(map['gray_ms']),
      layoutMs: _readMs(map['layout_ms']),
      pipelineDumpMs: _readMs(map['pipeline_dump_ms']),
      parallelMs: _readMs(map['parallel_ms']),
      ocr: UiExtractOcrTimings.fromJson(map['ocr']),
      attachWordsMs: _readMs(map['attach_words_ms']),
      icon: UiExtractIconStats.fromJson(map['icon']),
    );
  }
}

class UiExtractOcrTimings {
  const UiExtractOcrTimings({
    this.initMs = 0,
    this.predictMs = 0,
  });

  final double initMs;
  final double predictMs;

  factory UiExtractOcrTimings.fromJson(Object? json) {
    if (json is! Map) return const UiExtractOcrTimings();
    final map = Map<String, dynamic>.from(json);
    return UiExtractOcrTimings(
      initMs: _readMs(map['init_ms']),
      predictMs: _readMs(map['predict_ms']),
    );
  }
}

class UiExtractIconStats {
  const UiExtractIconStats({
    this.candidates = 0,
    this.matched = 0,
    this.timings = const UiExtractIconTimings(),
  });

  final int candidates;
  final int matched;
  final UiExtractIconTimings timings;

  factory UiExtractIconStats.fromJson(Object? json) {
    if (json is! Map) return const UiExtractIconStats();
    final map = Map<String, dynamic>.from(json);
    return UiExtractIconStats(
      candidates: (map['candidates'] as num?)?.toInt() ?? 0,
      matched: (map['matched'] as num?)?.toInt() ?? 0,
      timings: UiExtractIconTimings.fromJson(map['timings']),
    );
  }
}

class UiExtractIconTimings {
  const UiExtractIconTimings({
    this.loadMs = 0,
    this.matchMs = 0,
  });

  final double loadMs;
  final double matchMs;

  factory UiExtractIconTimings.fromJson(Object? json) {
    if (json is! Map) return const UiExtractIconTimings();
    final map = Map<String, dynamic>.from(json);
    return UiExtractIconTimings(
      loadMs: _readMs(map['load_ms']),
      matchMs: _readMs(map['match_ms']),
    );
  }
}

double _readMs(Object? value) => (value as num?)?.toDouble() ?? 0;
