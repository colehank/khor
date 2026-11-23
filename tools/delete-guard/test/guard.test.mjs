import assert from 'node:assert/strict';
import test from 'node:test';
import { decide } from '../guard.mjs';

const pwsh = (command) => decide('pwsh', { command });

test('refuses the incident command', () => {
  const result = pwsh('Remove-Item $home -Recurse -Force -ErrorAction SilentlyContinue');
  assert.equal(result.allow, false);
  assert.match(result.reason, /system drive|user profile/);
});

test('refuses a literal system path delete', () => {
  assert.equal(pwsh('Remove-Item C:\\Users\\admin\\x -Recurse').allow, false);
});

test('refuses interpreter forms naming the system drive', () => {
  assert.equal(pwsh('python -c "import shutil;shutil.rmtree(r\'C:\\x\')"').allow, false);
  assert.equal(pwsh('cmd /c rd /s /q C:\\Users\\admin').allow, false);
  assert.equal(pwsh('node -e "require(\'fs\').rmSync(\'C:/x\', {recursive:true})"').allow, false);
});

test('refuses a chained read-only command', () => {
  assert.equal(pwsh('Get-ChildItem C:\\x; Remove-Item C:\\x -Recurse').allow, false);
});

test('allows a read-only command on the system drive', () => {
  assert.equal(pwsh('Get-Content C:\\Users\\admin\\.dsh\\sessions\\a.json').allow, true);
});

test('allows deleting a literal path on another drive', () => {
  assert.equal(pwsh('Remove-Item F:\\khor-target\\tmp -Recurse -Force').allow, true);
});

test('allows a plain relative path inside the working directory', () => {
  assert.equal(pwsh('Remove-Item .\\build -Recurse -Force').allow, true);
});

test('refuses a variable operand outside the system drive', () => {
  const result = pwsh('Remove-Item $dir -Recurse -Force');
  assert.equal(result.allow, false);
  assert.match(result.reason, /literal path/);
});

test('refuses walking upward', () => {
  assert.equal(pwsh('Remove-Item ..\\build -Recurse -Force').allow, false);
});

test('refuses robocopy purge from the system drive and allows it elsewhere', () => {
  assert.equal(pwsh('robocopy /mir C:\\src F:\\dst').allow, false);
  assert.equal(pwsh('robocopy /mir F:\\src F:\\dst').allow, true);
});

test('refuses writes to the system drive', () => {
  assert.equal(decide('write', { file_path: 'C:\\Users\\admin\\x.txt' }).allow, false);
  assert.equal(decide('write', { file_path: 'F:\\#Gong\\ok.txt' }).allow, true);
});

test('refuses plugin management but allows listing', () => {
  assert.equal(decide('plugin_manager', { action: 'set_plugin' }).allow, false);
  assert.equal(decide('plugin_manager', { action: 'list_plugins' }).allow, true);
});

test('leaves unrelated tools alone', () => {
  assert.equal(decide('read', { file_path: 'C:\\Users\\admin\\x' }).allow, true);
  assert.equal(decide('grep', { pattern: 'C:' }).allow, true);
});
