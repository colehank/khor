// Static contracts are portable; iOS compilation needs macOS, its SDK and a built face.
// Run: npm run mobile, npm run mobile:static, or npm run mobile:compile.
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { checkMobileSources } from "./check-mobile-static.mjs";

const args = process.argv.slice(2);
if (args.length > 1 || args.some(arg => !["--static-only", "--compile-only"].includes(arg))) {
  console.error("usage: check-mobile.mjs [--static-only | --compile-only]");
  process.exit(2);
}
const gui = fileURLToPath(new URL("..", import.meta.url));
const repo = join(gui, "..", "..");
const target = "aarch64-apple-ios";

if (args[0] !== "--compile-only") {
  const errors = checkMobileSources(gui);
  if (errors.length) {
    errors.forEach(message => console.error(message));
    process.exit(1);
  }
  console.log("mobile: source contracts passed (no compiler run)");
}
if (args[0] === "--static-only") process.exit(0);
if (process.platform !== "darwin") {
  console.error("mobile: iOS compilation needs macOS and Xcode; use mobile:static for source checks");
  process.exit(2);
}
const installed = spawnSync("rustup", ["target", "list", "--installed"], { encoding: "utf8" });
if (installed.status !== 0 || !installed.stdout.split("\n").includes(target)) {
  console.error(`mobile: install the iOS target with rustup target add ${target}; ${installed.stderr || installed.error || ""}`);
  process.exit(1);
}
if (!existsSync(join(gui, "dist", "index.html"))) {
  console.error("mobile: apps/gui/dist/index.html is missing; run npm run build first");
  process.exit(1);
}
const check = spawnSync("cargo", ["check", "--locked", "--target", target, "-p", "khor-gui"], {
  cwd: repo, stdio: "inherit",
});
if (check.status !== 0) {
  console.error(`mobile: iOS compilation failed${check.error ? `: ${check.error.message}` : ""}`);
  process.exit(check.status || 1);
}
console.log(`mobile: khor-gui compiles for ${target} (no runtime acceptance)`);
