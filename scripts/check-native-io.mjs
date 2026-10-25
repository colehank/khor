#!/usr/bin/env node
// Small std-only regression. Native Tauri/core tests are separately required.
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = fileURLToPath(new URL('..', import.meta.url));
const temp = mkdtempSync(join(tmpdir(), 'khor-native-io-'));
try {
  const term = readFileSync(resolve(root, 'apps/gui/core/src/term.rs'), 'utf8');
  const productionFunction = name => {
    const starts = [...term.matchAll(new RegExp(`^fn ${name}\\(`, 'gm'))];
    if (starts.length !== 1) throw new Error(`expected one production ${name} function`);
    const start = starts[0].index;
    const end = term.indexOf('\n}', start);
    if (end < 0) throw new Error(`production ${name} function has no closing brace`);
    return term.slice(start, end + 2);
  };
  const pasteFixture = readFileSync(resolve(root, 'scripts/fixtures/native-paste.rs'), 'utf8')
    .replace('// COMMAND_QUEUE_MODULE', `#[path = ${JSON.stringify(resolve(root, 'apps/gui/core/src/command_queue.rs'))}] mod command_queue;`)
    .replace('// PRODUCTION_PASTE_FUNCTIONS', ['term_paste', 'paste'].map(productionFunction).join('\n\n'));
  const pasteSource = join(temp, 'paste.rs');
  writeFileSync(pasteSource, pasteFixture);
  for (const [cmd, args] of [
    ['rustc', ['--edition=2024', '--test', resolve(root, 'scripts/fixtures/native-io.rs'), '-o', join(temp, 'check')]],
    [join(temp, 'check'), ['--test-threads=1', '--nocapture']],
    ['rustc', ['--edition=2024', '--test', pasteSource, '-o', join(temp, 'paste')]],
    [join(temp, 'paste'), ['--test-threads=1', '--nocapture']],
  ]) {
    const result = spawnSync(cmd, args, { cwd: root, stdio: 'inherit', timeout: 30_000 });
    if (result.error) throw result.error;
    if (result.status !== 0) throw new Error(`${cmd} exited ${result.status}`);
  }
} finally {
  rmSync(temp, { recursive: true, force: true });
}
