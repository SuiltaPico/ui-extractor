import 'dart:io';

import 'package:code_assets/code_assets.dart';
import 'package:path/path.dart' as p;
import 'package:ui_extractor/src/http_proxy.dart';
import 'package:ui_extractor/src/native_release.dart';

Future<File> fetchNativeLibrary({
  required Directory outputDirectory,
  required OS targetOS,
  required Architecture targetArchitecture,
  required String repo,
  required String tag,
}) async {
  final assetBase = nativeAssetBaseName(
    targetOS: targetOS,
    targetArchitecture: targetArchitecture,
  );
  final url = releaseArchiveUrl(
    repo: repo,
    tag: tag,
    assetBaseName: assetBase,
  );
  const ext = '.zip';
  final archiveFile = File(p.join(outputDirectory.path, '$assetBase$ext'));
  final extractRoot = Directory(p.join(outputDirectory.path, assetBase));

  if (!await extractRoot.exists()) {
    await extractRoot.create(recursive: true);
  }

  final libRelative = targetOS == OS.android
      ? androidLibraryRelativePath(targetArchitecture)
      : p.join(
          'lib',
          targetOS.dylibFileName(bundledLibraryBaseName(targetOS)),
        );

  final libFile = File(p.join(extractRoot.path, libRelative));

  if (!await archiveFile.exists()) {
    await _download(url, archiveFile);
  }

  if (!await libFile.exists()) {
    if (!await archiveFile.exists()) {
      throw StateError('missing release archive for $assetBase');
    }
    await _extractZip(archiveFile: archiveFile, dest: extractRoot);
  }

  if (!await libFile.exists()) {
    throw StateError(
      'expected library at ${libFile.path} after extracting $url',
    );
  }
  return libFile;
}

/// Registers the primary native library only.
///
/// Android release zips also ship infer-core runtime `.so` files for standalone
/// JNI integration, but in Flutter apps those are bundled by `local_infer_core`.
/// Registering siblings here would duplicate filenames across packages.
void registerBundledNativeCodeAssets({
  required void Function(CodeAsset asset) addAsset,
  required String packageName,
  required String primaryAssetName,
  required File primaryLib,
  required OS targetOS,
}) {
  addAsset(
    CodeAsset(
      package: packageName,
      name: primaryAssetName,
      linkMode: DynamicLoadingBundled(),
      file: primaryLib.uri,
    ),
  );
}

Future<void> _download(Uri url, File dest) async {
  final client = HttpClient()
    ..findProxy = (uri) => resolveHttpProxy(uri);
  try {
    final request = await client.getUrl(url);
    final response = await request.close();
    if (response.statusCode != 200) {
      throw HttpException(
        'GET $url failed with status ${response.statusCode}',
        uri: url,
      );
    }
    await dest.parent.create(recursive: true);
    await response.pipe(dest.openWrite());
  } finally {
    client.close(force: true);
  }
}

Future<void> _extractZip({
  required File archiveFile,
  required Directory dest,
}) async {
  if (Platform.isWindows) {
    final result = await Process.run(
      'powershell',
      [
        '-NoProfile',
        '-Command',
        'Expand-Archive -LiteralPath "${archiveFile.path}" -DestinationPath "${dest.path}" -Force',
      ],
      runInShell: true,
    );
    if (result.exitCode != 0) {
      throw StateError('Expand-Archive failed: ${result.stderr}');
    }
    return;
  }

  final result = await Process.run(
    'unzip',
    ['-o', archiveFile.path, '-d', dest.path],
    runInShell: true,
  );
  if (result.exitCode != 0) {
    throw StateError('unzip failed: ${result.stderr}');
  }
}

Future<File> resolveNativeLibraryFile({
  required Directory outputDirectory,
  required OS targetOS,
  required Architecture targetArchitecture,
  required String repo,
  required String tag,
}) async {
  return fetchNativeLibrary(
    outputDirectory: outputDirectory,
    targetOS: targetOS,
    targetArchitecture: targetArchitecture,
    repo: repo,
    tag: tag,
  );
}
