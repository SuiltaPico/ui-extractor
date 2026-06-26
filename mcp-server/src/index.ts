#!/usr/bin/env node
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";

import {
  checkSetup,
  resolveConfig,
  runExtract,
} from "./config.js";

const server = new McpServer({
  name: "ui-extractor",
  version: "0.1.0",
});

server.registerTool(
  "check_setup",
  {
    description:
      "Verify ui-extractor runtime: binary, infer_core.dll, and model packs. " +
      "Call before extract_ui when setup is uncertain.",
    inputSchema: z.object({}),
  },
  async () => {
    const config = resolveConfig();
    const status = checkSetup(config);
    return {
      content: [
        {
          type: "text" as const,
          text: JSON.stringify(
            {
              ok: status.ok,
              config: {
                bin: config.bin,
                modelsDir: config.modelsDir,
                ocrPack: config.ocrPack,
                iconIndexPack: config.iconIndexPack,
                inferCoreDir: config.inferCoreDir,
              },
              checks: {
                binExists: status.binExists,
                inferCore: status.inferCore,
                inferCoreExists: status.inferCoreExists,
                modelsDirExists: status.modelsDirExists,
                ocrPackDir: status.ocrPackDir,
                ocrPackExists: status.ocrPackExists,
                iconIndexPackDir: status.iconIndexPackDir,
                iconIndexPackExists: status.iconIndexPackExists,
              },
              issues: status.issues,
            },
            null,
            2,
          ),
        },
      ],
    };
  },
);

server.registerTool(
  "extract_ui",
  {
    description:
      "Extract UI element tree, OCR text, and icon names from a screenshot. " +
      "Returns JSON with bounds, kind (root|container|text|icon), text content, and icon names. " +
      "Use for UI automation, accessibility analysis, or feeding layout context to text-only LLMs.",
    inputSchema: z.object({
      image_path: z
        .string()
        .describe("Absolute or workspace-relative path to PNG/JPEG/WebP screenshot"),
      layout_only: z
        .boolean()
        .optional()
        .describe("Skip OCR and icon matching; layout tree only"),
      no_icon: z
        .boolean()
        .optional()
        .describe("Skip icon recognition"),
      annotate: z
        .boolean()
        .optional()
        .describe("Write annotated PNG (requires output_path)"),
      output_path: z
        .string()
        .optional()
        .describe("Write JSON to this path instead of stdout"),
      min_area: z
        .number()
        .int()
        .optional()
        .describe("Minimum contour area in pixels (default 100)"),
      min_cosine: z
        .number()
        .optional()
        .describe("Minimum cosine similarity for icon match (default 0.72)"),
      ocr_max_side: z
        .number()
        .int()
        .optional()
        .describe("OCR input long-edge limit; 0 = full resolution (default 960)"),
      models_dir: z
        .string()
        .optional()
        .describe("Override models root (default: UI_EXTRACTOR_MODELS_DIR or ./models)"),
      ocr_pack: z.string().optional().describe("OCR pack id"),
      icon_index_pack: z.string().optional().describe("Icon index pack id"),
    }),
  },
  async (args) => {
    try {
      const result = runExtract({
        inputPath: args.image_path,
        layoutOnly: args.layout_only,
        noIcon: args.no_icon,
        annotate: args.annotate,
        outputPath: args.output_path,
        minArea: args.min_area,
        minCosine: args.min_cosine,
        ocrMaxSide: args.ocr_max_side,
        modelsDir: args.models_dir,
        ocrPack: args.ocr_pack,
        iconIndexPack: args.icon_index_pack,
      });

      const payload: Record<string, unknown> = {
        result: result.json,
      };
      if (result.outputPath) payload.output_path = result.outputPath;
      if (result.annotatedPath) payload.annotated_path = result.annotatedPath;
      if (result.stderr.trim()) payload.logs = result.stderr.trim();

      return {
        content: [
          {
            type: "text" as const,
            text: JSON.stringify(payload, null, 2),
          },
        ],
      };
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      return {
        isError: true,
        content: [{ type: "text" as const, text: message }],
      };
    }
  },
);

server.registerPrompt(
  "analyze_ui",
  {
    description:
      "Analyze a screenshot's UI structure for automation or accessibility review",
    argsSchema: {
      image_path: z.string().describe("Path to the screenshot"),
      goal: z
        .string()
        .optional()
        .describe("What the user wants to do on this screen"),
    },
  },
  async ({ image_path, goal }) => ({
    messages: [
      {
        role: "user" as const,
        content: {
          type: "text" as const,
          text: [
            "Use the extract_ui tool on this screenshot, then analyze the UI tree.",
            "",
            `Screenshot: ${image_path}`,
            goal ? `Goal: ${goal}` : "",
            "",
            "After extraction, summarize:",
            "1. Main interactive elements (buttons, inputs, tabs) with bounds",
            "2. Visible text labels and their positions",
            "3. Recognized icons and likely actions",
            "4. Suggested click targets to achieve the goal",
          ]
            .filter(Boolean)
            .join("\n"),
        },
      },
    ],
  }),
);

async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
