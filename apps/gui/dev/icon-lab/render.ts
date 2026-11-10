// CLI boundary: the browser and asset generator use the same pure renderer.
import { readFileSync } from 'node:fs';
import { parsePreset, render } from './renderer.ts';
const payload: unknown = JSON.parse(readFileSync(0, 'utf8'));
const parameters = parsePreset(payload);
process.stdout.write(render(parameters, 'artwork') + '\n');
