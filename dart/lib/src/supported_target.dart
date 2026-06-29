import 'package:code_assets/code_assets.dart';

/// Release 产物仅覆盖这些 OS/架构组合；其余 hook 调用直接跳过。
bool isBundledNativeTargetSupported(OS os, Architecture arch) {
  return switch (os) {
    OS.windows => arch == Architecture.x64 || arch == Architecture.arm64,
    OS.android => arch == Architecture.arm64 || arch == Architecture.x64,
    _ => false,
  };
}
