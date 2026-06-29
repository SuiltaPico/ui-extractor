import 'dart:convert';
import 'dart:ffi';
import 'dart:typed_data';

import 'package:ffi/ffi.dart';

import 'exceptions.dart';
import 'ffi_native.dart';
import 'ffi_types.dart';
import 'native_library.dart';

final class _Bindings {
  _Bindings._() {
    if (usesBundledNativeAsset) {
      _initBundled();
    } else {
      _initDynamicLibrary(uiExtractorLibrary);
    }
  }

  static final _Bindings instance = _Bindings._();

  late final UiExtractorVersionFn _version;
  late final UiExtractorStringFreeFn _stringFree;
  late final UiExtractorCreateFn _create;
  late final UiExtractorDestroyFn _destroy;
  late final UiExtractorExtractBytesFn _extractBytes;
  late final UiExtractorExtractFileFn _extractFile;

  void _initBundled() {
    _version = nativeUiExtractorVersion;
    _stringFree = nativeUiExtractorStringFree;
    _create = nativeUiExtractorCreate;
    _destroy = nativeUiExtractorDestroy;
    _extractBytes = nativeUiExtractorExtractBytes;
    _extractFile = nativeUiExtractorExtractFile;
  }

  void _initDynamicLibrary(DynamicLibrary lib) {
    _version = lib.lookupFunction<UiExtractorVersionFn, UiExtractorVersionFn>(
      'ui_extractor_version',
    );
    _stringFree = lib.lookupFunction<
        UiExtractorStringFreeNative, UiExtractorStringFreeFn>(
      'ui_extractor_string_free',
    );
    _create = lib.lookupFunction<UiExtractorCreateFn, UiExtractorCreateFn>(
      'ui_extractor_create',
    );
    _destroy = lib.lookupFunction<
        UiExtractorDestroyNative, UiExtractorDestroyFn>(
      'ui_extractor_destroy',
    );
    _extractBytes = lib.lookupFunction<
        UiExtractorExtractBytesNative, UiExtractorExtractBytesFn>(
      'ui_extractor_extract_bytes',
    );
    _extractFile = lib.lookupFunction<
        UiExtractorExtractFileNative, UiExtractorExtractFileFn>(
      'ui_extractor_extract_file',
    );
  }

  String get version => _version().toDartString();

  Pointer<Void> createHandle({
    required Pointer<Void>? inferRegistry,
    required Map<String, dynamic> config,
  }) {
    final configJson = jsonEncode(config);
    final configPtr = configJson.toNativeUtf8();
    final errorPtr = calloc<Pointer<Utf8>>();
    try {
      final handle = _create(inferRegistry ?? nullptr, configPtr, errorPtr);
      if (handle == nullptr) {
        throw UiExtractorException(_takeOwnedString(errorPtr.value));
      }
      return handle;
    } finally {
      calloc.free(configPtr);
      calloc.free(errorPtr);
    }
  }

  void destroyHandle(Pointer<Void> handle) {
    _destroy(handle);
  }

  Map<String, dynamic> extractBytes(Pointer<Void> handle, Uint8List bytes) {
    final dataPtr = calloc<Uint8>(bytes.length);
    final jsonPtr = calloc<Pointer<Utf8>>();
    final errorPtr = calloc<Pointer<Utf8>>();
    try {
      dataPtr.asTypedList(bytes.length).setAll(0, bytes);
      final rc = _extractBytes(handle, dataPtr, bytes.length, jsonPtr, errorPtr);
      if (rc != 0) {
        throw UiExtractorException(_takeOwnedString(errorPtr.value));
      }
      return _decodeJson(_takeOwnedString(jsonPtr.value));
    } finally {
      calloc.free(dataPtr);
      calloc.free(jsonPtr);
      calloc.free(errorPtr);
    }
  }

  Map<String, dynamic> extractFile(Pointer<Void> handle, String path) {
    final pathPtr = path.toNativeUtf8();
    final jsonPtr = calloc<Pointer<Utf8>>();
    final errorPtr = calloc<Pointer<Utf8>>();
    try {
      final rc = _extractFile(handle, pathPtr, jsonPtr, errorPtr);
      if (rc != 0) {
        throw UiExtractorException(_takeOwnedString(errorPtr.value));
      }
      return _decodeJson(_takeOwnedString(jsonPtr.value));
    } finally {
      calloc.free(pathPtr);
      calloc.free(jsonPtr);
      calloc.free(errorPtr);
    }
  }

  String _takeOwnedString(Pointer<Utf8> ptr) {
    if (ptr == nullptr) {
      return 'unknown native error';
    }
    try {
      return ptr.toDartString();
    } finally {
      _stringFree(ptr);
    }
  }

  Map<String, dynamic> _decodeJson(String json) {
    final decoded = jsonDecode(json);
    if (decoded is! Map<String, dynamic>) {
      throw UiExtractorException('expected JSON object from native library');
    }
    return decoded;
  }
}

/// Low-level FFI access. Prefer [UiExtractorEngine] in `engine.dart`.
final nativeBindings = _Bindings.instance;
