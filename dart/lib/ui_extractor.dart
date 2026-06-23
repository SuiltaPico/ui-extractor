library ui_extractor;

export 'src/config.dart';
export 'src/engine.dart';
export 'src/exceptions.dart';
export 'src/assets.dart' show BundledAssets;

import 'src/ffi_bindings.dart' show nativeBindings;

/// Entry point for package metadata.
abstract final class UiExtractor {
  /// Native library version string.
  static String get version => nativeBindings.version;
}
