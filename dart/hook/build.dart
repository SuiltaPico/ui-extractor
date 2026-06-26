import 'dart:io';

import 'package:code_assets/code_assets.dart';
import 'package:hooks/hooks.dart';
import 'package:ui_extractor/src/native_release.dart';
import 'package:ui_extractor/src/native_release_fetch.dart';

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

    final repo =
        input.userDefines['release_repo'] as String? ?? defaultReleaseRepo;
    final tag =
        input.userDefines['release_tag'] as String? ?? defaultReleaseTag;
    final localLibUri = input.userDefines.path('local_lib');

    try {
      final libFile = await resolveNativeLibraryFile(
        outputDirectory: Directory.fromUri(input.outputDirectoryShared),
        packageRoot: input.packageRoot,
        targetOS: targetOS,
        targetArchitecture: targetArchitecture,
        repo: repo,
        tag: tag,
        localLib: localLibUri?.toFilePath(),
      );

      output.assets.code.add(
        CodeAsset(
          package: input.packageName,
          name: nativeAssetName,
          linkMode: DynamicLoadingBundled(),
          file: libFile.uri,
        ),
      );

      if (Platform.isLinux || Platform.isMacOS) {
        output.dependencies.add(libFile.uri);
      }
    } on UnsupportedError catch (e) {
      throw UnsupportedError(
        'ui_extractor: ${e.message ?? e}\n'
        'Supported: Windows (x64, arm64), Android (arm64, x64).\n'
        'Use hooks user_defines local_lib, $uiExtractorLibEnv, '
        'or cargo build --release in the ui-extractor repo.',
      );
    } on HttpException catch (e) {
      throw StateError(
        'ui_extractor: failed to download native library (${e.uri}): ${e.message}\n'
        'Build locally in the ui-extractor repo, or set hooks user_defines '
        'local_lib / $uiExtractorLibEnv.',
      );
    }
  });
}
