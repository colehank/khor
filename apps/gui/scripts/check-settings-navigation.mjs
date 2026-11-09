import assert from 'node:assert/strict';
import {readFileSync, writeFileSync, mkdtempSync, rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';
import ts from 'typescript';
const folder=mkdtempSync(join(tmpdir(),'khor-settings-navigation-'));
try {
  const source=readFileSync(new URL('../src/lib/settings-navigation.ts',import.meta.url),'utf8');
  const file=join(folder,'navigation.mjs');
  writeFileSync(file,ts.transpileModule(source,{compilerOptions:{module:ts.ModuleKind.ESNext,target:ts.ScriptTarget.ES2022}}).outputText);
  const {settingsPages,isSettingsPage,lastSettingsPage,rememberSettingsPage}=await import(pathToFileURL(file));
  const values=new Map();
  globalThis.window={localStorage:{getItem:key=>values.get(key)??null,setItem:(key,value)=>values.set(key,value)}};
  assert.equal(lastSettingsPage(),'appearance');
  rememberSettingsPage('workspace');assert.equal(lastSettingsPage(),'workspace');
  for(const page of settingsPages){rememberSettingsPage(page);assert.equal(lastSettingsPage(),page);}
  values.set('khor.settings-pane','deleted-category');assert.equal(lastSettingsPage(),'appearance');
  for(const invalid of [null,undefined,{},'me','doctor','settings','<script>'])assert.equal(isSettingsPage(invalid),false);
  window.localStorage.getItem=()=>{throw new Error('storage unavailable');};
  window.localStorage.setItem=()=>{throw new Error('storage unavailable');};
  assert.equal(lastSettingsPage(),'appearance');assert.doesNotThrow(()=>rememberSettingsPage('terminal'));
  const native=readFileSync(new URL('../../../crates/node/src/config.rs',import.meta.url),'utf8');
  const inventory=native.match(/pub const PAGES:[\s\S]*?=\s*&\[([\s\S]*?)\];/)[1];
  assert.deepEqual(settingsPages,[...inventory.matchAll(/"([^"]+)"/g)].map(match=>match[1]),'Navigation and native deep links use the same settings categories');
  console.log('PASS settings navigation: restoration, invalid/stale state, denied storage, native category inventory');
} finally {rmSync(folder,{recursive:true,force:true});}
