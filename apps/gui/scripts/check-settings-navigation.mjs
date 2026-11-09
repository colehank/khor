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
  const moreSource=readFileSync(new URL('../src/lib/more-navigation.ts',import.meta.url),'utf8');
  const moreFile=join(folder,'more.mjs');
  writeFileSync(moreFile,ts.transpileModule(moreSource,{compilerOptions:{module:ts.ModuleKind.ESNext,target:ts.ScriptTarget.ES2022}}).outputText.replace('./settings-navigation','./navigation.mjs'));
  const {morePageOrder,isMorePage,lastMorePage,rememberMorePage}=await import(pathToFileURL(moreFile));
  const values=new Map();
  globalThis.window={localStorage:{getItem:key=>values.get(key)??null,setItem:(key,value)=>values.set(key,value)}};
  assert.equal(lastSettingsPage(),'session');
  rememberSettingsPage('workspace');assert.equal(lastSettingsPage(),'workspace');
  rememberSettingsPage('terminal');assert.equal(lastSettingsPage(),'session','Existing Terminal deep links normalize to expanded Session settings');
  assert.equal(isSettingsPage('terminal'),true);
  for(const page of settingsPages){rememberSettingsPage(page);assert.equal(lastSettingsPage(),page);}
  assert.equal(lastMorePage(),'session');
  assert.deepEqual(morePageOrder,['session','workspace','appearance','connection','security','store','doctor','about']);
  assert.equal(isMorePage('notifications'),false);
  for(const page of morePageOrder){rememberMorePage(page);assert.equal(lastMorePage(),page);assert.equal(isMorePage(page),true);}
  rememberMorePage('workspace');rememberMorePage('me');assert.equal(lastMorePage(),'workspace');
  rememberMorePage('terminal');assert.equal(lastMorePage(),'session');
  values.set('khor.more-page','me');assert.equal(lastMorePage(),'session','Old My Khor preference cannot reopen it inside More');
  assert.equal(isMorePage('me'),true,'Persisted standalone My Khor routes remain valid');
  values.set('khor.more-page','removed-more-section');assert.equal(lastMorePage(),'session');
  for(const invalid of [null,undefined,{},'settings','network','<script>'])assert.equal(isMorePage(invalid),false);
  values.set('khor.settings-pane','deleted-category');assert.equal(lastSettingsPage(),'session');
  for(const invalid of [null,undefined,{},'me','doctor','settings','<script>'])assert.equal(isSettingsPage(invalid),false);
  window.localStorage.getItem=()=>{throw new Error('storage unavailable');};
  window.localStorage.setItem=()=>{throw new Error('storage unavailable');};
  assert.equal(lastMorePage(),'session');assert.doesNotThrow(()=>rememberMorePage('doctor'));
  assert.equal(lastSettingsPage(),'session');assert.doesNotThrow(()=>rememberSettingsPage('terminal'));
  const native=readFileSync(new URL('../../../crates/node/src/config.rs',import.meta.url),'utf8');
  const inventory=native.match(/pub const PAGES:[\s\S]*?=\s*&\[([\s\S]*?)\];/)[1];
  const nativePages=[...inventory.matchAll(/"([^"]+)"/g)].map(match=>match[1]);
  assert.deepEqual([...settingsPages].sort(),nativePages.filter(page=>page!=='terminal').sort(),'Native categories remain reachable in the requested sidebar order');
  assert.ok(nativePages.every(isSettingsPage),'Every native settings deep link remains valid');
  console.log('PASS settings navigation: restoration, invalid/stale state, denied storage, native category inventory');
} finally {rmSync(folder,{recursive:true,force:true});}
