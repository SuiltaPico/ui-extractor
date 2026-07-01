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
    this.detMs = 0,
    this.recMs = 0,
    this.postMs = 0,
  });

  final double initMs;
  final double predictMs;
  final double detMs;
  final double recMs;
  final double postMs;

  factory UiExtractOcrTimings.fromJson(Object? json) {
    if (json is! Map) return const UiExtractOcrTimings();
    final map = Map<String, dynamic>.from(json);
    return UiExtractOcrTimings(
      initMs: _readMs(map['init_ms']),
      predictMs: _readMs(map['predict_ms']),
      detMs: _readMs(map['det_ms']),
      recMs: _readMs(map['rec_ms']),
      postMs: _readMs(map['post_ms']),
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
    this.grayMs = 0,
    this.cropMs = 0,
    this.preprocessMs = 0,
    this.embedMs = 0,
    this.indexMs = 0,
    this.matchMs = 0,
    this.embedDetail = const UiExtractIconEmbedDetail(),
  });

  final double loadMs;
  final double grayMs;
  final double cropMs;
  final double preprocessMs;
  final double embedMs;
  final double indexMs;
  final double matchMs;
  final UiExtractIconEmbedDetail embedDetail;

  factory UiExtractIconTimings.fromJson(Object? json) {
    if (json is! Map) return const UiExtractIconTimings();
    final map = Map<String, dynamic>.from(json);
    return UiExtractIconTimings(
      loadMs: _readMs(map['load_ms']),
      grayMs: _readMs(map['gray_ms']),
      cropMs: _readMs(map['crop_ms']),
      preprocessMs: _readMs(map['preprocess_ms']),
      embedMs: _readMs(map['embed_ms']),
      indexMs: _readMs(map['index_ms']),
      matchMs: _readMs(map['match_ms']),
      embedDetail: UiExtractIconEmbedDetail.fromJson(map['embed_detail']),
    );
  }
}

class UiExtractIconEmbedDetail {
  const UiExtractIconEmbedDetail({
    this.resizeMs = 0,
    this.packNchwMs = 0,
    this.copyInputMs = 0,
    this.runSessionMs = 0,
    this.readOutputMs = 0,
    this.finalizeMs = 0,
    this.batchRuns = 0,
    this.imageCount = 0,
  });

  final double resizeMs;
  final double packNchwMs;
  final double copyInputMs;
  final double runSessionMs;
  final double readOutputMs;
  final double finalizeMs;
  final int batchRuns;
  final int imageCount;

  bool get hasBreakdown =>
      resizeMs > 0 ||
      packNchwMs > 0 ||
      copyInputMs > 0 ||
      runSessionMs > 0 ||
      readOutputMs > 0 ||
      finalizeMs > 0;

  factory UiExtractIconEmbedDetail.fromJson(Object? json) {
    if (json is! Map) return const UiExtractIconEmbedDetail();
    final map = Map<String, dynamic>.from(json);
    return UiExtractIconEmbedDetail(
      resizeMs: _readMs(map['resize_ms']),
      packNchwMs: _readMs(map['pack_nchw_ms']),
      copyInputMs: _readMs(map['copy_input_ms']),
      runSessionMs: _readMs(map['run_session_ms']),
      readOutputMs: _readMs(map['read_output_ms']),
      finalizeMs: _readMs(map['finalize_ms']),
      batchRuns: (map['batch_runs'] as num?)?.toInt() ?? 0,
      imageCount: (map['image_count'] as num?)?.toInt() ?? 0,
    );
  }
}

double _readMs(Object? value) => (value as num?)?.toDouble() ?? 0;
