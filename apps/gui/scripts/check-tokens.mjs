// The token count's spelling (UX-50, the owner 2026-10-08): every place
// the face abbreviates a token count — the usage half's axes, sums, table
// and tooltips, the composer's context reading — goes through
// `src/lib/tokens.ts`, and that says K / M / B, base 1000, in every
// language: never the G a byte size wears, never a myriad.
//
// Claims:
//   1. `tokens` steps 999 → 1.2K → 3.4M → 5.6B, and honours its decimals;
//   2. `tokensShort` reads 82K / 1.2M / 12M / 3B at a glance;
//   3. no unit it can print is G, k or a CJK numeral;
//   4. the usage half and the composer import it rather than keeping a
//      formatter of their own (a second one is how the G came back).
//
// Run: `npm run tokens`; also in `npm test`.
import { build } from "esbuild";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const gui = fileURLToPath(new URL("..", import.meta.url));
const out = await build({
  stdin: { contents: `import * as t from "./src/lib/tokens"; globalThis.lab = t;`, resolveDir: gui, loader: "ts" },
  bundle: true,
  format: "esm",
  platform: "node",
  write: false,
  logLevel: "error",
});
await import(`data:text/javascript;base64,${Buffer.from(out.outputFiles[0].text).toString("base64")}`);
const { tokens, tokensShort, TOKEN_UNITS } = globalThis.lab;

let bad = 0;
const check = (ok, m) => {
  if (!ok) {
    bad++;
    console.error(`tokens gate: ${m}`);
  }
};
const eq = (got, want, what) => check(got === want, `${what} reads ${JSON.stringify(got)}, want ${JSON.stringify(want)}`);

eq(tokens(999), "999", "tokens(999)");
eq(tokens(1_200), "1.2K", "tokens(1200)");
eq(tokens(3_400_000), "3.4M", "tokens(3.4e6)");
eq(tokens(5_600_000_000), "5.6B", "tokens(5.6e9)");
eq(tokens(5_600_000_000, 2), "5.60B", "tokens(5.6e9, 2)");
eq(tokensShort(82_400), "82K", "tokensShort(82400)");
eq(tokensShort(1_200_000), "1.2M", "tokensShort(1.2e6)");
eq(tokensShort(12_000_000), "12M", "tokensShort(1.2e7)");
eq(tokensShort(3_000_000_000), "3B", "tokensShort(3e9)");
check(!TOKEN_UNITS.some((u) => /[Gk]|[一-鿿]/.test(u)), `the units are ${JSON.stringify(TOKEN_UNITS)} — K / M / B, no G, no k, no 万 / 亿`);
for (const f of ["src/views/UsageHalf.tsx", "src/views/Composer.tsx"]) {
  const src = readFileSync(new URL(`../${f}`, import.meta.url), "utf8");
  check(src.includes('from "@/lib/tokens"'), `${f} does not take its token count from src/lib/tokens.ts`);
  check(!/\[\s*"",\s*"k"|"G"\s*[,\]]|\}k`|\}G`/.test(src), `${f} spells a token unit of its own (k or G)`);
}
if (bad) process.exit(1);
console.log("tokens gate: K / M / B, one formatter, the usage half and the composer on it");
