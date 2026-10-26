import { mkdirSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
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
const result = spawnSync("bun", ["build", "--compile", "--no-compile-autoload-bunfig", "--no-compile-autoload-dotenv", `--target=${requested ? targets[requested] : "bun"}`, "--outfile", output, "src/host.ts"], { cwd, stdio: "inherit" });
if (result.error) throw result.error;
process.exitCode = result.status ?? 1;
