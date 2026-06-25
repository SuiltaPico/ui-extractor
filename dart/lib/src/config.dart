import 'dart:io';

import 'package:path/path.dart' as p;

/// Configuration passed to the native extractor as JSON.
class ExtractorConfig {
  const ExtractorConfig({
    this.runOcr = true,
    this.runIcon = true,
    this.minArea = 100,
    this.modelsDir,
    this.ocrPack = 'ocr.paddle.ppocr6-tiny.onnx.fp32',
    this.iconIndexPack = 'icons.bundled.v1.mobileclip2-s0.int8',
    this.ocrMaxSide = 960,
    this.ocrMinConfidence = 0.5,
    this.templateSize = 48,
    this.minCosine = 0.72,
    this.iconMinSide = 12,
    this.iconMaxSide = 96,
    this.iconMinAspect = 0.55,
    this.iconMaxAspect = 1.85,
    this.runtime,
  });

  final bool runOcr;
  final bool runIcon;
  final int minArea;
  final String? modelsDir;
  final String ocrPack;
  final String iconIndexPack;
  final int ocrMaxSide;
  final double ocrMinConfidence;
  final int templateSize;
  final double minCosine;
  final int iconMinSide;
  final int iconMaxSide;
  final double iconMinAspect;
  final double iconMaxAspect;
  final Map<String, dynamic>? runtime;

  /// Default pack layout under `./models` or `LOCAL_INFER_ROOT`.
  factory ExtractorConfig.defaults({String? modelsDir}) {
    return ExtractorConfig(
      modelsDir: modelsDir ?? _defaultModelsDir(),
    );
  }

  static String _defaultModelsDir() {
    final fromEnv = Platform.environment['LOCAL_INFER_ROOT'];
    if (fromEnv != null && fromEnv.isNotEmpty) {
      return p.normalize(fromEnv);
    }
    return p.normalize('models');
  }

  Map<String, dynamic> toJson() {
    final ocr = <String, dynamic>{
      'max_side': ocrMaxSide,
      'min_confidence': ocrMinConfidence,
    };

    final icon = <String, dynamic>{
      'template_size': templateSize,
      'min_cosine': minCosine,
      'min_side': iconMinSide,
      'max_side': iconMaxSide,
      'min_aspect': iconMinAspect,
      'max_aspect': iconMaxAspect,
    };

    return {
      'run_ocr': runOcr,
      'run_icon': runIcon,
      'layout': {'min_area': minArea},
      if (modelsDir != null) 'models_dir': p.normalize(modelsDir!),
      'ocr_pack': ocrPack,
      'icon_index_pack': iconIndexPack,
      if (runtime != null) 'runtime': runtime,
      'ocr': ocr,
      'icon': icon,
    };
  }
}
