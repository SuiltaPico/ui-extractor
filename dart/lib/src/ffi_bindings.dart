import 'dart:convert';
import 'dart:ffi';
import 'dart:typed_data';

import 'package:ffi/ffi.dart';

import 'config.dart';
import 'exceptions.dart';
import 'native_library.dart';

final class _Bindings {
  _Bindings._();

  static final _Bindings instance = _Bindings._();

  late final DynamicLibrary _lib = openUiExtractorLibrary();

  late final Pointer<Utf8> Function() _version =
      _lib.lookupFunction<Pointer<Utf8> Function(), Pointer<Utf8> Function()>(
    'ui_extractor_version',
  );

  late final void Function(Pointer<Utf8>) _stringFree =
      _lib.lookupFunction<Void Function(Pointer<Utf8>), void Function(Pointer<Utf8>)>(
    'ui_extractor_string_free',
  );

  late final Pointer<Void> Function(Pointer<Utf8>, Pointer<Pointer<Utf8>>)
      _create = _lib.lookupFunction<
          Pointer<Void> Function(Pointer<Utf8>, Pointer<Pointer<Utf8>>),
          Pointer<Void> Function(Pointer<Utf8>, Pointer<Pointer<Utf8>>)>(
    'ui_extractor_create',
  );

  late final void Function(Pointer<Void>) _destroy =
      _lib.lookupFunction<Void Function(Pointer<Void>), void Function(Pointer<Void>)>(
    'ui_extractor_destroy',
  );

  late final int Function(
    Pointer<Void>,
    Pointer<Uint8>,
    int,
    Pointer<Pointer<Utf8>>,
    Pointer<Pointer<Utf8>>,
  ) _extractBytes = _lib.lookupFunction<
      Int32 Function(
        Pointer<Void>,
        Pointer<Uint8>,
        IntPtr,
        Pointer<Pointer<Utf8>>,
        Pointer<Pointer<Utf8>>,
      ),
      int Function(
        Pointer<Void>,
        Pointer<Uint8>,
        int,
        Pointer<Pointer<Utf8>>,
        Pointer<Pointer<Utf8>>,
      )>(
    'ui_extractor_extract_bytes',
  );

  late final int Function(
    Pointer<Void>,
    Pointer<Utf8>,
    Pointer<Pointer<Utf8>>,
    Pointer<Pointer<Utf8>>,
  ) _extractFile = _lib.lookupFunction<
      Int32 Function(
        Pointer<Void>,
        Pointer<Utf8>,
        Pointer<Pointer<Utf8>>,
        Pointer<Pointer<Utf8>>,
      ),
      int Function(
        Pointer<Void>,
        Pointer<Utf8>,
        Pointer<Pointer<Utf8>>,
        Pointer<Pointer<Utf8>>,
      )>(
    'ui_extractor_extract_file',
  );

  String get version => _version().toDartString();

  Pointer<Void> createHandle(ExtractorConfig config) {
    final configJson = jsonEncode(config.toJson());
    final configPtr = configJson.toNativeUtf8();
    final errorPtr = calloc<Pointer<Utf8>>();
    try {
      final handle = _create(configPtr, errorPtr);
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
