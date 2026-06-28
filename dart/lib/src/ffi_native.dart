import 'dart:ffi';

import 'package:ffi/ffi.dart';

import 'ffi_types.dart';

/// Bundled native asset id (see `hook/build.dart`).
const String nativeAssetId = 'package:ui_extractor/src/native_library.dart';

@Native<UiExtractorVersionFn>(
  assetId: nativeAssetId,
  symbol: 'ui_extractor_version',
  isLeaf: true,
)
external Pointer<Utf8> nativeUiExtractorVersion();

@Native<UiExtractorStringFreeNative>(
  assetId: nativeAssetId,
  symbol: 'ui_extractor_string_free',
)
external void nativeUiExtractorStringFree(Pointer<Utf8> ptr);

@Native<UiExtractorCreateFn>(
  assetId: nativeAssetId,
  symbol: 'ui_extractor_create',
)
external Pointer<Void> nativeUiExtractorCreate(
  Pointer<Void> inferRegistry,
  Pointer<Utf8> configJson,
  Pointer<Pointer<Utf8>> errorOut,
);

@Native<UiExtractorDestroyNative>(
  assetId: nativeAssetId,
  symbol: 'ui_extractor_destroy',
)
external void nativeUiExtractorDestroy(Pointer<Void> handle);

@Native<UiExtractorExtractBytesNative>(
  assetId: nativeAssetId,
  symbol: 'ui_extractor_extract_bytes',
)
external int nativeUiExtractorExtractBytes(
  Pointer<Void> handle,
  Pointer<Uint8> data,
  int len,
  Pointer<Pointer<Utf8>> jsonOut,
  Pointer<Pointer<Utf8>> errorOut,
);

@Native<UiExtractorExtractFileNative>(
  assetId: nativeAssetId,
  symbol: 'ui_extractor_extract_file',
)
external int nativeUiExtractorExtractFile(
  Pointer<Void> handle,
  Pointer<Utf8> path,
  Pointer<Pointer<Utf8>> jsonOut,
  Pointer<Pointer<Utf8>> errorOut,
);
