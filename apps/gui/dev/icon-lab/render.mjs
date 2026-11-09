// CLI boundary: the browser and asset generator use the same pure renderer.
import { readFileSync } from 'node:fs';
import { parsePreset, render } from './renderer.mjs';
const parameters = parsePreset(JSON.parse(readFileSync(0, 'utf8')));
process.stdout.write(render(parameters, 'artwork') + '\n');
