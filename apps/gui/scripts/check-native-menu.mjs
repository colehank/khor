// Exercise the actual cross-platform Rust lease module without linking Tauri
// or touching user app data. Deliberately optional: npm test stays Cargo-free.
import assert from 'node:assert/strict';
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawn } from 'node:child_process';

const gui = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const repo = resolve(gui, '..', '..');
const protocol = join(gui, 'src-tauri/src/menu_overlay.rs');
const lock = readFileSync(join(repo, 'Cargo.lock'), 'utf8');
const version = name => {
  const block = lock.split('[[package]]').find(block => new RegExp(`^name = "${name}"$`, 'm').test(block));
  const value = block?.match(/^version = "([^"]+)"$/m)?.[1];
  assert(value, `Cargo.lock must pin ${name}`);
  return value;
};
// Reading the production module here also makes a missing or renamed module
// fail before creating build artifacts; the harness imports it by path.
assert(readFileSync(protocol, 'utf8').includes('fn stale_callbacks_cannot_act_on_the_new_owner'));
const base = process.env.KHOR_GATE_TMP || tmpdir();
mkdirSync(base, { recursive: true });
const scratch = mkdtempSync(join(base, 'khor-native-menu-'));
try {
  mkdirSync(join(scratch, 'src'));
  writeFileSync(join(scratch, 'Cargo.toml'), `[package]
name = "khor-native-menu-protocol-gate"
version = "0.0.0"
edition = "2024"

[dependencies]
serde = { version = "=${version('serde')}", features = ["derive"] }
serde_json = "=${version('serde_json')}"

[workspace]
`);
  copyFileSync(join(repo, 'Cargo.lock'), join(scratch, 'Cargo.lock'));
  writeFileSync(join(scratch, 'src/lib.rs'), `#![allow(unexpected_cfgs, dead_code)]
#[path = ${JSON.stringify(protocol)}]
mod menu_overlay;
`);
  await new Promise((accept, reject) => {
    const child = spawn('cargo', ['test', '--offline', '--manifest-path', join(scratch, 'Cargo.toml'), '--target-dir', join(scratch, 'target'), '--lib', '--', '--test-threads=1'], {
      cwd: repo, stdio: 'inherit', env: process.env,
    });
    const timeout = setTimeout(() => child.kill('SIGTERM'), 120_000);
    child.once('error', error => { clearTimeout(timeout); reject(error); });
    child.once('close', (code, signal) => {
      clearTimeout(timeout);
      if (code === 0) accept();
      else reject(new Error(`native menu protocol failed (${signal || code}); no native rendering acceptance is implied`));
    });
  });
  console.log('native-menu: actual Rust protocol passed (leases, owner isolation, stale callbacks, row membership, finite geometry); native stacking/focus still needs platform acceptance');
} finally {
  rmSync(scratch, { recursive: true, force: true });
}
