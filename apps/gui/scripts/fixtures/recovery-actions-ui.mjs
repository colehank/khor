import assert from 'node:assert/strict';

// Fixed UI facts and controlled external IO. No Node execution qualification,
// idempotency/reservation algorithm, canonical parser or replacement IDB owner.
export const recoveryActionCases = ['offer-and-same-sid-rebind', 'idb-retries-and-shared-pages', 'directory-required-shell', 'terminal-owner-transfer'];

export const recoveryProbe = `
import React,{useState} from 'react';
import {createRoot} from 'react-dom/client';
import {DetailPane} from '/src/views/DetailPane';
import {TooltipProvider} from '/src/components/ui/tooltip';
import {tabAct} from '/src/lib/tab-acts';
import {gui} from '/src/gen/catalog';
import '/src/styles/tokens.css'; import '/src/styles/app.css';
document.documentElement.dataset.theme='light';
function Probe(){
 const [shown,setShown]=useState(true),[id,setId]=useState(new URL(location.href).searchParams.get('id')||'alpha');
 const [rows,setRows]=useState(window.__recoveryWire.rows);
 window.__recoveryWords=gui;
 const patch=(change,target=id)=>setRows(old=>({...old,[target]:{...old[target],...change}}));
 window.__chatReact={close:()=>setShown(false),open:()=>setShown(true),switch:()=>setId('beta'),reload:()=>tabAct(id)?.run()};
 window.__recoveryProbe={patch,select:setId,current:()=>id,row:()=>rows[id]};
 return <TooltipProvider><div style={{height:'100vh',display:'flex',flexDirection:'column',minHeight:0}}>
 <header><span data-lab-session-settings-slot/></header>
 {shown&&<DetailPane row={rows[id]} narrow={false} onBack={()=>{}} active onSession={setId} sessions={Object.values(rows)} openIds={Object.keys(rows)}/>}</div></TooltipProvider>;
}
createRoot(document.getElementById('root')).render(<Probe/>);
`;

export function installRecoveryActionsTransport() {
  const base = window.__TAURI_INTERNALS__.invoke, chat = window.__chatWire;
  const A = 'a'.repeat(64), B = 'b'.repeat(64);
  const row = (id, patch = {}) => ({id, kind:'gui', title:`Recovery ${id}`, word:'idle',
    observation_owner:null, content_locator:`fixed-content-${id}`, can_control:true,
    at_ms:1, unread:0, source:null, pinned:false, imported:false, stale:false, quiet:false,
    category:'claude', group:'', face:null, hosted:true, attachable:false, last:null,
    doing:null, via:null, via_face:null, home:'fixture-A', place:'/fixture/old', mux:false,
    ended:null, execution_home:A, ...patch});
  const offer = (id, home = A, patch = {}) => ({id, home, current_run:`fixture-run-${id}`,
    next_attempt_epoch:'1', execution:'live', action:'join',
    native:{agent:'claude',native_id:`fixed-native-${id}`,support:'resume'}, previous:null, reason:null,...patch});
  const offers = new Map();
  for (const id of ['alpha','beta','fresh-shell']) for (const home of [A,B]) offers.set(JSON.stringify([home,id]), offer(id,home));
  const wire = window.__recoveryWire = {
    rows:{alpha:row('alpha'),beta:row('beta'), 'fresh-shell':row('fresh-shell',{kind:'shell',category:'shell',attachable:true})},
    calls:[], offers:[], actions:[], terminals:[], waits:[], drops:[], peers:[], holdOffer:new URL(location.href).searchParams.get('recovery_case')==='offer-and-same-sid-rebind',
    holdAction:false, replies:[], holdDrop:false, termHome:A, holdOfferTargets:[],
    offer(home,id,change) { offers.set(JSON.stringify([home,id]),offer(id,home,change)); },
    releaseOffer(index, reply) { const held=wire.offers[index];if(!held||held.done)throw new Error('offer not held');held.done=true;held.resolve(reply); },
    releaseAction(index, reply, error=false) { const held=wire.actions[index];if(!held||held.done)throw new Error('action not held');held.done=true;error?held.reject(new Error(reply)):held.resolve(reply); },
    releaseDrop(index) { const held=wire.drops[index];if(!held||held.done)throw new Error('drop not held');held.done=true;held.resolve({landed:['/fixture/landed'],miss:null}); },
  };
  window.__TAURI_INTERNALS__.invoke = async (cmd,args={}) => {
    wire.calls.push({cmd,args:structuredClone(args)});
    if(cmd==='session_recovery') {
      const answer=offers.get(JSON.stringify([args.home_id,args.id]));if(!answer)throw new Error('unknown fixed offer target');
      if(wire.holdOffer||wire.holdOfferTargets.includes(JSON.stringify([args.home_id,args.id])))return new Promise(resolve=>wire.offers.push({args,answer,resolve,done:false}));
      return structuredClone(answer);
    }
    if(cmd==='recover_session'||cmd==='new_shell_run') {
      if(wire.holdAction)return new Promise((resolve,reject)=>wire.actions.push({cmd,args:structuredClone(args),resolve,reject,done:false}));
      const answer=wire.replies.shift();if(!answer)throw new Error('unconfigured recovery action reply');
      if(answer.error)throw new Error(answer.error);return structuredClone(answer);
    }
    if(cmd==='session_content') return {kind:'available',binding:'canonical-1',following:false,capture:null};
    if(cmd==='devices') {
      const device=(id,name,shown,me)=>({id,name,shown,me,label:null,paused:false,face:null,pinned:false,vitals:null,
        hop:null,heard_ms:null,rtt_ms:null,down_ms:null,last_rtt_ms:null,since_ms:null,last_hop:null,
        relay:null,via:null,sponsor:null,ssh_jump:null,watch:[],watched:null,receive_dir:null,receive_at:null,
        receive_for:null,receive_why:null,seat:{left:0,top:0},hosts:true,roads:[]});
      return [device(A,'fixture-A','Fixture A',true),device(B,'fixture-B','Fixture B',false)];
    }
    if(cmd==='ls')return {path:'/fixture/',entries:[],truncated:false};
    if(cmd==='term_ended')return {ended:{exit:0},recent:[],screen:null};
    if(cmd==='term_drop_forget')return null;
    if(cmd.startsWith('term_')) {
      const admission=window.__attachmentLeases(cmd,args);if(admission.handled)return admission.value;
      if(cmd==='term_open'){wire.terminals.push({id:args.id,lease:args.lease,home:wire.termHome,number:wire.terminals.length+1});return null;}
      const current=wire.terminals.find(value=>value.lease===args.lease);
      if(cmd==='term_attach')return {bytes:btoa(`TERMINAL ${current.home} ${args.id} OPEN ${current.number}\r\n`),seq:current.number};
      if(cmd==='term_wait'||cmd==='term_stream')return new Promise(resolve=>wire.waits.push({lease:args.lease,seq:current.number,resolve,done:false}));
      if(cmd==='term_drop_items'||cmd==='term_drop') {
        if(wire.holdDrop)return new Promise(resolve=>wire.drops.push({lease:args.lease,resolve,done:false}));
        return {landed:['/fixture/landed'],miss:null};
      }
      if(cmd==='term_drop_progress')return 0.25;
      if(['term_key','term_resize','term_ready','term_drop_cancel','term_forget','term_prompt','term_scroll','term_pane_history'].includes(cmd))return null;
      throw new Error(`unconfigured terminal fixture command ${cmd}`);
    }
    if(['face_note','bell_haptic','term_drop_forget'].includes(cmd))return null;
    const answer=await base(cmd,args);
    // The existing attachment validator owns close; the controlled wait ends
    // only on that real leave, never by clearing an arbitrary fixture table.
    return answer;
  };
  // Preserve the existing lease validation and claim timing on leave too.
  const invoke=window.__TAURI_INTERNALS__.invoke;
  window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
    if(cmd==='term_leave') {
      if(wire.drops.some(drop=>drop.lease===args.lease&&!drop.done))throw new Error('terminal left before its accepted drop settled');
      for(const wait of wire.waits)if(wait.lease===args.lease&&!wait.done){wait.done=true;wait.resolve({bytes:'',seq:wait.seq,gone:true,reset:false,stall:null});}
    }
    return invoke(cmd,args);
  };
}

async function advanceUntil(page,check,description){return until(async()=>{await page.clock.runFor(32);return check();},description);}
const sleep = ms => new Promise(resolve=>setTimeout(resolve,ms));
async function until(check,description){const end=Date.now()+8000;while(!await check()){assert(Date.now()<end,description);await sleep(10);}}
const calls = (page,cmd) => page.evaluate(cmd=>window.__recoveryWire.calls.filter(call=>call.cmd===cmd),cmd);
async function words(page){return page.evaluate(()=>({title:window.__recoveryWords.recovery_title,resume:window.__recoveryWords.recovery_resume,join:window.__recoveryWords.recovery_join,retry:window.__recoveryWords.recovery_retry_intent,choose:window.__recoveryWords.recovery_choose_directory}));}
async function menu(page){const w=await words(page);await page.getByRole('button',{name:w.title,exact:true}).click();}
async function action(page,label){await menu(page);await page.getByRole('menuitem',{name:label,exact:true}).click();}
async function closeMenu(page){await page.keyboard.press('Escape');}
async function pendingCount(page){return page.evaluate(()=>new Promise((resolve,reject)=>{const q=indexedDB.open('khor-session-recovery',1);q.onsuccess=()=>{const db=q.result,t=db.transaction('caller'),r=t.objectStore('caller').count();r.onsuccess=()=>resolve(r.result);r.onerror=()=>reject(r.error);t.oncomplete=()=>db.close();};q.onerror=()=>reject(q.error);}));}
async function newTerminal(page) {
  await page.evaluate(()=>window.__recoveryProbe.patch({kind:'shell',category:'shell',can_control:true,attachable:true,word:'idle',ended:null}));
  await page.clock.runFor(500);
  await advanceUntil(page,()=>page.evaluate(()=>window.__recoveryWire.calls.some(call=>call.cmd==='term_ready')),'actual terminal attached and ready');
  await page.waitForFunction(()=>document.querySelector('[data-terminal]')?.__xterm?.buffer.active.getLine(0)?.translateToString().includes('TERMINAL'));
}
async function holdPathDrop(page) {
  const index=await page.evaluate(()=>window.__recoveryWire.drops.length);
  await page.evaluate(index=>{window.__recoveryWire.holdDrop=true;const dt=new DataTransfer();dt.setData('application/x-khor-path',JSON.stringify({machine:null,path:'/fixture/controlled-file',token:'recovery-owned-drop-'+index}));document.querySelector('[data-terminal]').dispatchEvent(new DragEvent('drop',{dataTransfer:dt,bubbles:true,cancelable:true}));},index);
  await until(()=>page.evaluate(index=>window.__recoveryWire.drops.length===index+1,index),'actual DOM path-drop entered real sender IO');
  return index;
}

// Four explicitly new S3 groups in the existing harness. The original thirteen
// keep their names and complete capture/read assertions independently.
export async function exerciseRecoveryActions(scenario,page,helpers) {
  const A='a'.repeat(64),B='b'.repeat(64);
  const publicCalls=cmd=>calls(page,cmd);
  const heldAction=async(label)=>{const index=await page.evaluate(()=>window.__recoveryWire.actions.length);await page.evaluate(()=>{window.__recoveryWire.holdAction=true;});const w=await words(page);await action(page,label??w.join);await until(()=>page.evaluate(index=>window.__recoveryWire.actions.length===index+1,index),'explicit action reached controlled IO');return index;};
  if(scenario==='offer-and-same-sid-rebind') {
    // The initial read-only offer was held before DetailPane mounted. No
    // response to it is needed to paint the canonical history below.
    await until(()=>page.evaluate(()=>window.__recoveryWire.offers.length>0),'read-only offer held');
    assert((await page.locator('[data-chat-column]').innerText()).includes('Complete known history.'));
    assert.equal((await publicCalls('recover_session')).length,0);assert.equal((await publicCalls('new_shell_run')).length,0);
    await page.evaluate(()=>{const w=window.__recoveryWire;w.holdOffer=false;w.releaseOffer(0,w.offers[0].answer);});
    await page.evaluate(({A})=>{window.__recoveryWire.offer(A,'alpha',{execution:'ended',action:'native'});window.__recoveryProbe.patch({word:'done',ended:{exit:0}});},{A});
    await page.waitForFunction(()=>window.__recoveryProbe.row().word==='done'&&window.__recoveryProbe.row().ended?.exit===0);
    const before=(await helpers.calls('chat_open')).length;
    const firstAction=await heldAction((await words(page)).resume);await closeMenu(page);
    await page.evaluate(({A})=>{window.__chatWire.source(['Recovered after own row transition.']);window.__recoveryWire.offer(A,'alpha',{current_run:'published-new-run',next_attempt_epoch:'2'});window.__recoveryProbe.patch({word:'idle',ended:null,can_control:true});},{A});
    await page.waitForFunction(()=>window.__recoveryProbe.row().word==='idle'&&window.__recoveryProbe.row().ended===null);
    assert.equal((await helpers.calls('chat_open')).length,before,'row polling alone does not fake the required sameSID rebind');
    await page.evaluate(index=>window.__recoveryWire.releaseAction(index,{kind:'opened',id:'alpha',run_id:'published-new-run'}),firstAction);
    await until(async()=>(await helpers.calls('chat_open')).length===before+1,'sameSID success really reopens view after its own row status changed');
    await helpers.capture();await helpers.check(['Recovered after own row transition.']);

    const lateAction=await heldAction();await closeMenu(page);await page.evaluate(()=>{window.__chatWire.source(['Current beta remains.']);window.__recoveryProbe.select('beta');});
    await until(async()=>(await helpers.calls('chat_open')).at(-1)?.args.id==='beta','new target has its own actual view');
    await helpers.capture();const betaOpens=(await helpers.calls('chat_open')).length;
    await page.evaluate(index=>window.__recoveryWire.releaseAction(index,{kind:'opened',id:'alpha',run_id:'late-retired-run'}),lateAction);
    await page.clock.runFor(200);assert.equal(await page.evaluate(()=>window.__recoveryProbe.current()),'beta');assert.equal((await helpers.calls('chat_open')).length,betaOpens,'old target does not rebind or navigate current view');await helpers.check(['Current beta remains.'],['Late abandoned owner.']);
  } else if(scenario==='idb-retries-and-shared-pages') {
    const w=await words(page);await page.evaluate(()=>{window.__recoveryWire.replies.push({error:'controlled transport ambiguity'});});
    await action(page,w.join);await until(async()=>(await publicCalls('recover_session')).length===1,'first actual recovery payload sent');
    await closeMenu(page);await page.evaluate(()=>window.__recoveryWire.replies.push({kind:'pending',id:'alpha',attempt_epoch:'1'}));
    await until(()=>page.getByRole('button',{name:w.title,exact:true}).isEnabled(),'recovery surface available');
    await action(page,w.retry);await until(async()=>(await publicCalls('recover_session')).length===2,'transport retry sent');
    await closeMenu(page);await page.evaluate(()=>window.__recoveryWire.replies.push({kind:'unknown',id:'alpha',attempt_epoch:'1',reason:'controlled unknown'}));
    await action(page,w.retry);await until(async()=>(await publicCalls('recover_session')).length===3,'pending retry sent');
    const sent=await publicCalls('recover_session');assert.deepEqual(sent[1].args,sent[0].args);assert.deepEqual(sent[2].args,sent[0].args);assert.equal(await pendingCount(page),1);
    await closeMenu(page);
    const capturesBefore=(await helpers.calls('chat_capture')).length,leavesBefore=(await helpers.calls('chat_leave')).length;
    await page.evaluate(()=>window.__chatReact.close());await until(async()=>(await helpers.calls('chat_leave')).length>leavesBefore,'actual view unmounted with unresolved caller intent');
    await page.evaluate(()=>window.__chatReact.open());await until(async()=>(await helpers.calls('chat_capture')).length>capturesBefore,'new mounted view captures independently');await helpers.capture();
    await page.evaluate(()=>window.__recoveryWire.replies.push({kind:'pending',id:'alpha',attempt_epoch:'1'}));await action(page,w.retry);await until(async()=>(await publicCalls('recover_session')).length===4,'remounted UI retries persisted payload');assert.deepEqual((await publicCalls('recover_session'))[3].args,sent[0].args);
    await closeMenu(page);await page.evaluate(()=>window.__recoveryWire.replies.push({kind:'refused',reason:'controlled final reply'}));await action(page,w.retry);await until(async()=>await pendingCount(page)===0,'final typed reply retires only this payload');await closeMenu(page);
    // Both pages share one real browser context and origin/IndexedDB.
    const peer=await page.context().newPage();assert.equal(peer.context(),page.context(),'both UI pages use the same browser context');try {
      await helpers.preparePeer(peer);await peer.goto(page.url());assert.equal(await peer.evaluate(()=>location.origin),await page.evaluate(()=>location.origin),'both pages share the actual IDB origin');await until(()=>peer.locator('[data-chat-column]').count().then(n=>n>0),'peer production view mounted');
      await page.evaluate(({A})=>window.__recoveryWire.offer(A,'alpha',{next_attempt_epoch:'2'}),{A});const sharedAction=await heldAction();await closeMenu(page);const first=(await publicCalls('recover_session')).at(-1).args;
      const w=await words(peer);await peer.evaluate(()=>{window.__recoveryWire.holdAction=true;});await action(peer,w.retry);
      await until(()=>calls(peer,'recover_session').then(a=>a.length===1),'same-target second page sends existing payload');assert.deepEqual((await calls(peer,'recover_session'))[0].args,first);
      await closeMenu(peer);await peer.evaluate(()=>window.__recoveryProbe.select('beta'));await until(()=>peer.locator('[data-chat-column]').count().then(n=>n>0),'different target shown');
      await action(peer,w.join);await until(()=>calls(peer,'recover_session').then(a=>a.length===2),'different target does not wait for alpha request');const other=(await calls(peer,'recover_session'))[1].args;assert.equal(other.id,'beta');assert.notEqual(other.token.nonce,first.token.nonce);assert.equal(await pendingCount(page),2);
      await page.evaluate(index=>window.__recoveryWire.releaseAction(index,{kind:'refused',reason:'controlled final alpha'}),sharedAction);await peer.evaluate(()=>{window.__recoveryWire.releaseAction(0,{kind:'refused',reason:'controlled same-alpha final'});window.__recoveryWire.releaseAction(1,{kind:'refused',reason:'controlled beta final'});});await until(async()=>await pendingCount(page)===0,'independent final replies release both target slots');
      const peerFacts=await peer.evaluate(()=>({origin:location.origin,calls:window.__recoveryWire.calls.slice(-48)}));await page.evaluate(facts=>window.__recoveryWire.peers.push(facts),peerFacts);
    }finally{await peer.close();}
  } else if(scenario==='directory-required-shell') {
    // Independent new Shell UI coverage; original13 do not claim this behavior.
    await page.evaluate(({A})=>{window.__recoveryWire.offer(A,'alpha',{action:'new_shell',execution:'ended',current_run:'alpha',next_attempt_epoch:'4',native:null});window.__recoveryWire.replies.push({kind:'directory_required',id:'alpha'});window.__recoveryProbe.patch({kind:'shell',category:'shell',word:'failed',can_control:false,attachable:false,ended:{exit:0}});},{A});
    await page.clock.runFor(200);const newLabel=await page.evaluate(()=>window.__recoveryWords.recovery_new_shell);await action(page,newLabel);
    await until(async()=>(await publicCalls('new_shell_run')).length===1,'old cwd request completed');const first=(await publicCalls('new_shell_run'))[0].args;
    await page.evaluate(({A})=>{window.__recoveryWire.holdOfferTargets.push(JSON.stringify([A,'fresh-shell']));window.__recoveryWire.offer(A,'alpha',{action:'new_shell',execution:'ended',current_run:'alpha',next_attempt_epoch:'5',native:null});window.__recoveryWire.replies.push({kind:'opened',id:'fresh-shell',previous:'alpha'});},{A});
    const w=await words(page);await page.getByRole('menuitem',{name:w.choose,exact:true}).click();const field=page.locator('[data-dir-picker-path] input, input[data-dir-picker-path]');await field.fill('/fixture/chosen');
    const take=await page.evaluate(()=>window.__recoveryWords.dir_picker_take);await page.getByRole('button',{name:take,exact:true}).click();
    await until(async()=>(await publicCalls('new_shell_run')).length===2,'confirmed new cwd sends a new intent');const second=(await publicCalls('new_shell_run'))[1].args;
    assert.equal(first.cwd,null);assert.equal(second.cwd,'/fixture/chosen');assert.equal(second.expected_run,'alpha');assert.equal(second.home_id,A);assert.equal(second.token.epoch,'5');assert.notEqual(second.token.nonce,first.token.nonce);
    await until(()=>page.evaluate(()=>window.__recoveryProbe.current()==='fresh-shell'),'newShell alone opens fresh logical SID');
    await page.clock.runFor(500);await advanceUntil(page,()=>page.evaluate(()=>window.__recoveryWire.calls.some(c=>c.cmd==='term_ready'&&c.args.id==='fresh-shell')),'newShell really mounts a Ready terminal');
    await menu(page);const previous=await page.evaluate(()=>window.__recoveryWords.recovery_previous);assert(await page.getByRole('menuitem').filter({hasText:previous}).filter({hasText:'alpha'}).isVisible());await closeMenu(page);
  } else if(scenario==='terminal-owner-transfer') {
    await page.evaluate(({A})=>window.__recoveryWire.offer(A,'alpha',{native:null}),{A});await newTerminal(page);
    await page.evaluate(()=>{window.__firstRecoveryTerminal=document.querySelector('[data-terminal]').__xterm;window.__recoveryProbe.patch({kind:'shell',category:'shell',can_control:true,attachable:true},'beta');window.__recoveryProbe.select('beta');});await page.clock.runFor(500);await advanceUntil(page,()=>page.evaluate(()=>window.__recoveryWire.calls.some(c=>c.cmd==='term_ready'&&c.args.id==='beta')),'second actual kept terminal opened');
    await page.evaluate(()=>{window.__otherRecoveryTerminal=document.querySelector('[data-kept-terminal="beta"] [data-terminal]').__xterm;window.__recoveryProbe.select('alpha');});await page.clock.runFor(32);
    for (const bounce of [false,true]) {
      const index=await holdPathDrop(page);
      const opened=await page.evaluate(()=>window.__recoveryWire.terminals.length);
      const readyBefore=(await publicCalls('term_ready')).filter(c=>c.args.id==='alpha').length;
      const leaves=(await publicCalls('term_leave')).length, cancels=(await publicCalls('term_drop_cancel')).length;
      if(!bounce) {
        await page.evaluate(()=>window.__recoveryWire.replies.push({kind:'join',id:'alpha',current_run:'fixture-run-alpha'}));const w=await words(page);await action(page,w.join);await closeMenu(page);
      } else {
        await page.evaluate(({B})=>{window.__recoveryWire.termHome=B;window.__recoveryProbe.patch({execution_home:B,home:'fixture-B'});},{B});
        await page.clock.runFor(32);
        await until(()=>page.locator('[data-kept-terminal="alpha"][data-active]').count().then(n=>n===0),'home B retires actual interactive A');
        await page.evaluate(({A})=>{window.__recoveryWire.termHome=A;window.__recoveryProbe.patch({execution_home:A,home:'fixture-A'});},{A});
      }
      await page.clock.runFor(400);
      assert.equal(await page.evaluate(()=>window.__recoveryWire.terminals.length),opened,'no reconnect can destroy accepted held transfer');
      assert.equal((await publicCalls('term_drop_cancel')).length,cancels);assert.equal((await publicCalls('term_leave')).length,leaves);
      assert(await page.evaluate(()=>document.querySelector('[data-terminal]').__xterm===window.__firstRecoveryTerminal),'same owned terminal remains until IO finishes');
      if(bounce)assert.equal(await page.locator('[data-kept-terminal="alpha"][data-active]').count(),0,'returning A does not unretire its old seat');
      await page.evaluate(index=>window.__recoveryWire.releaseDrop(index),index);await page.clock.runFor(500);
      await advanceUntil(page,async()=>await page.evaluate(opened=>window.__recoveryWire.terminals.length===opened+1,opened)&&(await publicCalls('term_ready')).filter(c=>c.args.id==='alpha').length===readyBefore+1,'actual original IO completion permits a fully Ready new connection');
      if(!bounce)assert(await page.evaluate(()=>document.querySelector('[data-terminal]').__xterm===window.__firstRecoveryTerminal),'same-home rejoin preserves mounted terminal and staged state');
      else assert(await page.evaluate(()=>document.querySelector('[data-terminal]').__xterm!==window.__firstRecoveryTerminal),'retired A gets a fresh generation after IO');
    }
    assert.equal((await publicCalls('term_open')).filter(call=>call.args.id==='beta').length,1,'other kept terminal never remounts');
    assert.equal((await publicCalls('term_leave')).filter(call=>call.args.id==='beta').length,0,'other kept resource never leaves');
    assert(await page.evaluate(()=>document.querySelector('[data-kept-terminal="beta"] [data-terminal]').__xterm===window.__otherRecoveryTerminal));
  } else throw new Error('Unknown S3 recovery group: '+scenario);
}
