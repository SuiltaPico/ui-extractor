import 'dart:convert';
import 'dart:ffi';
import 'dart:io';

import 'package:path/path.dart' as p;
import 'package:ui_extractor/src/native_release.dart';
import 'package:ui_extractor/src/native_release_fetch.dart';

/// Bundled native asset id (see `hook/build.dart`).
const String nativeAssetId = 'package:ui_extractor/src/native_library.dart';

DynamicLibrary openUiExtractorLibrary() {
  final override = Platform.environment[uiExtractorLibEnv];
  if (override != null && override.isNotEmpty) {
    return DynamicLibrary.open(override);
  }

  if (Platform.isAndroid) {
    return DynamicLibrary.open('libui_extractor.so');
  }

  if (_tryOpenBundledLibrary() case final lib?) {
    return lib;
  }

  final path = resolveNativeLibraryPath();
  if (!File(path).existsSync()) {
    throw StateError(
      'ui_extractor native library not found at:\n  $path\n'
      'Run `dart pub get` (build hook), build the Rust crate, '
      'or set $uiExtractorLibEnv.',
    );
  }
  return DynamicLibrary.open(path);
}

DynamicLibrary? _tryOpenBundledLibrary() {
  try {
    _uiExtractorVersionSymbol();
    final lib = DynamicLibrary.process();
    if (lib.providesSymbol('ui_extractor_version')) {
      return lib;
    }
  } on Object {
    // No bundled asset for this target.
  }
  return null;
}

@Native<Pointer<Utf8> Function()>(
  assetId: nativeAssetId,
  symbol: 'ui_extractor_version',
  isLeaf: true,
)
external Pointer<Utf8> _uiExtractorVersionSymbol();

String resolveNativeLibraryPath() {
  if (Platform.isAndroid) {
    return 'libui_extractor.so';
  }

  final packageRoot = _packageRoot();
  if (Platform.isWindows) {
    return p.join(
      packageRoot,
      'native',
      'windows',
      _windowsArch(),
      'lib',
      'ui_extractor.dll',
    );
  }
  throw UnsupportedError(
    'ui_extractor: unsupported platform ${Platform.operatingSystem}',
  );
}

String _packageRoot() {
  final fromConfig = _packageRootFromPackageConfig();
  if (fromConfig != null) {
    return fromConfig;
  }

  final fromCwd = _packageRootFromCwdPubspec();
  if (fromCwd != null) {
    return fromCwd;
  }

  throw StateError(
    'ui_extractor: cannot locate package root; run from a project with '
    'ui_extractor in pubspec, or set $uiExtractorLibEnv',
  );
}

String? _packageRootFromPackageConfig() {
  final explicit = Platform.packageConfig;
  if (explicit != null) {
    final root = _uiExtractorRootFromConfig(File(explicit));
    if (root != null) {
      return root;
    }
  }

  var dir = Directory.current;
  while (true) {
    final configFile = File(p.join(dir.path, '.dart_tool', 'package_config.json'));
    if (configFile.existsSync()) {
      final root = _uiExtractorRootFromConfig(configFile);
      if (root != null) {
        return root;
      }
    }
    final parent = dir.parent;
    if (parent.path == dir.path) {
      break;
    }
    dir = parent;
  }
  return null;
}

String? _uiExtractorRootFromConfig(File configFile) {
  try {
    final json =
        jsonDecode(configFile.readAsStringSync()) as Map<String, dynamic>;
    final packages = json['packages'] as List<dynamic>?;
    if (packages == null) {
      return null;
    }
    for (final pkg in packages) {
      final map = pkg as Map<String, dynamic>;
      if (map['name'] != 'ui_extractor') {
        continue;
      }
      final rootUri = map['rootUri'] as String;
      final configDir = p.dirname(configFile.path);
      final root = rootUri.startsWith('file:')
          ? p.fromUri(Uri.parse(rootUri))
          : p.normalize(p.join(configDir, rootUri));
      if (_isPackageRoot(root)) {
        return root;
      }
    }
  } on Object {
    return null;
  }
  return null;
}

String? _packageRootFromCwdPubspec() {
  var dir = Directory.current;
  while (true) {
    if (_isPackageRoot(dir.path)) {
      return dir.path;
    }
    final parent = dir.parent;
    if (parent.path == dir.path) {
      break;
    }
    dir = parent;
  }
  return null;
}

bool _isPackageRoot(String dir) {
  final pubspec = File(p.join(dir, 'pubspec.yaml'));
  if (!pubspec.existsSync()) {
    return false;
  }
  return pubspec.readAsStringSync().contains('name: ui_extractor');
}

String _windowsArch() =>
    Platform.version.contains('arm64') ? 'arm64' : 'x64';
