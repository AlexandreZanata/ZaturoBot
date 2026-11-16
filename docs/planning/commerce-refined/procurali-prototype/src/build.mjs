#!/usr/bin/env node
/**
 * Procurali — stable site-builder entrypoint used by the VPS site sync.
 *
 * `deploy/procurali/procurali_site_sync.py` keeps invoking the historical contract
 *
 *   node src/build.mjs --api-snapshot <site-catalog.json> --out-dir <release-dir>
 *
 * while the storefront itself is a Next.js static export. This wrapper:
 *   1. runs scripts/prepare.mjs to validate the snapshot and write .generated/ + public/;
 *   2. runs `next build` into an isolated distDir (the export lands there, not in ./out);
 *   3. copies the exported tree into --out-dir and removes the temporary distDir.
 *
 * Without --api-snapshot (or PROCURALI_API_SNAPSHOT / PROCURALI_HUB_API_URL) the demo
 * fixtures are used, exactly like `npm run build`. Builds are expected to be serialized by
 * the caller (the site sync is a single loop). When the package directory is read-only the
 * build runs in a per-package writable copy under PROCURALI_SITE_WORK_DIR (default: tmpdir).
 */

import { spawnSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const PACKAGE_ROOT = path.resolve(
  path.dirname(fs.realpathSync(fileURLToPath(import.meta.url))),
  ".."
);

export function parseCliArgs(argv, env = process.env) {
  const opts = {
    apiSnapshotPath: env.PROCURALI_API_SNAPSHOT ? path.resolve(env.PROCURALI_API_SNAPSHOT) : null,
    hubApiUrl: env.PROCURALI_HUB_API_URL || null,
    outDir: env.PROCURALI_SITE_DIR ? path.resolve(env.PROCURALI_SITE_DIR) : null,
  };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--api-snapshot" && argv[i + 1]) {
      opts.apiSnapshotPath = path.resolve(argv[(i += 1)]);
    } else if (arg === "--hub-url" && argv[i + 1]) {
      opts.hubApiUrl = argv[(i += 1)];
    } else if (arg === "--out-dir" && argv[i + 1]) {
      opts.outDir = path.resolve(argv[(i += 1)]);
    } else {
      throw new Error(`Unsupported argument: ${arg}`);
    }
  }
  if (!opts.outDir) throw new Error("--out-dir (or PROCURALI_SITE_DIR) is required");
  return opts;
}

function isWritable(dir) {
  try {
    fs.accessSync(dir, fs.constants.W_OK);
    return true;
  } catch {
    return false;
  }
}

/** Returns a writable project root: the package itself or a reusable per-package copy. */
function resolveWorkRoot() {
  if (isWritable(PACKAGE_ROOT)) return PACKAGE_ROOT;
  const base = process.env.PROCURALI_SITE_WORK_DIR || path.join(os.tmpdir(), "procurali-site-builder");
  const key = crypto.createHash("sha256").update(PACKAGE_ROOT).digest("hex").slice(0, 16);
  const workRoot = path.join(base, key);
  const marker = path.join(workRoot, ".procurali-copy-complete");
  if (!fs.existsSync(marker)) {
    fs.rmSync(workRoot, { recursive: true, force: true });
    fs.mkdirSync(base, { recursive: true });
    fs.cpSync(PACKAGE_ROOT, workRoot, { recursive: true });
    fs.chmodSync(workRoot, 0o755);
    fs.writeFileSync(marker, `${PACKAGE_ROOT}\n`, "utf8");
  }
  return workRoot;
}

function runNode(workRoot, args, extraEnv) {
  const result = spawnSync(process.execPath, args, {
    cwd: workRoot,
    env: { ...process.env, NEXT_TELEMETRY_DISABLED: "1", ...extraEnv },
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    const tail = `${result.stdout || ""}\n${result.stderr || ""}`.trim().slice(-4000);
    throw new Error(`node ${path.basename(args[0])} failed with status ${result.status}:\n${tail}`);
  }
  return result.stdout || "";
}

export function buildSite(options) {
  const workRoot = resolveWorkRoot();
  const nextBin = path.join(workRoot, "node_modules", "next", "dist", "bin", "next");
  if (!fs.existsSync(nextBin)) {
    throw new Error(`Next.js is not installed in ${workRoot}; run \`npm ci\` first`);
  }

  const prepareArgs = [path.join(workRoot, "scripts", "prepare.mjs")];
  if (options.apiSnapshotPath) prepareArgs.push("--api-snapshot", options.apiSnapshotPath);
  if (options.hubApiUrl) prepareArgs.push("--hub-url", options.hubApiUrl);
  const prepared = runNode(workRoot, prepareArgs, {});

  const distName = `out-sitebuild-${process.pid}`;
  const distDir = path.join(workRoot, distName);
  fs.rmSync(distDir, { recursive: true, force: true });
  try {
    runNode(workRoot, [nextBin, "build"], { PROCURALI_DIST_DIR: distName });
    if (!fs.existsSync(path.join(distDir, "index.html"))) {
      throw new Error(`next build produced no static export in ${distDir}`);
    }
    fs.rmSync(options.outDir, { recursive: true, force: true });
    fs.mkdirSync(path.dirname(options.outDir), { recursive: true });
    fs.cpSync(distDir, options.outDir, { recursive: true });
  } finally {
    fs.rmSync(distDir, { recursive: true, force: true });
  }
  const summary = prepared.trim().split("\n").pop() || "Prepared Procurali catalog.";
  return `${summary} Exported Next.js site to ${options.outDir}.`;
}

const isEntrypoint =
  Boolean(process.argv[1]) &&
  pathToFileURL(fs.realpathSync(path.resolve(process.argv[1]))).href === import.meta.url;

if (isEntrypoint) {
  try {
    console.log(buildSite(parseCliArgs(process.argv.slice(2))));
  } catch (err) {
    console.error(err instanceof Error ? err.message : err);
    process.exitCode = 1;
  }
}
