import 'dart:io';

import 'package:path/path.dart' as p;

/// Configuration passed to the native extractor as JSON.
class ExtractorConfig {
  const ExtractorConfig({
    this.runOcr = true,
    this.runIcon = true,
    this.minArea = 100,
    this.modelDir,
    this.ocrMaxSide = 960,
    this.ocrMinConfidence = 0.5,
    this.embeddingIndex,
    this.visionModel,
    this.templateSize = 48,
    this.minCosine = 0.72,
    this.iconMinSide = 12,
    this.iconMaxSide = 96,
    this.iconMinAspect = 0.55,
    this.iconMaxAspect = 1.85,
  });

  final bool runOcr;
  final bool runIcon;
  final int minArea;
  final String? modelDir;
  final int ocrMaxSide;
  final double ocrMinConfidence;
  final String? embeddingIndex;
  final String? visionModel;
  final int templateSize;
  final double minCosine;
  final int iconMinSide;
  final int iconMaxSide;
  final double iconMinAspect;
  final double iconMaxAspect;

  /// Relative defaults (`models/`, `assets/embeddings.bin`) for running inside
  /// an extracted release zip.
  factory ExtractorConfig.defaults({String baseDir = '.'}) {
    final root = p.normalize(baseDir);
    return ExtractorConfig.fromAssetRoot(root);
  }

  /// Build config from a directory that contains `models/` and `assets/`.
  factory ExtractorConfig.fromAssetRoot(String root) {
    final normalized = p.normalize(root);
    return ExtractorConfig(
      modelDir: p.join(normalized, 'models'),
      embeddingIndex: p.join(normalized, 'assets', 'embeddings.bin'),
      visionModel: _defaultVisionModel(normalized),
    );
  }

  static String _defaultVisionModel(String root) {
    if (Platform.isAndroid) {
      return p.join(root, 'models', 'mobileclip2-s0-vision.ncnn.param');
    }
    return p.join(root, 'models', 'mobileclip2-s0-vision.onnx');
  }

  Map<String, dynamic> toJson() {
    final ocr = <String, dynamic>{
      'max_side': ocrMaxSide,
      'min_confidence': ocrMinConfidence,
    };
    if (modelDir != null) {
      ocr['model_dir'] = p.normalize(modelDir!);
    }

    final icon = <String, dynamic>{
      'template_size': templateSize,
      'min_cosine': minCosine,
      'min_side': iconMinSide,
      'max_side': iconMaxSide,
      'min_aspect': iconMinAspect,
      'max_aspect': iconMaxAspect,
    };
    if (embeddingIndex != null) {
      icon['embedding_index'] = p.normalize(embeddingIndex!);
    }
    if (visionModel != null) {
      icon['vision_model'] = p.normalize(visionModel!);
    }

    return {
      'run_ocr': runOcr,
      'run_icon': runIcon,
      'layout': {'min_area': minArea},
      'ocr': ocr,
      'icon': icon,
    };
  }
}
