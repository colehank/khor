import { existsSync, mkdirSync, renameSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
const targets = {
  "x86_64-unknown-linux-gnu": "bun-linux-x64-baseline",
  "aarch64-unknown-linux-gnu": "bun-linux-arm64",
  "x86_64-unknown-linux-musl": "bun-linux-x64-baseline-musl",
  "aarch64-unknown-linux-musl": "bun-linux-arm64-musl",
  "x86_64-apple-darwin": "bun-darwin-x64",
  "aarch64-apple-darwin": "bun-darwin-arm64",
  "x86_64-pc-windows-msvc": "bun-windows-x64-baseline",
};
const requested = process.argv[2];
if (requested && !targets[requested]) throw new Error(`Unsupported khor agent target: ${requested}`);
const cwd = fileURLToPath(new URL(".", import.meta.url));
const output = process.argv[3] ? resolve(process.argv[3]) : fileURLToPath(new URL("dist/khor-agent", import.meta.url));
mkdirSync(dirname(output), { recursive: true });
const catalog = spawnSync(process.execPath, ["scripts/providers.ts"], { cwd, stdio: "inherit" });
if (catalog.error) throw catalog.error;
if (catalog.status !== 0) throw new Error("Failed to generate Pi provider catalog");
// Bun 1.3.x extracts compile targets next to the project, then renames them
// into its cache. Windows cannot rename that file across drive letters (#11198).
// Keep this build's cache on the project volume without moving the user's cache.
let compileEnv;
if (process.platform === "win32" && !process.env.BUN_INSTALL_CACHE_DIR) {
  const cache = join(cwd, "dist", ".bun-compile-cache");
  mkdirSync(cache, { recursive: true });
  compileEnv = { ...process.env, BUN_INSTALL_CACHE_DIR: cache };
}
const result = spawnSync("bun", ["build", "--compile", "--no-compile-autoload-bunfig", "--no-compile-autoload-dotenv", `--target=${requested ? targets[requested] : "bun"}`, "--outfile", output, "src/host.ts"], { cwd, stdio: "inherit", env: compileEnv });
if (result.error) throw result.error;
if (result.status === 0 && !existsSync(output) && existsSync(output + ".exe")) {
  renameSync(output + ".exe", output);
}
if (result.status === 0 && !existsSync(output)) throw new Error("Compiled Pi runtime is missing");
process.exitCode = result.status ?? 1;
