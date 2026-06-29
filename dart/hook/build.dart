import 'dart:io';

import 'package:code_assets/code_assets.dart';
import 'package:hooks/hooks.dart';
import 'package:ui_extractor/src/native_release.dart';
import 'package:ui_extractor/src/native_release_fetch.dart';
import 'package:ui_extractor/src/supported_target.dart';

const String nativeAssetName = 'src/native_library.dart';

void main(List<String> args) async {
  await build(args, (input, output) async {
    if (!input.config.buildCodeAssets) {
      return;
    }

    final code = input.config.code;
    final targetOS = code.targetOS;
    final targetArchitecture = code.targetArchitecture;

    if (input.userDefines['skip_download'] == true) {
      return;
    }

    if (!isBundledNativeTargetSupported(targetOS, targetArchitecture)) {
      return;
    }

    final repo =
        input.userDefines['release_repo'] as String? ?? defaultReleaseRepo;
    final tag =
        input.userDefines['release_tag'] as String? ?? defaultReleaseTag;

    try {
      final libFile = await resolveNativeLibraryFile(
        outputDirectory: Directory.fromUri(input.outputDirectoryShared),
        targetOS: targetOS,
        targetArchitecture: targetArchitecture,
        repo: repo,
        tag: tag,
      );

      registerBundledNativeCodeAssets(
        addAsset: output.assets.code.add,
        packageName: input.packageName,
        primaryAssetName: nativeAssetName,
        primaryLib: libFile,
        targetOS: targetOS,
      );

      if (Platform.isLinux || Platform.isMacOS) {
        output.dependencies.add(libFile.uri);
      }
    } on UnsupportedError catch (e) {
      throw UnsupportedError(
        'ui_extractor: ${e.message ?? e}\n'
        'Supported: Windows (x64, arm64), Android (arm64, x64).\n'
        'Native libs are downloaded from GitHub Release (see pubspec hooks user_defines).',
      );
    } on HttpException catch (e) {
      throw StateError(
        'ui_extractor: failed to download native library (${e.uri}): ${e.message}\n'
        'Check network/proxy access to GitHub Releases, or override release_tag in pubspec hooks.',
      );
    }
  });
}
