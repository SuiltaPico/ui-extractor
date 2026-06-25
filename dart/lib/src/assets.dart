import 'config.dart';

/// Resolve models directory for the native extractor.
class BundledAssets {
  /// Build config using [LOCAL_INFER_ROOT] or `./models`.
  static ExtractorConfig resolveConfig({ExtractorConfig? overrides}) {
    final base = ExtractorConfig.defaults();
    if (overrides == null) return base;

    return ExtractorConfig(
      runOcr: overrides.runOcr,
      runIcon: overrides.runIcon,
      minArea: overrides.minArea,
      modelsDir: overrides.modelsDir ?? base.modelsDir,
      ocrPack: overrides.ocrPack,
      iconIndexPack: overrides.iconIndexPack,
      ocrMaxSide: overrides.ocrMaxSide,
      ocrMinConfidence: overrides.ocrMinConfidence,
      templateSize: overrides.templateSize,
      minCosine: overrides.minCosine,
      iconMinSide: overrides.iconMinSide,
      iconMaxSide: overrides.iconMaxSide,
      iconMinAspect: overrides.iconMinAspect,
      iconMaxAspect: overrides.iconMaxAspect,
      runtime: overrides.runtime,
    );
  }
}
