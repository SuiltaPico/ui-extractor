import 'dart:io';

import 'package:code_assets/code_assets.dart';
import 'package:path/path.dart' as p;
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
  final extractRoot = Directory(p.join(outputDirectory.path, assetBase));

  if (!await extractRoot.exists()) {
    await extractRoot.create(recursive: true);
  }

  // Prefer slim zip (ui_extractor.dll only); fall back to full CLI/SDK bundle.
  for (final suffix in ['-slim', '']) {
    final archiveBase = '$assetBase$suffix';
    final url = releaseArchiveUrl(
      repo: repo,
      tag: tag,
      assetBaseName: archiveBase,
    );
    final archiveFile = File(p.join(outputDirectory.path, '$archiveBase.zip'));
    try {
      if (!await archiveFile.exists()) {
        await _download(url, archiveFile);
      }
      await _extractZip(archiveFile: archiveFile, dest: extractRoot);
      break;
    } on HttpException {
      if (suffix.isEmpty) rethrow;
      continue;
    } on StateError {
      if (suffix.isEmpty) rethrow;
      continue;
    }
  }

  final libRelative = targetOS == OS.android
      ? androidLibraryRelativePath(targetArchitecture)
      : targetOS.dylibFileName(bundledLibraryBaseName(targetOS));

  final libFile = File(p.join(extractRoot.path, libRelative));
  if (!await libFile.exists()) {
    throw StateError(
      'expected library at ${libFile.path} after extracting release archives for $assetBase',
    );
  }
  return libFile;
}

Future<void> _download(Uri url, File dest) async {
  final client = HttpClient()..findProxy = HttpClient.findProxyFromEnvironment;
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
