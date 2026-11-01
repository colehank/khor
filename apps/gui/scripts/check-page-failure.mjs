// Executes the production typed-error projection with explicit label tokens.
// Does not substitute generated wire types or claim translated/catalog output.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {execFileSync} from 'node:child_process';
import vm from 'node:vm';
import ts from 'typescript';
const legacy=process.argv.includes('--legacy');
const source=legacy?execFileSync('git',['show','2ba5e983:apps/gui/src/browse/page-fail.ts'],{encoding:'utf8'}):fs.readFileSync(new URL('../src/browse/page-fail.ts',import.meta.url),'utf8');
const compiled=ts.transpileModule(source,{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText;
const context=vm.createContext({exports:{},require:name=>{assert.equal(name,'@/gen/catalog');return {gui:new Proxy({},{get:(_,key)=>key})};}});
vm.runInContext(compiled,context);const {pageFail}=context.exports;
if(legacy){assert.equal(pageFail('-1005|NSURLErrorDomain|The network connection was lost.|https://original.test').reason,'page_fail_lost','a lost connection is not proof of an actively refused connection');process.exit(0);}
const error=(code,domain='NSURLErrorDomain',description='actual OS message')=>({code:String(code),domain,description,url:'https://original.test/a|b',underlying:[],truncated:false});
let passed=0;function check(name,run){run();console.log('PASS',name);passed++;}
for(const [code,label] of [[-1003,'host'],[-1004,'connect'],[-1005,'lost'],[-1001,'timeout'],[-1009,'offline'],[-1200,'tls']])check(`NSError ${code} retains the correct reason`,()=>{
 const raw=error(code),result=pageFail(raw);assert.equal(result.reason,'page_fail_'+label);assert.equal(result.url,raw.url);assert(result.detail.includes('NSURLErrorDomain '+code+': actual OS message'));
});
check('certificate override remains limited to the original passable codes',()=>{
 for(const code of [-1201,-1202,-1203,-1204])assert.equal(pageFail(error(code)).pass,true);
 assert.equal(pageFail(error(-1200)).pass,undefined);
});
check('explicit route facts map to route; a generic khor error does not',()=>{
 assert.equal(pageFail(error('route','khor','')).reason,'page_fail_route');
 assert.equal(pageFail(error(310,'kCFErrorDomainCFNetwork')).reason,'page_fail_route');
 assert.equal(pageFail(error('observer','khor','native observer unavailable')).reason,'native observer unavailable');
});
check('unknown errors keep the original description, code and bounded cause evidence',()=>{
 const raw=error('-9223372036854775808','UnexpectedDomain','actual explanation');raw.underlying=[{code:'61',domain:'POSIX',description:'underlying refusal',url:''}];
 const result=pageFail(raw);assert.equal(result.reason,'actual explanation');assert(result.detail.includes(raw.code));assert(result.detail.includes('POSIX 61: underlying refusal'));assert.equal(result.url,raw.url);
});
check('null snapshot has no invented failure',()=>assert.equal(pageFail(null),null));
console.log(`${passed} page failure projection checks passed`);
