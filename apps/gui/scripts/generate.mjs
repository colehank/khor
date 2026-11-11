import { closeSync, mkdirSync, openSync, renameSync, rmSync } from "node:fs";
import { spawnSync } from "node:child_process";

mkdirSync("src/gen", { recursive: true });
const temporary = `src/gen/catalog-${process.pid}.tmp`;
try {
  const output = openSync(temporary, "w");
  try {
    const catalog = spawnSync("cargo", ["run", "--locked", "-q", "-p", "khor-catalog", "--bin", "gen_ts"], {
      stdio: ["ignore", output, "inherit"],
    });
    if (catalog.error) throw catalog.error;
    if (catalog.status !== 0) throw new Error(`Catalog generation failed (${catalog.status})`);
  } finally {
    closeSync(output);
  }
  const bindings = spawnSync("cargo", ["run", "--locked", "-q", "-p", "khor-gui-core", "--bin", "gen_bindings"], {
    stdio: ["ignore", "inherit", "inherit"],
  });
  if (bindings.error) throw bindings.error;
  if (bindings.status !== 0) throw new Error(`Binding generation failed (${bindings.status})`);
  renameSync(temporary, "src/gen/catalog.ts");
} finally {
  rmSync(temporary, { force: true });
}
