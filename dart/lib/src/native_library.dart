import 'dart:ffi';
import 'dart:io';

import 'ffi_native.dart';

export 'ffi_native.dart' show nativeAssetId;

/// Thrown when FFI is used before the native library is available.
class UiExtractorLibraryNotInitialized implements Exception {
  UiExtractorLibraryNotInitialized(this.message);

  final String message;

  @override
  String toString() => message;
}

enum _NativeLibraryKind {
  uninitialized,
  explicitPath,
  androidPlugin,
  bundledAsset,
}

_NativeLibraryKind _kind = _NativeLibraryKind.uninitialized;
DynamicLibrary? _library;

/// Whether FFI resolves symbols via build-hook [@Native] assets (desktop).
bool get usesBundledNativeAsset {
  _ensureResolved();
  return _kind == _NativeLibraryKind.bundledAsset;
}

/// Load `ui_extractor` from an explicit path (`.dll` / `.so` / `.dylib`).
///
/// Takes precedence over bundled assets from the build hook. Call before any
/// engine API when you manage the library path yourself.
void initUiExtractorLibrary(String libraryPath) {
  final file = File(libraryPath);
  if (!file.existsSync()) {
    throw UiExtractorLibraryNotInitialized(
      'native library not found: $libraryPath',
    );
  }
  _library = DynamicLibrary.open(libraryPath);
  _kind = _NativeLibraryKind.explicitPath;
}

/// Whether the native library has been loaded (explicitly or lazily).
bool get isUiExtractorLibraryInitialized =>
    _kind != _NativeLibraryKind.uninitialized;

DynamicLibrary get uiExtractorLibrary {
  _ensureResolved();
  if (_kind == _NativeLibraryKind.bundledAsset) {
    throw UiExtractorLibraryNotInitialized(
      'DynamicLibrary is not used when ui_extractor is loaded via bundled native '
      'assets. Call initUiExtractorLibrary(path) if you need a DynamicLibrary '
      'handle.',
    );
  }
  return _library!;
}

void _ensureResolved() {
  if (_kind != _NativeLibraryKind.uninitialized) {
    return;
  }

  if (_library != null) {
    _kind = _NativeLibraryKind.explicitPath;
    return;
  }

  if (Platform.isAndroid) {
    _library = DynamicLibrary.open('libui_extractor.so');
    _kind = _NativeLibraryKind.androidPlugin;
    return;
  }

  try {
    nativeUiExtractorVersion();
    _kind = _NativeLibraryKind.bundledAsset;
  } on Object {
    throw UiExtractorLibraryNotInitialized(
      'Call initUiExtractorLibrary(path) before using ui_extractor, '
      'or run `flutter pub get` so the build hook can bundle ui_extractor.',
    );
  }
}
