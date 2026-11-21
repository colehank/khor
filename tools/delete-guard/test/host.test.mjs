import assert from 'node:assert/strict';
import test from 'node:test';
import { apply } from '../host.mjs';

function harness() {
  let handler;
  apply({ on(event, fn) { assert.equal(event, 'tools/pre-execute'); handler = fn; } });
  return handler;
}

test('denies the incident command before dispatch', async () => {
  const handler = harness();
  let ran = false;
  const decision = await handler(
    { name: 'pwsh', arguments: { command: 'Remove-Item $home -Recurse -Force -ErrorAction SilentlyContinue' } },
    async () => { ran = true; return { kind: 'allow' }; },
  );
  assert.equal(decision.kind, 'deny');
  assert.equal(ran, false);
});

test('passes an allowed command through next exactly once', async () => {
  const handler = harness();
  let called = 0;
  const decision = await handler(
    { name: 'pwsh', arguments: { command: 'Get-ChildItem F:\\#Gong' } },
    async () => { called += 1; return { kind: 'allow' }; },
  );
  assert.equal(decision.kind, 'allow');
  assert.equal(called, 1);
});
