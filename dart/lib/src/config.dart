import 'dart:io';

import 'package:local_infer_core/local_infer_core.dart';
import 'package:path/path.dart' as p;

/// Layout / OCR / icon parameters passed to ui-extractor (no models_dir or runtime).
class ExtractorLayoutConfig {
  const ExtractorLayoutConfig({
    this.runOcr = true,
    this.runIcon = true,
    this.minArea = 100,
    this.ocrPack = defaultOcrPack,
    this.iconIndexPack = defaultIconIndexPack,
    this.ocrMaxSide = 960,
    this.ocrMinConfidence = 0.5,
    this.templateSize = 48,
    this.minCosine = 0.72,
    this.iconMinSide = 12,
    this.iconMaxSide = 96,
    this.iconMinAspect = 0.55,
    this.iconMaxAspect = 1.85,
  });

  static const defaultOcrPack = 'ocr.paddle.ppocr6-tiny.onnx.fp32';
  static const defaultIconIndexPack =
      'icons.bundled.v1.mobileclip2-s0.int8';

  final bool runOcr;
  final bool runIcon;
  final int minArea;
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

  Map<String, dynamic> toJson() {
    return {
      'run_ocr': runOcr,
      'run_icon': runIcon,
      'layout': {'min_area': minArea},
      'ocr_pack': ocrPack,
      'icon_index_pack': iconIndexPack,
      'ocr': {
        'max_side': ocrMaxSide,
        'min_confidence': ocrMinConfidence,
      },
      'icon': {
        'template_size': templateSize,
        'min_cosine': minCosine,
        'min_side': iconMinSide,
        'max_side': iconMaxSide,
        'min_aspect': iconMinAspect,
        'max_aspect': iconMaxAspect,
      },
    };
  }
}

/// Standalone config: includes models root and optional runtime for owned registry.
class ExtractorConfig extends ExtractorLayoutConfig {
  const ExtractorConfig({
    super.runOcr,
    super.runIcon,
    super.minArea,
    super.ocrPack,
    super.iconIndexPack,
    super.ocrMaxSide,
    super.ocrMinConfidence,
    super.templateSize,
    super.minCosine,
    super.iconMinSide,
    super.iconMaxSide,
    super.iconMinAspect,
    super.iconMaxAspect,
    this.modelsDir,
    this.runtime,
  });

  final String? modelsDir;
  final RuntimeConfig? runtime;

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

  @override
  Map<String, dynamic> toJson() {
    return {
      ...super.toJson(),
      if (modelsDir != null) 'models_dir': p.normalize(modelsDir!),
      if (runtime != null) 'runtime': runtime!.toJson(),
    };
  }
}
