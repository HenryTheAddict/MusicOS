import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const host = execFileSync("rustc", ["-vV"], { encoding: "utf8" }).match(
  /^host: (.+)$/m,
)[1];
const target =
  process.env.TAURI_ENV_TARGET_TRIPLE || process.env.CARGO_BUILD_TARGET || host;
const debug =
  process.env.TAURI_ENV_DEBUG === "true" || process.argv.includes("--debug");
const args = [
  "build",
  "-p",
  "musicos-init",
  "--bin",
  "musicos-mac",
  "--features",
  "desktop-app,desktop-audio",
];
if (!debug) args.push("--release");
if (target !== host) args.push("--target", target);
execFileSync("cargo", args, { cwd: root, stdio: "inherit" });
const executable = `musicos-mac${target.includes("windows") ? ".exe" : ""}`;
const source = resolve(
  root,
  process.env.CARGO_TARGET_DIR || "target",
  ...(target !== host ? [target] : []),
  debug ? "debug" : "release",
  executable,
);
const folder = resolve(root, "src-tauri/binaries");
mkdirSync(folder, { recursive: true });
copyFileSync(
  source,
  resolve(
    folder,
    `musicos-mac-${target}${target.includes("windows") ? ".exe" : ""}`,
  ),
);
