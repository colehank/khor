// Real production snapshot projection. No DOM, native handles, generated
// substitutes or socket calls; the type-only generated import is erased.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';
import {build} from 'esbuild';
import {fileURLToPath} from 'node:url';
const file=fileURLToPath(new URL('../src/browse/loading-rule.ts',import.meta.url));
const {outputFiles}=await build({entryPoints:[file],bundle:true,write:false,format:'esm',platform:'node'});
const {nativeReceived,nativeBusy}=await import('data:text/javascript;base64,'+Buffer.from(outputFiles[0].text).toString('base64'));
const state=(patch={})=>({tab:'tab',view:'1',navigation:'2',revision:'3',phase:'started',estimate:.1,requested_url:'https://requested.test',url:'',exit:'exit',proxied:true,error:null,...patch});
let passed=0;function check(name,run){run();console.log('PASS',name);passed++;}
check('a delayed initial query cannot rewind a delivered event',()=>{
 const event=nativeReceived(new Map(),state({revision:'5',estimate:.6}));
 assert.equal(nativeReceived(event,state()),event);
});
check('queued old same-view progress and start snapshots do not affect the newer load',()=>{
 const a=state(),progress=state({revision:'4',estimate:.8});
 const b=state({navigation:'5',revision:'6',requested_url:'https://new.test',estimate:.2});
 const now=nativeReceived(nativeReceived(new Map(),a),b);
 assert.equal(nativeReceived(now,progress),now);assert.equal(nativeReceived(now,a),now);
 assert.equal(now.get('tab'),b);
});
check('same URL reload has new navigation identity; redirects retain it',()=>{
 const a=state();let loads=nativeReceived(new Map(),a);
 const redirect=state({phase:'committed',revision:'4',url:'https://redirect.test',estimate:.7});
 loads=nativeReceived(loads,redirect);assert.equal(loads.get('tab').navigation,'2');
 const finished=state({phase:'finished',revision:'5',estimate:1});loads=nativeReceived(loads,finished);
 const reload=state({navigation:'6',revision:'7',estimate:null});loads=nativeReceived(loads,reload);
 assert.equal(loads.get('tab'),reload);
});
check('a replaced view rejects the old view even when its URL is the same',()=>{
 const current=state({view:'10',navigation:'11',revision:'12'}),loads=nativeReceived(new Map(),current);
 assert.equal(nativeReceived(loads,state({revision:'13'})),loads);
});
check('a non-WK settled load has stopped but does not invent successful completion',()=>{
 const settled=state({phase:'settled',estimate:null});const loads=nativeReceived(new Map(),settled);
 assert.equal(loads.get('tab'),settled);assert.equal(nativeBusy(settled),false);
 assert.equal(nativeReceived(loads,state({phase:'finished',estimate:1,revision:'4'})),loads);
});
check('failure and cancellation are terminal without pretending to complete',()=>{
 for(const phase of ['failed','cancelled']){
  const failed=state({phase,estimate:.4,error:{domain:'NSURLErrorDomain',code:'-1004',description:'raw failure',url:'https://failed.test',underlying:[],truncated:false}});
  let loads=nativeReceived(new Map(),failed);assert.equal(loads.get('tab'),failed);assert.equal(nativeBusy(failed),false);
  assert.equal(nativeReceived(loads,state({phase:'finished',estimate:1,revision:'4'})),loads);
 }
});
check('only successful finish may carry a complete estimate',()=>{
 let loads=new Map();assert.equal(nativeReceived(loads,state({estimate:1})),loads);
 assert.equal(nativeReceived(loads,state({phase:'finished',estimate:.9})),loads);
 const finished=state({phase:'finished',estimate:1});loads=nativeReceived(loads,finished);assert.equal(loads.get('tab'),finished);
 for(const estimate of [NaN,Infinity,-.1,1.1])assert.equal(nativeReceived(new Map(),state({estimate})).size,0);
});
check('native identity comparison preserves integers beyond JS Number precision',()=>{
 const a=state({view:'9007199254740993',navigation:'9007199254740994',revision:'9007199254740995'});
 const b=state({view:a.view,navigation:'9007199254740996',revision:'9007199254740997'});
 const loads=nativeReceived(nativeReceived(new Map(),a),b);assert.equal(loads.get('tab'),b);assert.equal(nativeReceived(loads,a),loads);
 for(const revision of ['0','-1','nan','1e6','9'.repeat(100)])assert.equal(nativeReceived(new Map(),state({revision})).size,0);
});
// The production render body distinguishes unknown from a measured share.
// createElement is only a record boundary; this does not mount ReactDOM.
const barSource=fs.readFileSync(new URL('../src/components/SessionBar.tsx',import.meta.url),'utf8');
const ast=ts.createSourceFile('bar.tsx',barSource,ts.ScriptTarget.Latest,true,ts.ScriptKind.TSX);
const body=ast.statements.find(node=>ts.isFunctionDeclaration(node)&&node.name?.text==='PillProgress');assert(body);
const ui=vm.createContext({React:{createElement:(tag,props,...children)=>({tag,props,children})}});
vm.runInContext(ts.transpileModule(body.getText(ast).replace(/^export /,'')+'\nglobalThis.line=PillProgress;',{compilerOptions:{target:ts.ScriptTarget.ES2022,jsx:ts.JsxEmit.React}}).outputText,ui);
check('unknown native progress is a static pending marker, with no empty numeric loop selector',()=>{
 const line=ui.line({share:null});assert.equal(line.props['data-pill-pending'],true);assert.equal('data-pill-progress' in line.props,false);assert.equal(line.children.length,0);
});
check('measured progress and successful completion render their actual shares',()=>{
 for(const share of [.1,.7,1]){const line=ui.line({share});assert.equal(line.props['data-pill-progress'],share);assert.equal(line.children[0].props.style.transform,`scaleX(${share})`);}
});
check('omitting a page share preserves other kinds existing busy indicator',()=>{
 const line=ui.line();assert.equal(line.props['data-pill-progress'],'');assert.equal(line.children[0].props.style,undefined);
});
console.log(`${passed} native page projection/render checks passed; controlled element records, no DOM`);
