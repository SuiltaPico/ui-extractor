import 'dart:ffi';

import 'package:ffi/ffi.dart';

typedef UiExtractorVersionFn = Pointer<Utf8> Function();

typedef UiExtractorStringFreeNative = Void Function(Pointer<Utf8>);
typedef UiExtractorStringFreeFn = void Function(Pointer<Utf8>);

typedef UiExtractorCreateFn = Pointer<Void> Function(
  Pointer<Void>,
  Pointer<Utf8>,
  Pointer<Pointer<Utf8>>,
);

typedef UiExtractorDestroyNative = Void Function(Pointer<Void>);
typedef UiExtractorDestroyFn = void Function(Pointer<Void>);

typedef UiExtractorExtractBytesNative = Int32 Function(
  Pointer<Void>,
  Pointer<Uint8>,
  IntPtr,
  Pointer<Pointer<Utf8>>,
  Pointer<Pointer<Utf8>>,
);
typedef UiExtractorExtractBytesFn = int Function(
  Pointer<Void>,
  Pointer<Uint8>,
  int,
  Pointer<Pointer<Utf8>>,
  Pointer<Pointer<Utf8>>,
);

typedef UiExtractorExtractFileNative = Int32 Function(
  Pointer<Void>,
  Pointer<Utf8>,
  Pointer<Pointer<Utf8>>,
  Pointer<Pointer<Utf8>>,
);
typedef UiExtractorExtractFileFn = int Function(
  Pointer<Void>,
  Pointer<Utf8>,
  Pointer<Pointer<Utf8>>,
  Pointer<Pointer<Utf8>>,
);
