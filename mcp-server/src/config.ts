import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, "..", "..");

export interface ServerConfig {
  bin: string;
  modelsDir: string;
  ocrPack: string;
  iconIndexPack: string;
  inferCoreDir: string | null;
}

export interface SetupCheck {
  ok: boolean;
  bin: string;
  binExists: boolean;
  inferCore: string | null;
  inferCoreExists: boolean;
  modelsDir: string;
  modelsDirExists: boolean;
  ocrPackDir: string;
  ocrPackExists: boolean;
  iconIndexPackDir: string;
  iconIndexPackExists: boolean;
  issues: string[];
}

export interface ExtractOptions {
  inputPath: string;
  layoutOnly?: boolean;
  noIcon?: boolean;
  annotate?: boolean;
  outputPath?: string;
  minArea?: number;
  minCosine?: number;
  ocrMaxSide?: number;
  modelsDir?: string;
  ocrPack?: string;
  iconIndexPack?: string;
}

export interface ExtractResult {
  json: unknown;
  stdout: string;
  stderr: string;
  annotatedPath?: string;
  outputPath?: string;
}

function firstExisting(paths: string[]): string | null {
  for (const p of paths) {
    if (existsSync(p)) return p;
  }
  return null;
}

function exeName(name: string): string {
  return process.platform === "win32" ? `${name}.exe` : name;
}

export function resolveConfig(): ServerConfig {
  const bin =
    process.env.UI_EXTRACTOR_BIN ??
    firstExisting([
      join(REPO_ROOT, "target", "release", exeName("ui-extractor")),
      join(REPO_ROOT, "target", "debug", exeName("ui-extractor")),
    ]) ??
    "ui-extractor";

  const modelsDir = resolve(
    process.env.UI_EXTRACTOR_MODELS_DIR ??
      process.env.LOCAL_INFER_ROOT ??
      join(REPO_ROOT, "models"),
  );

  const inferCoreDir =
    firstExisting([
      join(
        REPO_ROOT,
        ".infer-core-release",
        "infer-core-windows-x86_64",
        "lib",
      ),
      join(
        REPO_ROOT,
        ".infer-core-release",
        "infer-core-windows-aarch64",
        "lib",
      ),
      dirname(bin),
      join(REPO_ROOT, "target", "release"),
      join(REPO_ROOT, "target", "debug"),
    ]);

  return {
    bin,
    modelsDir,
    ocrPack:
      process.env.UI_EXTRACTOR_OCR_PACK ??
      "ocr.paddle.ppocr6-tiny.onnx.fp32",
    iconIndexPack:
      process.env.UI_EXTRACTOR_ICON_INDEX_PACK ??
      "icons.bundled.v1.mobileclip2-s0.int8",
    inferCoreDir,
  };
}

export function checkSetup(config: ServerConfig = resolveConfig()): SetupCheck {
  const issues: string[] = [];
  const binExists = existsSync(config.bin);
  if (!binExists) {
    issues.push(
      `ui-extractor binary not found: ${config.bin}. Build with: cargo build --release`,
    );
  }

  const inferCoreName =
    process.platform === "win32" ? "infer_core.dll" : "libinfer_core.so";
  const inferCore = config.inferCoreDir
    ? join(config.inferCoreDir, inferCoreName)
    : null;
  const inferCoreExists = inferCore ? existsSync(inferCore) : false;
  if (!inferCoreExists) {
    issues.push(
      `${inferCoreName} not found near ${config.inferCoreDir ?? "(unknown)"}. ` +
        "Run scripts/download_infer_core_release.ps1 and scripts/build.ps1 (copies infer_core.dll next to the binary).",
    );
  }

  const modelsDirExists = existsSync(config.modelsDir);
  if (!modelsDirExists) {
    issues.push(
      `models directory missing: ${config.modelsDir}. Run scripts/install_packs.ps1`,
    );
  }

  const ocrPackDir = join(config.modelsDir, config.ocrPack);
  const ocrPackExists = existsSync(ocrPackDir);
  if (!ocrPackExists) {
    issues.push(`OCR pack missing: ${ocrPackDir}`);
  }

  const iconIndexPackDir = join(config.modelsDir, config.iconIndexPack);
  const iconIndexPackExists = existsSync(iconIndexPackDir);
  if (!iconIndexPackExists) {
    issues.push(`Icon index pack missing: ${iconIndexPackDir}`);
  }

  return {
    ok: issues.length === 0,
    bin: config.bin,
    binExists,
    inferCore,
    inferCoreExists,
    modelsDir: config.modelsDir,
    modelsDirExists,
    ocrPackDir,
    ocrPackExists,
    iconIndexPackDir,
    iconIndexPackExists,
    issues,
  };
}

function buildPathEnv(config: ServerConfig): string {
  const parts = new Set<string>();
  const sep = process.platform === "win32" ? ";" : ":";

  if (config.inferCoreDir) parts.add(config.inferCoreDir);
  parts.add(dirname(config.bin));

  for (const p of (process.env.PATH ?? "").split(sep)) {
    if (p) parts.add(p);
  }

  return [...parts].join(sep);
}

function resolveInputPath(inputPath: string): string {
  const p = isAbsolute(inputPath) ? inputPath : resolve(process.cwd(), inputPath);
  if (!existsSync(p)) {
    throw new Error(`Input image not found: ${p}`);
  }
  return p;
}

export function runExtract(
  options: ExtractOptions,
  config: ServerConfig = resolveConfig(),
): ExtractResult {
  const inputPath = resolveInputPath(options.inputPath);
  const modelsDir = resolve(options.modelsDir ?? config.modelsDir);
  const ocrPack = options.ocrPack ?? config.ocrPack;
  const iconIndexPack = options.iconIndexPack ?? config.iconIndexPack;

  const args = [
    "extract",
    "--input",
    inputPath,
    "--models-dir",
    modelsDir,
    "--ocr-pack",
    ocrPack,
    "--icon-index-pack",
    iconIndexPack,
    "--format",
    "json",
  ];

  if (options.layoutOnly) args.push("--layout-only");
  if (options.noIcon) args.push("--no-icon");
  if (options.minArea !== undefined) {
    args.push("--min-area", String(options.minArea));
  }
  if (options.minCosine !== undefined) {
    args.push("--min-cosine", String(options.minCosine));
  }
  if (options.ocrMaxSide !== undefined) {
    args.push("--ocr-max-side", String(options.ocrMaxSide));
  }

  let outputPath: string | undefined;
  if (options.outputPath) {
    outputPath = isAbsolute(options.outputPath)
      ? options.outputPath
      : resolve(process.cwd(), options.outputPath);
    args.push("--output", outputPath);
  }

  if (options.annotate) args.push("--annotate");

  const env = {
    ...process.env,
    PATH: buildPathEnv(config),
    LOCAL_INFER_ROOT: modelsDir,
  };

  const result = spawnSync(config.bin, args, {
    encoding: "utf8",
    env,
    maxBuffer: 64 * 1024 * 1024,
  });

  if (result.error) {
    throw new Error(`Failed to spawn ui-extractor: ${result.error.message}`);
  }

  const stdout = result.stdout ?? "";
  const stderr = result.stderr ?? "";

  if (result.status !== 0) {
    throw new Error(
      `ui-extractor exited with code ${result.status}\n${stderr || stdout}`,
    );
  }

  const jsonText = outputPath ? undefined : stdout.trim();
  let json: unknown;
  if (jsonText) {
    try {
      json = JSON.parse(jsonText);
    } catch {
      throw new Error(`ui-extractor returned invalid JSON:\n${jsonText.slice(0, 500)}`);
    }
  } else if (outputPath && existsSync(outputPath)) {
    json = JSON.parse(readFileSync(outputPath, "utf8"));
  } else {
    throw new Error("No JSON output from ui-extractor");
  }

  let annotatedPath: string | undefined;
  if (options.annotate && outputPath) {
    const candidate = outputPath.replace(/\.json$/i, ".png");
    if (existsSync(candidate)) annotatedPath = candidate;
  }

  return { json, stdout, stderr, annotatedPath, outputPath };
}
