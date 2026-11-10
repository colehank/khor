// Regressions for transmitted light, stored presets and the approved artwork.
import assert from 'node:assert/strict';
import test from 'node:test';
import fs from 'node:fs';
import * as engine from './renderer.ts';

function readObject(url: URL): Record<string, unknown> {
  const value: unknown = JSON.parse(fs.readFileSync(url, 'utf8'));
  assert(engine.isRecord(value));
  return value;
}
const legacy = readObject(new URL('./fixtures/legacy-v1.json', import.meta.url));
assert(engine.isRecord(legacy.parameters));
const legacyParameters = legacy.parameters;
function lightGroup(svg: string, name: 'internal-background' | 'external-transmission'): string {
  const content = svg.match(new RegExp(`<g data-light="${name}">([\\s\\S]*?)<\\/g>`))?.[1];
  assert.notEqual(content, undefined, `missing light group: ${name}`);
  return content ?? '';
}
const isExtinguished = (svg: string) => [...svg.matchAll(/opacity="([^"]+)"/g)].every(match => Number(match[1]) === 0);

test('external light transmits to the opposite side independently of internal emission', () => {
  const restored = engine.parsePreset(legacy);
  const optical = engine.lightingState(restored);
  assert.equal(optical.transmitted.angle, 45);
  assert.equal(engine.lightingState({ ...restored, angle: 45 }).transmitted.angle, 225);
  assert.equal(engine.lightingState({ ...restored, external: 0, emission: 2 }).transmitted.power, 0);
  assert.equal(engine.lightingState({ ...restored, transmission: 0 }).transmitted.power, 0);
  assert(engine.lightingState({ ...restored, external: 1.5 }).transmitted.power > optical.transmitted.power);
  assert.equal(engine.lightingState({ ...restored, emission: 0 }).transmitted.power, optical.transmitted.power);
  assert(optical.transmitted.power > 0);
  assert.equal(lightGroup(engine.render(restored), 'external-transmission'),
    lightGroup(engine.render({ ...restored, emission: 0 }), 'external-transmission'));
  assert(isExtinguished(lightGroup(engine.render({ ...restored, external: 0 }), 'external-transmission')));
  assert(isExtinguished(lightGroup(engine.render({ ...restored, emission: 0 }), 'internal-background')));
});

test('legacy parameters round trip without reintroducing a fixed green light', () => {
  const restored = engine.parsePreset(legacy);
  for (const control of engine.controls) {
    const value = legacyParameters[control.id];
    if (value !== undefined) assert.equal(restored[control.id], value);
  }
  assert(engine.render(restored).includes('clip-rule="evenodd"'));
  const chosen = { ...restored, directionalSpill: true, spillAngle: 137 };
  assert.deepEqual(engine.parsePreset(engine.packagePreset(chosen)), chosen);
  assert.equal(engine.render({ ...chosen, spillAngle: 0 }), engine.render({ ...chosen, spillAngle: 180 }));
  assert(!engine.render(chosen).includes('data-light="directional-spill"'));
  assert.throws(() => engine.parsePreset({ ...legacy, parameters: { ...legacyParameters, directionalSpill: 'yes' } }));
});

test('untrusted presets fail instead of clamping imported values', () => {
  for (const value of [null, '1', -1, 100, NaN, Infinity]) {
    assert.throws(() => engine.parsePreset({ ...legacy, parameters: { ...legacyParameters, external: value } }));
  }
  for (const value of [null, [], 'preset', { ...legacy, parameters: [] }]) {
    assert.throws(() => engine.parsePreset(value));
  }
});

test('TypeScript renderer preserves the exact approved production SVG', () => {
  const preset = readObject(new URL('../../src-tauri/icons/src/app-icon.json', import.meta.url));
  const parameters = engine.parsePreset(preset);
  assert.deepEqual(engine.parsePreset(engine.packagePreset(parameters)), parameters);
  const approved = fs.readFileSync(new URL('../../src-tauri/icons/src/mandala-glass.svg', import.meta.url), 'utf8');
  assert.equal(engine.render(parameters, 'artwork') + '\n', approved);
  assert(!engine.render(parameters, '<injected>').includes('<injected>'));
  assert(!engine.render(parameters).includes('<image'));
});
