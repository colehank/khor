import assert from 'node:assert/strict';
import {build} from 'esbuild';

const result=await build({entryPoints:['src/lib/recovery-relationship.ts'],bundle:true,format:'esm',write:false});
const {recoveryRelationship:project,shouldContinueOnEntry:auto}=await import('data:text/javascript;base64,'+Buffer.from(result.outputFiles[0].text).toString('base64'));
const offer=(patch={})=>({id:'fixed-session',home:'a'.repeat(64),current_run:'fixed-run',next_attempt_epoch:'1',execution:'live',action:'join',native:{agent:'fixture',native_id:'fixed-native',support:'resume'},previous:null,reason:null,...patch});
const facts=(patch={})=>({kind:'gui',offer:offer(),intentOperation:null,outcome:null,ready:true,busy:false,fault:false,...patch});
let passed=0;
function test(name,run){run();passed++;console.log('PASS',name);}

for(const kind of ['shell','gui'])test(`healthy ${kind} has no recovery prompt or automatic action`,()=>{const f=facts({kind});assert.deepEqual(project(f),{state:'quiet',action:null});assert.equal(auto(f),false);});
test('loading an offer does not invent a fault',()=>assert.deepEqual(project(facts({offer:null,ready:false})),{state:'quiet',action:null}));
test('an ended shell explicitly offers a new terminal',()=>{const f=facts({kind:'shell',offer:offer({execution:'ended',action:'new_shell',native:null})});assert.deepEqual(project(f),{state:'ended',action:'new_shell'});assert.equal(auto(f),false);});
test('an ended shell without an epoch cannot offer to start',()=>assert.deepEqual(project(facts({kind:'shell',offer:offer({execution:'ended',action:'new_shell',next_attempt_epoch:null})})),{state:'ended',action:null}));
test('an unknown shell never offers to start',()=>{const f=facts({kind:'shell',offer:offer({execution:'unknown',action:'none',native:null})});assert.deepEqual(project(f),{state:'unconfirmed',action:'refresh'});assert.equal(auto(f),false);});
test('an offline host offers a read-only refresh',()=>{const f=facts({offer:offer({execution:'offline',action:'none'})});assert.deepEqual(project(f),{state:'unconfirmed',action:'refresh'});assert.equal(auto(f),false);});
for(const support of ['resume','load'])test(`owned ended GUI can continue on entry with confirmed ${support}`,()=>assert.equal(auto(facts({offer:offer({execution:'ended',action:'native',native:{agent:'fixture',native_id:'fixed-native',support}})})),true));
for(const support of ['unknown','unsupported'])test(`unconfirmed capability ${support} never automatically starts`,()=>assert.equal(auto(facts({offer:offer({execution:'ended',action:'native',native:{agent:'fixture',native_id:'fixed-native',support}})})),false));
for(const kind of ['observe','tui'])test(`external ${kind} remains quiet and is not taken over`,()=>{const f=facts({kind,offer:offer({execution:'ended',action:'native'})});assert.deepEqual(project(f),{state:'quiet',action:null});assert.equal(auto(f),false);});
for(const patch of [{ready:false},{busy:true},{fault:true}])test('entry action waits for a trustworthy ready surface '+JSON.stringify(patch),()=>assert.equal(auto(facts({offer:offer({execution:'ended',action:'native'}),...patch})),false));
test('there is no automatic action without a fresh offered epoch',()=>assert.equal(auto(facts({offer:offer({execution:'ended',action:'native',next_attempt_epoch:null})})),false));
test('a saved native intent is reconciled instead of discarded on a live offer',()=>{const f=facts({intentOperation:'native'});assert.deepEqual(project(f),{state:'unconfirmed',action:'retry'});assert.equal(auto(f),true);});
test('a saved shell intent may only reconcile its original caller operation',()=>assert.equal(auto(facts({kind:'shell',intentOperation:'new_shell'})),true));
for(const [patch,state] of [[{busy:true},'connecting'],[{busy:true,intentOperation:'native',offer:offer({action:'native'})},'continuing'],[{busy:true,intentOperation:'new_shell'},'opening']])test(`${state} has no duplicate clickable action`,()=>assert.deepEqual(project(facts(patch)),{state,action:null}));
test('a pending original intent stays in progress without manual retry',()=>assert.deepEqual(project(facts({outcome:'pending',intentOperation:'native'})),{state:'connecting',action:null}));
test('a failed pending request can retry only the retained operation',()=>assert.deepEqual(project(facts({outcome:'pending',intentOperation:'native',fault:true})),{state:'failed',action:'retry'}));
test('a storage error offers refresh while safe intent loading is unavailable',()=>assert.deepEqual(project(facts({fault:true,ready:false})),{state:'failed',action:'refresh'}));
test('an unknown outcome with the original intent does not offer a new shell',()=>assert.deepEqual(project(facts({kind:'shell',intentOperation:'new_shell',outcome:'unknown'})),{state:'unconfirmed',action:'retry'}));
test('a missing working directory is selected as a new explicit intent',()=>assert.deepEqual(project(facts({kind:'shell',outcome:'directory_required'})),{state:'directory',action:'directory'}));
test('a completed rejoin does not leave a continue confirmation',()=>assert.deepEqual(project(facts({outcome:'join'})),{state:'quiet',action:null}));
test('a final refused attempt does not mark a confirmed live owner broken',()=>assert.deepEqual(project(facts({outcome:'refused'})),{state:'quiet',action:null}));
console.log(`recovery relationship: ${passed} checks passed`);
