// Regressions for transmitted external light and existing presets.
import assert from 'node:assert/strict';
import test from 'node:test';
import fs from 'node:fs';
import * as engine from './renderer.mjs';
const legacy = JSON.parse(fs.readFileSync(new URL('./fixtures/legacy-v1.json', import.meta.url), 'utf8'));
test('light sources, glass clipping and legacy presets', () => {
const restored = engine.parsePreset(legacy);
assert(engine.render(restored).includes('clip-rule="evenodd"'),'glass annulus must clip its center out');
for(const [key,value] of Object.entries(legacy.parameters)) assert.equal(restored[key],value);
assert(engine.render(restored).includes('data-light="external-transmission"'),'external light must create its own transmitted green spill');
const optical=engine.lightingState(restored);
assert.equal(optical.transmitted.angle,45,'upper-left source must transmit toward lower right');
assert.equal(engine.lightingState({...restored,angle:45}).transmitted.angle,225);
assert.equal(engine.lightingState({...restored,external:0,emission:2}).transmitted.power,0);
assert(engine.lightingState({...restored,emission:0}).transmitted.power>0,'transmission cannot depend on emissive beads');
assert.equal(engine.lightingState({...restored,emission:0}).transmitted.power,optical.transmitted.power);
assert.equal(engine.lightingState({...restored,transmission:0}).transmitted.power,0);
assert(engine.lightingState({...restored,external:1.5}).transmitted.power>optical.transmitted.power);
const a={...restored,spillAngle:0}, b={...restored,spillAngle:180};
assert.equal(engine.render(a),engine.render(b),'inactive direction must not affect any layer');
assert(!engine.render(a).includes('data-light="directional-spill"'));
assert.equal(engine.render({...a,directionalSpill:true}),engine.render({...b,directionalSpill:true}),'legacy manual direction must not override transmitted light');
const off=engine.render({...restored,emission:0});
const background=off.match(/<g data-light="internal-background">([\s\S]*?)<\/g>/)[1];
assert([...background.matchAll(/opacity="([^"]+)"/g)].every(m=>Number(m[1])===0),'internal spill must extinguish with its source');
const chosen={...restored,directionalSpill:true,spillAngle:137};
assert.deepEqual(engine.parsePreset(engine.packagePreset(chosen)),chosen);
assert.throws(()=>engine.parsePreset({...legacy,parameters:{...legacy.parameters,directionalSpill:'yes'}}));
const externalOff=engine.render({...restored,external:0});
const transmitted=externalOff.match(/<g data-light="external-transmission">([\s\S]*?)<\/g>/)[1];
assert([...transmitted.matchAll(/opacity="([^"]+)"/g)].every(m=>Number(m[1])===0));
const extractTransmission=svg=>svg.match(/<g data-light="external-transmission">([\s\S]*?)<\/g>/)[1];
assert.equal(extractTransmission(engine.render(restored)),extractTransmission(engine.render({...restored,emission:0})),'rendered transmission must remain when internal emission is off');
});

test('invalid presets fail instead of clamping imported data', () => {
  for (const value of [null, '1', -1, 100, NaN, Infinity]) {
    assert.throws(() => engine.parsePreset({ ...legacy, parameters: { ...legacy.parameters, external: value } }));
  }
});

test('current production preset imports and round trips', () => {
  const preset = JSON.parse(fs.readFileSync(new URL('../../src-tauri/icons/src/app-icon.json', import.meta.url), 'utf8'));
  const parameters = engine.parsePreset(preset);
  assert.deepEqual(engine.parsePreset(engine.packagePreset(parameters)), parameters);
  assert(!engine.render(parameters, '<injected>').includes('<injected>'));
  assert(!engine.render(parameters).includes('<image'));
});
