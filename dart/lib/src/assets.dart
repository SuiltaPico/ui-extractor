import 'dart:io';

import 'package:path/path.dart' as p;

import 'config.dart';

/// Locate bundled models shipped by the native-assets build hook.
class BundledAssets {
  /// Search common hook output locations, then fall back to [ExtractorConfig.defaults].
  static ExtractorConfig resolveConfig({ExtractorConfig? overrides}) {
    final root = findAssetRoot();
    final base = root != null
        ? ExtractorConfig.fromAssetRoot(root)
        : ExtractorConfig.defaults();
    if (overrides == null) return base;

    return ExtractorConfig(
      runOcr: overrides.runOcr,
      runIcon: overrides.runIcon,
      minArea: overrides.minArea,
      modelDir: overrides.modelDir ?? base.modelDir,
      ocrMaxSide: overrides.ocrMaxSide,
      ocrMinConfidence: overrides.ocrMinConfidence,
      embeddingIndex: overrides.embeddingIndex ?? base.embeddingIndex,
      visionModel: overrides.visionModel ?? base.visionModel,
      templateSize: overrides.templateSize,
      minCosine: overrides.minCosine,
      iconMinSide: overrides.iconMinSide,
      iconMaxSide: overrides.iconMaxSide,
      iconMinAspect: overrides.iconMinAspect,
      iconMaxAspect: overrides.iconMaxAspect,
    );
  }

  /// Returns a directory containing `models/` and `assets/embeddings.bin`, or null.
  static String? findAssetRoot() {
    for (final candidate in _candidateRoots()) {
      if (_looksLikeAssetRoot(candidate)) {
        return p.normalize(candidate);
      }
    }
    return null;
  }

  static Iterable<String> _candidateRoots() sync* {
    final cwd = Directory.current.path;
    yield cwd;

    final hooksRunner = p.join(cwd, '.dart_tool', 'hooks_runner');
    if (Directory(hooksRunner).existsSync()) {
      for (final entity in Directory(hooksRunner).listSync(recursive: true)) {
        if (entity is! Directory) continue;
        final name = p.basename(entity.path);
        if (name.startsWith('ui-extractor-')) {
          yield entity.path;
        }
      }
    }
  }

  static bool _looksLikeAssetRoot(String root) {
    final models = Directory(p.join(root, 'models'));
    final embeddings = File(p.join(root, 'assets', 'embeddings.bin'));
    return models.existsSync() && embeddings.existsSync();
  }
}
