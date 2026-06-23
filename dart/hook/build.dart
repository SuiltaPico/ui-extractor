import 'dart:io';
import 'package:native_assets_cli/native_assets_cli.dart';
import 'package:http/http.dart' as http;
import 'package:archive/archive.dart';
import 'package:path/path.dart' as p;

const String version = '0.1.0';
const String repo = 'SuiltaPico/ui-extractor';

void main(List<String> args) async {
  await build(args, (config, output) async {
    final packageName = config.packageName;
    final targetOS = config.targetOS;
    final targetArch = config.targetArchitecture;

    final assetName = _getAssetName(targetOS, targetArch);
    if (assetName == null) {
      throw UnsupportedError('Unsupported target: $targetOS $targetArch');
    }

    final downloadUrl = 'https://github.com/$repo/releases/download/v$version/$assetName.zip';
    final outDir = config.outputDirectory.toFilePath();
    final zipPath = p.join(outDir, '$assetName.zip');
    final extractDir = p.join(outDir, assetName);

    final libName = _getLibName(targetOS);
    final libPath = p.join(extractDir, libName);

    if (!File(libPath).existsSync()) {
      print('Downloading native assets from $downloadUrl...');
      final response = await http.get(Uri.parse(downloadUrl));
      if (response.statusCode != 200) {
        // Fallback or error
        print('Failed to download from GitHub (Status: ${response.statusCode}).');
        print('If this is a development build, ensure you have built the native library manually.');
        // For now, we throw an error to let the user know.
        // In a real scenario, we might want to check if the file exists locally.
        throw Exception('Failed to download native assets: ${response.reasonPhrase}');
      }

      final bytes = response.bodyBytes;
      final archive = ZipDecoder().decodeBytes(bytes);

      for (final file in archive) {
        final filename = file.name;
        if (file.isFile) {
          final data = file.content as List<int>;
          File(p.join(outDir, assetName, filename))
            ..createSync(recursive: true)
            ..writeAsBytesSync(data);
        } else {
          Directory(p.join(outDir, assetName, filename)).createSync(recursive: true);
        }
      }
    }

    // Register the native library
    output.addAsset(NativeCodeAsset(
      package: packageName,
      name: 'ui_extractor',
      linkMode: DynamicLoadingBundled(),
      os: targetOS,
      architecture: targetArch,
      file: Uri.file(libPath),
    ));

    // Register models as data assets
    final modelsDir = Directory(p.join(extractDir, 'models'));
    if (modelsDir.existsSync()) {
      for (final file in modelsDir.listSync(recursive: true)) {
        if (file is File) {
          final relativePath = p.relative(file.path, from: extractDir);
          output.addAsset(DataAsset(
            package: packageName,
            name: 'assets/$relativePath',
            file: Uri.file(file.path),
          ));
        }
      }
    }
    
    // Register embeddings.bin
    final embeddingsPath = p.join(extractDir, 'assets', 'embeddings.bin');
    if (File(embeddingsPath).existsSync()) {
      output.addAsset(DataAsset(
        package: packageName,
        name: 'assets/embeddings.bin',
        file: Uri.file(embeddingsPath),
      ));
    }
  });
}

String? _getAssetName(OS os, Architecture? arch) {
  if (os == OS.windows) {
    if (arch == Architecture.x64) return 'ui-extractor-windows-x64';
    if (arch == Architecture.arm64) return 'ui-extractor-windows-arm64';
  } else if (os == OS.android) {
    if (arch == Architecture.arm64) return 'ui-extractor-android-arm64-v8a';
    if (arch == Architecture.x64) return 'ui-extractor-android-x86_64';
  }
  return null;
}

String _getLibName(OS os) {
  if (os == OS.windows) return 'ui_extractor.dll';
  if (os == OS.android) return 'libui_extractor.so';
  return 'libui_extractor.so';
}
