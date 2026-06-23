import 'dart:ffi';
import 'dart:typed_data';

import 'assets.dart';
import 'config.dart';
import 'ffi_bindings.dart';

/// Stateful UI extractor backed by the native library.
///
/// Create once, call [extractBytes] / [extractFile] many times, then [dispose].
class UiExtractorEngine {
  UiExtractorEngine._(this._handle);

  Pointer<Void>? _handle;

  /// Create an engine with explicit config (paths must exist on disk).
  factory UiExtractorEngine.create(ExtractorConfig config) {
    final handle = nativeBindings.createHandle(config);
    return UiExtractorEngine._(handle);
  }

  /// Create an engine using bundled assets from the build hook when possible.
  factory UiExtractorEngine.createBundled({ExtractorConfig? config}) {
    return UiExtractorEngine.create(
      BundledAssets.resolveConfig(overrides: config),
    );
  }

  /// Extract UI tree from in-memory image bytes (PNG/JPEG/WebP, etc.).
  Map<String, dynamic> extractBytes(Uint8List imageBytes) {
    final handle = _handle;
    if (handle == null) {
      throw StateError('UiExtractorEngine already disposed');
    }
    return nativeBindings.extractBytes(handle, imageBytes);
  }

  /// Extract UI tree from an image file path.
  Map<String, dynamic> extractFile(String path) {
    final handle = _handle;
    if (handle == null) {
      throw StateError('UiExtractorEngine already disposed');
    }
    return nativeBindings.extractFile(handle, path);
  }

  void dispose() {
    final handle = _handle;
    if (handle == null) return;
    nativeBindings.destroyHandle(handle);
    _handle = null;
  }
}
