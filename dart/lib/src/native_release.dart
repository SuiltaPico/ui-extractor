import 'package:code_assets/code_assets.dart';

const String defaultReleaseRepo = 'SuiltaPico/ui-extractor';
const String defaultReleaseTag = '0.1.0';

String nativeAssetBaseName({
  required OS targetOS,
  required Architecture targetArchitecture,
}) {
  if (targetOS == OS.android) {
    final abi = androidJniAbi(targetArchitecture);
    return 'ui-extractor-android-$abi';
  }
  final label = switch (targetArchitecture) {
    Architecture.x64 => 'x64',
    Architecture.arm64 => 'arm64',
    _ => throw UnsupportedError(
        'unsupported desktop architecture: ${targetArchitecture.name}',
      ),
  };
  return 'ui-extractor-windows-$label';
}

String androidJniAbi(Architecture architecture) => switch (architecture) {
      Architecture.arm64 => 'arm64-v8a',
      Architecture.arm => 'armeabi-v7a',
      Architecture.ia32 => 'x86',
      Architecture.x64 => 'x86_64',
      _ => throw UnsupportedError(
          'unsupported Android architecture: ${architecture.name}',
        ),
    };

String bundledLibraryBaseName(OS targetOS) => 'ui_extractor';

String desktopLibraryRelativePath({
  required OS targetOS,
  required Architecture targetArchitecture,
}) {
  final platform = switch (targetOS) {
    OS.windows => 'windows',
    _ => throw UnsupportedError('unsupported target OS: ${targetOS.name}'),
  };
  final archFolder = switch (targetArchitecture) {
    Architecture.x64 => 'x64',
    Architecture.arm64 => 'arm64',
    _ => throw UnsupportedError(
        'unsupported desktop architecture: ${targetArchitecture.name}',
      ),
  };
  final fileName = targetOS.dylibFileName(bundledLibraryBaseName(targetOS));
  return 'native/$platform/$archFolder/lib/$fileName';
}

String androidLibraryRelativePath(Architecture targetArchitecture) {
  final abi = androidJniAbi(targetArchitecture);
  return 'jniLibs/$abi/libui_extractor.so';
}

Uri releaseArchiveUrl({
  required String repo,
  required String tag,
  required String assetBaseName,
}) {
  final vTag = tag.startsWith('v') ? tag : 'v$tag';
  return Uri.https(
    'github.com',
    '/$repo/releases/download/$vTag/$assetBaseName.zip',
  );
}

String? preinstalledLibraryRelativePath({
  required OS targetOS,
  required Architecture targetArchitecture,
}) {
  try {
    if (targetOS == OS.android) {
      return androidLibraryRelativePath(targetArchitecture);
    }
    return desktopLibraryRelativePath(
      targetOS: targetOS,
      targetArchitecture: targetArchitecture,
    );
  } on UnsupportedError {
    return null;
  }
}
