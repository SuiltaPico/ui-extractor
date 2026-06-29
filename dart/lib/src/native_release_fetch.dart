import 'dart:io';

import 'package:code_assets/code_assets.dart';
import 'package:path/path.dart' as p;
import 'package:ui_extractor/src/native_release.dart';

/// Environment variable for an explicit native library path (CI / local dev).
const String uiExtractorLibEnv = 'LOCAL_UI_EXTRACTOR_LIB';

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
  required Uri packageRoot,
  required OS targetOS,
  required Architecture targetArchitecture,
  required String repo,
  required String tag,
  String? localLib,
}) async {
  if (localLib != null && localLib.isNotEmpty) {
    final file = File(localLib);
    if (!await file.exists()) {
      throw StateError('local_lib not found: $localLib');
    }
    return file;
  }

  final envLib = Platform.environment[uiExtractorLibEnv];
  if (envLib != null && envLib.isNotEmpty) {
    final file = File(envLib);
    if (!await file.exists()) {
      throw StateError('$uiExtractorLibEnv not found: $envLib');
    }
    return file;
  }

  final preinstalledRelative = preinstalledLibraryRelativePath(
    targetOS: targetOS,
    targetArchitecture: targetArchitecture,
  );
  if (preinstalledRelative != null) {
    final preinstalled = File(
      p.join(packageRoot.toFilePath(), preinstalledRelative),
    );
    if (await preinstalled.exists()) {
      return preinstalled;
    }
  }

  final cargoOut = _cargoReleaseLibrary(packageRoot.toFilePath(), targetOS);
  if (cargoOut != null && await File(cargoOut).exists()) {
    return File(cargoOut);
  }

  return fetchNativeLibrary(
    outputDirectory: outputDirectory,
    targetOS: targetOS,
    targetArchitecture: targetArchitecture,
    repo: repo,
    tag: tag,
  );
}

String? _cargoReleaseLibrary(String packageRoot, OS targetOS) {
  final repoRoot = p.normalize(p.join(packageRoot, '..'));
  final fileName = targetOS.dylibFileName(bundledLibraryBaseName(targetOS));
  return p.join(repoRoot, 'target', 'release', fileName);
}
