import { mkdirSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
const targets = {
  "x86_64-unknown-linux-gnu": "bun-linux-x64-baseline",
  "aarch64-unknown-linux-gnu": "bun-linux-arm64",
  "x86_64-apple-darwin": "bun-darwin-x64",
  "aarch64-apple-darwin": "bun-darwin-arm64",
  "x86_64-pc-windows-msvc": "bun-windows-x64-baseline",
};
const requested = process.argv[2];
if (requested && !targets[requested]) throw new Error(`Unsupported khor agent target: ${requested}`);
const cwd = fileURLToPath(new URL(".", import.meta.url));
mkdirSync(new URL("dist", import.meta.url), { recursive: true });
const catalog = spawnSync(process.execPath, ["scripts/providers.ts"], { cwd, stdio: "inherit" });
if (catalog.error) throw catalog.error;
if (catalog.status !== 0) throw new Error("Failed to generate Pi provider catalog");
const result = spawnSync("bun", ["build", "--compile", "--no-compile-autoload-bunfig", "--no-compile-autoload-dotenv", `--target=${requested ? targets[requested] : "bun"}`, "--outfile", "dist/khor-agent", "src/host.ts"], { cwd, stdio: "inherit" });
if (result.error) throw result.error;
process.exitCode = result.status ?? 1;
