import {checkResourceFiles} from './resource-checks.mjs';
import {checkLongHistory} from './history-checks.mjs';
import {checkWorkspaceBack} from './workspace-back-checks.mjs';
import {checkSettingsNavigation} from './settings-checks.mjs';
import {checkGuiExperience,checkWorkspaceSessions} from './gui-experience-checks.mjs';
import {checkCommandTab} from './child-command-checks.mjs';
import {checkConversationNavigation} from './conversation-navigation-checks.mjs';
import {chromium, webkit} from 'playwright-core';
import {spawn} from 'node:child_process';
import {mkdir, writeFile} from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import {checkMobileNavigation} from './navigation-checks.mjs';
import {checkListScroll} from './list-scroll-checks.mjs';
import {checkAgentSetup} from './agent-setup-checks.mjs';
import {checkMachinePairing} from './pairing-checks.mjs';
const report = process.env.LAB_REPORT;
await mkdir(report, {recursive: true});
const port = 23918;
const server = spawn('python3', ['-m', 'khor_lab.preview', process.env.LAB_BUNDLE, '--port', String(port)], {
  cwd: path.resolve('../..'), env: {...process.env, PYTHONPATH: 'scripts'}, stdio: 'inherit',
});

async function inspect(browser, scene, size, data = {}) {
  const context = await browser.newContext({viewport: size, isMobile: size.width < 500 && !data.fine, hasTouch: size.width < 500 && !data.fine});
  if(data.nativeMarker)await context.addInitScript(()=>{
    window.__labNativeCalls=[];
    window.__TAURI_INTERNALS__={metadata:{currentWindow:{label:'main'},currentWebview:{label:'khor-page-preview'}},invoke:async(...args)=>{window.__labNativeCalls.push(args);throw new Error('Frontend Lab must never invoke its browser host');}};
  });
  const page = await context.newPage(), errors = [];
  const machines = data.machines ?? 1;
  const items=data.items??5;
  const name = `${scene}-${size.width}x${size.height}${data.machines===undefined?'':`-machines-${machines}`}${data.gesture?`-gesture-${data.gesture}-${items}`:''}${data.engine?`-${data.engine}`:''}${data.fine?'-fine':''}${data.setup?'-setup':''}${data.pairing?'-pairing':''}${data.nativeMarker?'-embedded':''}`;
  let geometry = null;
  const contentLayout = [];
  const expectedConsoleErrors = [];
  const workspaceLayout = [];
  let listScroll = null;
  async function checkChildConversation() {
    const reader=page.locator('[data-child-session]:visible');
    await reader.waitFor();
    await reader.locator('[data-child-session-message]').first().waitFor();
    assert.match(await reader.innerText(),/3 checks passed/);
    assert.match(await reader.innerText(),/检查编辑器快捷键/);
    assert.equal(await page.locator('[role="dialog"]:visible').count(),0,'A child opens a conversation tab rather than a task dialog');
    assert.equal(await page.locator('[data-chat-input]:visible,[data-chat-send]:visible,[data-machine-chat] textarea:visible').count(),0,'Readonly child tabs never mount a reply composer');
    await page.screenshot({path:path.join(report,name+'-child-conversation.png')});
    const id=await reader.getAttribute('data-child-session-ref');
    if(size.width>=500){
      const childTab=page.locator('[data-session-tabs] [data-tab]').filter({has:page.locator('[data-tab-open]').filter({hasText:'codex-acp'})});
      assert.equal(await childTab.count(),1,'The delegated conversation participates in the ordinary session tab strip');
      await page.locator('[data-session-tabs] [data-tab-open="wide/all"]').click();
      await page.locator('[data-machine-chat-scroll]:visible').waitFor();
      await page.locator('[data-child-task-row]').first().click();
      await reader.waitFor();
      assert.equal(await childTab.count(),1,'Opening the same child focuses its existing tab');
      await childTab.locator('[data-tab-close]').click();
      await page.locator('[data-child-session]').waitFor({state:'detached'});
      assert.equal(await page.locator('[data-session-tabs] [data-tab-open="wide/all"]').count(),1,'Closing the child view keeps the parent tab');
      await page.locator('[data-session-tabs] [data-tab-open="wide/all"]').click();
    }else{
      const deck=page.locator('[data-session-screen][data-deck-at="bottom"] [data-session-bar]:visible');
      await deck.waitFor();
      assert.equal(await deck.locator('[data-deck-pill]').count(),1,'Readonly conversations use the ordinary mobile session pill');
      await page.locator('[data-child-session-parent]:visible').click();
      await page.locator('[data-child-session]').waitFor({state:'detached'});
      const tabs=await page.evaluate(()=>JSON.parse(localStorage.getItem('khor.sessions.tabs')));
      assert(tabs.tabs.includes(id)&&tabs.tabs.includes('wide/all'),'Returning to the parent keeps both ordinary tabs open');
      assert.equal(tabs.active,'wide/all');
    }
    await page.locator('[data-machine-chat-scroll]:visible').waitFor();
  }
  async function checkScroll(nativePan = true) {
    listScroll = await checkListScroll(page, async (phase, values) => {
      listScroll = {...listScroll, ...values};
      await page.screenshot({path:path.join(report,name+'-'+phase+'.png')});
    }, {nativePan});
  }
  async function checkWorkspaceSidebar() {
    const navigation=page.locator('[data-workspace-navigation][data-seat="workspace"]');
    await navigation.waitFor();
    await page.evaluate(async()=>{
      await document.fonts.ready;
      const nav=document.querySelector('[data-workspace-navigation][data-seat="workspace"]');
      let previous=null,stable=0;
      for(let frame=0;frame<120;frame++){
        await new Promise(resolve=>requestAnimationFrame(resolve));
        const rect=nav.getBoundingClientRect();
        const next=[rect.left,rect.top,rect.right,rect.bottom];
        stable=previous&&next.every((value,index)=>Math.abs(value-previous[index])<.01)?stable+1:0;
        if(stable>=3)return;
        previous=next;
      }
      throw new Error('The workspace sidebar never reached stable geometry');
    });
    const layout=await page.evaluate(()=>{
      const nav=document.querySelector('[data-workspace-navigation][data-seat="workspace"]');
      const back=[...document.querySelectorAll('[data-ws-list-back],[data-screen-head] [data-back]')].find(el=>el.getClientRects().length);
      const search=document.querySelector('[data-ws-side-head] [data-search-open]');
      const box=el=>{if(!el)return null;const r=el.getBoundingClientRect();return {left:r.left,right:r.right,center:r.left+r.width/2};};
      return {back:box(back),search:box(search),branch:box(nav?.querySelector('[data-workspace-branch-icon]')),pill:box(nav?.querySelector('[data-glass-capsule]'))};
    });
    workspaceLayout.push(layout);
    assert(layout.back&&layout.search&&layout.branch&&layout.pill,'The actual workspace header and sidebar controls must be present');
    assert(Math.abs(layout.branch.center-layout.back.center)<=1,`The branch icon must align with Back: ${JSON.stringify(layout)}`);
    assert(Math.abs(layout.pill.right-layout.search.right)<=1,`The sidebar pill must align with Search: ${JSON.stringify(layout)}`);
    assert.equal(await navigation.locator('button').count(),3,'Workspace navigation has directory, history and sessions');
    assert.equal(await page.locator('[data-ws-sessions-switch]').count(),0,'The workspace content must not retain a second sessions switch');
    await navigation.locator('[data-workspace-view="sessions"]').click();
    await page.locator('[data-ws-tree] [data-ws-sessions-list]').waitFor();
    assert.equal(await navigation.locator('[aria-pressed=true]').count(),1,'Exactly one sidebar destination is selected');
    assert((await page.locator('[data-ws-tree] [data-ws-sessions-list] [data-row-open]').count())>0,'The workspace sessions must appear in the left sidebar');
    assert.equal(await page.locator('[data-ws-sessions]').count(),0,'Selecting the session list must not open a separate right column');
    const sessionEdges=()=>page.evaluate(()=>{
      const nav=document.querySelector('[data-workspace-navigation][data-seat="workspace"]');
      const box=el=>{const r=el.getBoundingClientRect();return {left:r.left,right:r.right,width:r.width};};
      return {actions:box(document.querySelector('[data-ws-side-head] [data-glass-capsule]')),
        search:box(document.querySelector('[data-ws-side-head] [data-search-open]')),
        pill:box(nav.querySelector('[data-glass-capsule]')),
        items:[...nav.querySelectorAll('[data-glass-item]')].map(box)};
    });
    const oldWidth=await page.addStyleTag({content:'[data-workspace-navigation][data-seat="workspace"][data-with-sessions] > .glass {width:calc(3 * var(--strip-item) + 2 * var(--glass-pad))!important}'});
    const oldEdges=await sessionEdges();
    assert(oldEdges.pill.left-oldEdges.actions.left>1,'The original intrinsic three-slot width reproduces the reported left-edge mismatch');
    await oldWidth.evaluate(el=>el.remove());
    const aligned=await sessionEdges();workspaceLayout.push({sessions:aligned,original:oldEdges});
    assert(Math.abs(aligned.pill.left-aligned.actions.left)<=1,'Sessions pill must share the upper action group left edge');
    assert(Math.abs(aligned.pill.right-aligned.search.right)<=1,'Sessions pill must share Search right edge');
    assert(Math.max(...aligned.items.map(item=>item.width))-Math.min(...aligned.items.map(item=>item.width))<=1,'All three destinations retain equal widths');
    assert(Math.abs(aligned.pill.left-layout.pill.left)<=1,'Switching views must not move the navigation pill');
    await page.screenshot({path:path.join(report,name+'-sessions.png')});
    await navigation.locator('[data-workspace-view="files"]').click();
    await page.locator('[data-file-tree]').waitFor();
  }
  async function checkContentToolbar(phase) {
    const kind = phase.startsWith('file') ? 'file' : phase;
    await page.locator(`[data-session-side] [data-content-toolbar="${kind}"]:visible`).waitFor();
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    const layout = await page.evaluate(kind => {
      const head = document.querySelector('[data-session-tabs]')?.closest('[data-region-head]');
      const toolbar = [...document.querySelectorAll(`[data-session-side] [data-content-toolbar="${kind}"]`)].find(el => el.getClientRects().length);
      const box = el => el ? {top:el.getBoundingClientRect().top,bottom:el.getBoundingClientRect().bottom} : null;
      const controls = toolbar ? [...toolbar.querySelectorAll('button')].filter(el => el.getClientRects().length && !el.matches(':disabled,[aria-disabled="true"]')) : [];
      const contentHead=document.querySelector('[data-session-content-head]')?.closest('[data-region-head]');
      return {head:box(head),contentHead:box(contentHead),nested:!!head?.querySelector('[data-content-tabs]'), toolbar:box(toolbar), side:box(document.querySelector('[data-session-side]')),
        seam:box(document.querySelector('[data-session-side] [data-side-seam]')),
        panes:box(document.querySelector('[data-session-panes]')), controlsReachable:controls.length > 0 && controls.every(el => {
        const rect = el.getBoundingClientRect();
        return el.contains(document.elementFromPoint(rect.x + rect.width / 2, rect.y + rect.height / 2));
      })};
    }, kind);
    contentLayout.push({phase,...layout});
    assert(layout.head && layout.toolbar, 'The actual session header and content toolbar must be present');
    assert(layout.toolbar.top >= layout.head.bottom - 1, `The content toolbar must stay below the session tabs (${phase}): ${JSON.stringify(layout)}`);
    assert(layout.side && layout.seam && layout.panes, 'The actual split column and resize handle must be present');
    assert(layout.contentHead&&!layout.nested&&Math.abs(layout.contentHead.top-layout.head.top)<=1&&Math.abs((layout.contentHead.bottom-layout.contentHead.top)-(layout.head.bottom-layout.head.top))<=1, `Independent column headers align (${phase}): ${JSON.stringify(layout)}`);
    assert(Math.abs(layout.side.top-layout.head.top)<=1, `The content column starts at the same top edge (${phase}): ${JSON.stringify(layout)}`);
    assert(Math.abs(layout.seam.top-layout.head.top)<=1, `The content resize handle starts at the shared header edge (${phase}): ${JSON.stringify(layout)}`);
    assert(layout.side.bottom <= layout.panes.bottom + 1, `The inset column must stay inside the session body (${phase}): ${JSON.stringify(layout)}`);
    assert(layout.controlsReachable, `The session tabs must not cover the content controls (${phase})`);
    await page.screenshot({path:path.join(report,`${name}-${phase}.png`)});
  }
  page.on('pageerror', error => errors.push(String(error)));
  page.on('console', message => {
    if (message.type() === 'error' && message.text().includes('[Lab fixture]')) errors.push(message.text().split('\n')[0]);
  });
  try {
    // Select data only; every layout and action belongs to the production App.
    await page.route('**/lab/runtime*', route => route.fulfill({contentType: 'application/json', body: JSON.stringify({mode: 'frontend', scene, tree: 'browser-check', revision: 'one'})}));
    await page.goto(`http://127.0.0.1:${port}/app/?scene=${scene}&theme=dark&machines=${machines}&items=${items}`);
    const selector = scene === 'page' ? '[data-content-visible="true"] .mobile-content-page' : scene === 'list' ? '[data-list]' : scene === 'terminal' ? '[data-terminal]'
      : ['content','workspace'].includes(scene) ? '.cm-editor, [data-file-preview]' : scene === 'quota' ? '[data-quota-row]' : ['devices','workspaces'].includes(scene) ? '[data-list]' : '[data-chat-box]';
    await page.waitForSelector(selector);
    if(data.nativeMarker){
      await page.locator('[data-ws-row]').first().click();
      await page.locator('[data-workspace-navigation]').waitFor();
      assert.deepEqual(await page.evaluate(()=>window.__labNativeCalls),[],'An embedded frontend Lab must use only its isolated HTTP fixture');
      assert.equal(await page.evaluate(()=>('__TAURI_INTERNALS__' in window)),true,'The page must not delete or replace its host runtime');
    }
    if (scene === 'page') {
      const body = page.locator('[data-content-visible="true"]');
      await page.evaluate(() => {
        let title = false;
        window.addEventListener('pointerdown', event => {
          title = event.target instanceof Element && !!event.target.closest('[data-deck-title]');
        }, {capture: true, once: true});
        window.addEventListener('click', () => {
          window.__labAddressGestureFocus = {title, focused:document.activeElement?.matches('[data-page-address]')};
        }, {once: true});
      });
      await page.screenshot({path:path.join(report,name+'-page.png')});
      await body.locator('[data-deck-title]').first().tap();
      const focus = await page.evaluate(() => window.__labAddressGestureFocus);
      await writeFile(path.join(report,name+'-focus.json'), JSON.stringify(focus,null,2));
      assert.equal(await page.locator('[data-fault]:visible').count(), 0, 'Editing must not render an error boundary');
      assert.equal(focus.title, true, 'The gesture must begin on the actual address title');
      assert.equal(focus.focused, true, 'The address input must focus during the initiating tap, before animation frames');
      await page.screenshot({path:path.join(report,name+'-address.png')});
      await body.locator('[data-content-edit-end]').click();
      const before = await page.evaluate(async () => (await fetch('/api/tabs_list', {method: 'POST', body: JSON.stringify({kind: 'content'})})).json());
      const back = body.locator('[data-content-back]');
      assert.equal(await back.count(), 1, 'A session webpage must expose Back to its content picker');
      await back.click();
      await page.locator('[data-content-picker]:visible').waitFor();
      const after = await page.evaluate(async () => (await fetch('/api/tabs_list', {method: 'POST', body: JSON.stringify({kind: 'content'})})).json());
      assert.deepEqual(after.tabs.map(tab => tab.id), before.tabs.map(tab => tab.id), 'Back must preserve the content tab identities');
      const picker = page.locator('[data-content-picker]:visible');
      if (!(await picker.locator('.mobile-content-browser:not([hidden])').isVisible())) {
        await picker.locator('[data-content-scope]').click();
        await page.locator('[role="menuitem"][data-content-source="browser"]').click();
      }
      await picker.locator(`[data-place="http://127.0.0.1:${port}/lab/example"]`).click();
      await page.locator('[data-content-visible="true"] .mobile-content-page').waitFor();
      await page.locator('[data-content-visible="true"] [data-content-back]').click();
      await picker.locator('[data-content-scope]').click();
      await page.locator('[role="menuitem"][data-content-source="workspace"]').click();
      await picker.locator('[data-file-path="/preview/project/README.md"]').click();
      await page.locator('[data-content-visible="true"] .cm-editor, [data-content-visible="true"] [data-file-preview]').waitFor();
      const final = await page.evaluate(async () => (await fetch('/api/tabs_list', {method: 'POST', body: JSON.stringify({kind: 'content'})})).json());
      assert.deepEqual(final.tabs.map(tab => tab.id), before.tabs.map(tab => tab.id), 'Recent pages and workspace files must reuse the same content tab');
    }
    if (scene === 'content') {
      const document = await page.locator('.cm-editor, [data-file-preview]').first().innerText();
      if (!document.includes('fixture data')) errors.push('Actual file body is missing');
      if (size.width >= 500) {
        // Click real conversation links, not the content picker's in-tab road.
        const tabs=()=>page.evaluate(async()=>(await(await fetch('/api/tabs_list',{method:'POST',body:JSON.stringify({kind:'content'})})).json()).tabs);
        const beforeLinks=await tabs(),fileTab=beforeLinks.find(tab=>tab.url==='/preview/project/README.md');
        assert(fileTab,'The first clicked file is a durable content tab');
        const messageUrl=`http://127.0.0.1:${port}/lab/example?from=message`;
        await page.locator(`[data-md-link="${messageUrl}"]`).click();
        await page.locator('[data-content-visible="true"] .mobile-content-page').waitFor();
        await page.waitForFunction(async url => {
          const {tabs}=await (await fetch('/api/tabs_list',{method:'POST',body:JSON.stringify({kind:'content'})})).json();
          return tabs.some(tab=>tab.url===url);
        },messageUrl);
        const afterPage=await tabs();
        assert.equal(afterPage.find(tab=>tab.id===fileTab.id)?.url,fileTab.url,'A second message link must not replace the first document');
        assert.equal(afterPage.length,beforeLinks.length+1,'The previously unopened webpage has its own tab');
        await page.locator('[data-link="/preview/project/README.md"]').click();
        await page.locator('[data-content-visible="true"] .cm-editor, [data-content-visible="true"] [data-file-preview]').waitFor();
        assert.equal((await tabs()).find(tab=>tab.id===fileTab.id)?.url,fileTab.url,'Reopening a message link selects its existing file');
        await checkContentToolbar('file');
        for (const width of [1024,1280]) {
          await page.setViewportSize({width,height:size.height});
          await checkContentToolbar(`file-${width}`);
        }
        await page.locator('[data-session-side] [data-content-back]:visible').click();
        await checkContentToolbar('picker');
        const picker = page.locator('[data-content-picker]:visible');
        await picker.locator('[data-content-source="browser"]').click();
        await picker.locator(`[data-place="http://127.0.0.1:${port}/lab/example"]`).click();
        await checkContentToolbar('page');
      }
    }
    if (size.width < 500 && ['gui', 'codex'].includes(scene)) await page.waitForSelector('[data-session-compose-row]');
    await page.waitForTimeout(800);
    assert.equal(await page.locator('[data-fault]:visible').count(), 0, 'The production App must render without an error boundary');
    if (scene === 'workspace' && size.width < 500) {
      assert.equal(await page.locator('[data-ws-tab-strip]:visible').count(),0,'Mobile workspace files must use the shared bottom pill, without a top tab strip');
      const deck = page.locator('[data-ws-mid] [data-session-bar]');
      assert.equal(await deck.count(),1,'Mobile workspace content must mount the production shared deck');
      const rect = await deck.boundingBox();
      assert(rect && rect.y >= size.height / 2 && rect.y + rect.height <= size.height + 1,'Workspace deck must sit inside the bottom viewport');
    }
    if(scene==='workspace'&&size.width>=500)await checkWorkspaceSidebar();
    if (['list','devices'].includes(scene)) {
      if (machines === 3 && size.width < 500) await checkScroll(false);
      const count = await page.evaluate(async()=> (await (await fetch('/api/devices',{method:'POST',body:'{}'})).json()).length);
      assert.equal(count,machines,'Machine fixture count must reach the actual App');
      assert.equal(await page.locator('[data-alone]:visible,[data-machine-add-guidance]:visible').count(),machines===0?1:0,'Only a zero-machine list offers add-machine guidance');
      if (scene==='list' && machines===3) {
        const row=page.locator(size.width<500?'[data-screen="list"] [data-row="wide/all"] [data-row-open]':'[data-row="wide/all"] [data-row-open]');
        const control=row.locator('[data-side-disclosure]');
        await page.waitForFunction(()=>document.querySelector('[data-side-disclosure="wide/all"]')?.getAttribute('aria-label')==='3 个子会话');
        assert.equal(await page.locator('[data-unread-count]').count(),0,'The list does not show unread numbers');
        if(size.width>=500){await control.hover();await control.focus();}
        const spacing=await control.evaluate(button=>{
          const rect=button.getBoundingClientRect(),title=button.closest('[data-side-title]').getBoundingClientRect();
          const sub=button.closest('[data-row-open]').querySelector('[data-side-sub]').getBoundingClientRect();
          const style=getComputedStyle(button);
          return {top:rect.top,bottom:rect.bottom,titleTop:title.top,titleBottom:title.bottom,subTop:sub.top,paddingLeft:parseFloat(style.paddingLeft),paddingRight:parseFloat(style.paddingRight)};
        });
        assert(spacing.top>=spacing.titleTop-0.5&&spacing.bottom<=spacing.titleBottom+0.5,'The disclosure hover/focus box reserves its height inside the title line');
        assert(spacing.bottom<=spacing.subTop+0.5,'The disclosure does not cover the session summary');
        assert(spacing.paddingLeft>0&&spacing.paddingRight>0,'The label and chevron have horizontal breathing room');
        assert.equal(await control.locator('svg').count(),1,'The text action shows its own chevron beside the name');
        await page.screenshot({path:path.join(report,name+'-disclosure.png')});
      }
      if (scene==='list' && size.width < 500 && machines===3) {
        const parent=page.locator('[data-screen="list"] [data-row="wide/all"] [data-row-open]');
        const disclosure=page.locator('[data-screen="list"] [data-side-disclosure="wide/all"]');
        assert.equal((await disclosure.innerText()).trim(),'3 个子会话');
        assert.equal(await disclosure.evaluate(button=>button.parentElement.closest('button')===null),true,'Independent actions are never nested inside the row button');
        const controlBounds=await disclosure.boundingBox();assert(controlBounds&&controlBounds.width>=44&&controlBounds.height>=44,'Mobile disclosure keeps a full touch target');
        await disclosure.click();
        await page.locator('[data-child-tasks="wide/all"] [data-child-task-row]').first().waitFor();
        assert.equal(await disclosure.getAttribute('aria-expanded'),'true');
        assert.equal((await disclosure.innerText()).trim(),'收起');
        const childFaces=page.locator('[data-child-tasks="wide/all"] [data-child-task-face] [data-side-face]');
        assert.equal(await childFaces.count(),4,'Agents and command records reuse the session avatar assembly');
        const devices=await page.evaluate(async()=> (await (await fetch('/api/devices',{method:'POST',body:'{}'})).json()));
        assert.deepEqual(await page.locator('[data-child-task-face]').evaluateAll(nodes=>nodes.slice(0,3).map(node=>node.dataset.childTaskFace)),devices.slice(0,3).map(device=>device.id),'Child avatars resolve the stable machine identity rather than a display name');
        assert.equal(await page.locator('[role="dialog"]:visible').count(),0,'Expanding tasks stays in the session list');
        assert.equal(await parent.isVisible(),true,'The parent remains visible above its inline children');
        await page.screenshot({path:path.join(report,name+'-child-tree.png')});
        await page.locator('[data-child-task-row]').first().click();
        await checkChildConversation();
        await page.locator('[data-lab-model-back]:visible').click();
        await parent.waitFor({state:'visible'});
        assert.equal(await disclosure.getAttribute('aria-expanded'),'false','Returning to the mobile list restores its ordinary collapsed rows');
        await disclosure.click();
        await page.locator('[data-child-tasks="wide/all"] [data-child-task-row]').first().waitFor();
        await checkCommandTab(page,{report,name,mobile:true});
        await page.locator('[data-lab-model-back]:visible').click();
        await parent.waitFor({state:'visible'});
        await disclosure.click();
        await page.locator('[data-child-tasks="wide/all"] [data-child-task-row]').first().waitFor();
        await disclosure.click();
        await page.locator('[data-child-tasks="wide/all"]').waitFor({state:'detached'});
        assert.equal((await disclosure.innerText()).trim(),'3 个子会话');
        await parent.locator('[data-side-name]').click();
        await page.locator('[data-machine-chat-scroll]:visible').waitFor();
        await page.locator('[data-lab-model-back]:visible').click();
        await parent.waitFor({state:'visible'});
        await parent.locator('[data-side-name]').click();
        await page.locator('[data-machine-chat-scroll]:visible').waitFor();
        assert.equal(await page.locator('[data-child-task-row]').count(),0,'Returning from the mobile list opens the selected conversation before its child tasks');
        await page.locator('[data-lab-model-back]:visible').click();
        await parent.waitFor({state:'visible'});
      }
      if (scene==='list' && size.width >= 500 && machines===3) {
        const parent=page.locator('[data-row="wide/all"] [data-row-open]');
        await parent.locator('[data-side-name]').click();
        await parent.locator('[data-side-name]').click();
        const tasks=page.locator('[data-child-task-row]');
        await tasks.first().waitFor();
        assert.equal(await tasks.count(),4,'The active Khor row opens its agent and command tasks');
        assert.equal(await page.locator('[role="dialog"]:visible').count(),0,'Task branches expand in the original list');
        assert.equal(await page.locator('[data-side-list] [data-child-tasks="wide/all"]').count(),1);
        await page.screenshot({path:path.join(report,name+'-child-tree.png')});
        await tasks.first().click();
        await checkChildConversation();
        await tasks.first().waitFor();
        await checkCommandTab(page,{report,name,mobile:false});
        await page.locator('[data-side-disclosure="wide/all"]').click();
        await tasks.first().waitFor({state:'detached'});
        await parent.focus();
        assert.equal(await parent.locator('[data-side-open]').evaluate(button=>document.activeElement===button),true,'Programmatic/menu focus returns to the native primary control');
        await page.keyboard.press('ArrowDown');
        const disclosure=page.locator('[data-side-disclosure="wide/all"]');
        assert.equal(await disclosure.evaluate(button=>document.activeElement===button),true,'Arrow navigation reaches the independent title action');
        await page.keyboard.press('Space');await tasks.first().waitFor();
        await page.keyboard.press('Space');await tasks.first().waitFor({state:'detached'});
        await parent.locator('[data-editable-name]').first().dblclick();
        await parent.locator('[data-title-input]').waitFor();
        await page.keyboard.press('Escape');await parent.locator('[data-title-input]').waitFor({state:'detached'});
      }
      if(scene==='devices'){
        assert.equal(await page.locator('[data-mandala-map]').count(),0,'An unselected machine page must leave its detail area empty');
        assert.equal(await page.locator('[data-machine-card]').count(),0,'The machine overview must not silently select the first machine');
      }
      if(scene==='devices'&&machines===6){
        const devices=await page.evaluate(async()=> (await fetch('/api/devices',{method:'POST',body:'{}'})).json());
        assert.deepEqual(devices.map(device=>device.vitals?.vitals.system),['macos','linux','macos','windows','ios','android']);
        for(const device of devices){
          const row=page.locator(`[data-device="${device.name}"]`);
          assert.equal(await row.locator('[data-side-mark]').getAttribute('data-side-mark'),device.vitals.vitals.system,'Machine marks must come from the actual system facts');
          assert.equal(device.face.radius_ratio,.5,'Default fixture avatars use the real generator circle crop');
          assert.equal(await row.locator('[data-face]').first().evaluate(el=>el.style.borderRadius),'50%');
        }
      }
    }
    if(scene==='quota'){
      assert((await page.locator('[data-quota-row]:visible').count())>=6,'The actual quota page must render balances, windows and unavailable account examples');
      await page.locator('[data-quota-row="fixture"]:visible').click();
      await page.locator('[data-quota-detail="fixture"]:visible').waitFor();
    }
    if(scene==='workspaces'&&!data.nativeMarker){
      await checkScroll();
      assert.equal(await page.locator('[data-ws-row]:visible').count(),machines*4,'Each machine supplies distinct Git and plain-folder workspaces');
      await page.screenshot({path:path.join(report,name+'-list.png')});
      await page.locator('[data-ws-row]:visible').first().click();
      await checkWorkspaceSidebar();
      const names=await page.evaluate(async()=> (await (await fetch('/api/ls',{method:'POST',body:JSON.stringify({machine:'e'.repeat(64),path:'/preview/project'})})).json()).entries.map(entry=>entry.name));
      assert.deepEqual(names,['docs','media','src','design.md','notes.md','package.json','plan.md','README.md','report.md','.config','.gitignore'],'Generated ls data must retain the real backend directory and dotfile ordering');
      const visible=await page.locator('[data-file-tree] button[data-file-path]').evaluateAll(rows=>rows.map(row=>row.getAttribute('data-file-path').split('/').pop()));
      assert.deepEqual(visible,names,'The actual file tree must display the backend order');
      await page.locator('[data-workspace-navigation]').getByRole('button',{name:'提交历史',exact:true}).click();
      const commits=page.locator('[data-workspace-log-commit]');
      await commits.first().waitFor();
      const first=await commits.first().getAttribute('data-workspace-log-commit');
      await page.locator('[data-workspace-log-older]').click();
      await page.waitForFunction(first=>document.querySelector('[data-workspace-log-commit]')?.getAttribute('data-workspace-log-commit')!==first,first);
      await commits.first().click();
      await page.locator('[data-commit-page]:visible').waitFor();
      assert.equal(await page.locator('[data-ws-tab-strip]:visible').count(),0,'Mobile commits also retain the shared bottom deck');
      assert.equal(await page.locator('[data-ws-mid] [data-session-bar]:visible').count(),1);
    }
    geometry = await page.evaluate(() => {
      const row = document.querySelector('[data-session-compose-row]');
      const avatar = document.querySelector('[data-lab-compose-avatar]');
      const capsule = document.querySelector('[data-chat-box]');
      const box = el => el ? {top: el.getBoundingClientRect().top, bottom: el.getBoundingClientRect().bottom} : null;
      return {height: innerHeight, row: box(row), avatar: box(avatar), capsule: box(capsule), singleLine:!!capsule&&!capsule.hasAttribute('data-lab-compose-expanded'),composerSearch:document.querySelectorAll('[data-chat-box] [data-history-find-open]').length,avatarOutsideCapsule: !!avatar && !capsule?.contains(avatar), overflow: document.documentElement.scrollWidth > innerWidth + 1};
    });
    if (size.width < 500 && ['gui', 'codex'].includes(scene)) {
      if (!geometry.avatarOutsideCapsule) errors.push('Avatar must be the production sibling of the composer capsule');
      if (geometry.row?.bottom > geometry.height + 1) errors.push('Production composer falls below the viewport');
      assert.equal(geometry.composerSearch,0,'The composer no longer contains a search control');
      if(geometry.singleLine){assert(Math.abs(geometry.avatar.bottom-geometry.capsule.bottom)<1,'Avatar and composer bottom edges align');assert(Math.abs((geometry.avatar.bottom-geometry.avatar.top)-(geometry.capsule.bottom-geometry.capsule.top))<1,'Single-line composer and session control share one height');}
    }
    if(data.fine){
      const stale=await page.addStyleTag({content:'[data-lab-composer-session-slot],[data-lab-compose-avatar]{width:var(--glass-btn)!important;height:var(--glass-btn)!important}'});
      const mismatch=await page.evaluate(()=>Math.abs(document.querySelector('[data-chat-box]').getBoundingClientRect().height-document.querySelector('[data-lab-compose-avatar]').getBoundingClientRect().height));
      assert(mismatch>1,'The original fine-pointer sizing reproduces the reported mismatch');await stale.evaluate(el=>el.remove());
      const field=page.locator('[data-composer-text]');await field.fill('第一行\n第二行');await page.locator('[data-lab-compose-expanded]').waitFor();
      const edges=await page.evaluate(()=>({input:document.querySelector('[data-chat-box]').getBoundingClientRect().bottom,avatar:document.querySelector('[data-lab-compose-avatar]').getBoundingClientRect().bottom}));assert(Math.abs(edges.input-edges.avatar)<1,'Multiline input retains bottom alignment');await page.screenshot({path:path.join(report,name+'-multiline.png')});await field.fill('');
    }
    if (geometry.overflow) errors.push('App overflows horizontally');
    if(data.pairing)await checkMachinePairing(page,phase=>page.screenshot({path:path.join(report,`${name}-${phase}.png`)}),message=>{
      const at=errors.indexOf(message);
      assert(at>=0,'The deliberately invalid credential must produce the expected handled fixture error');
      expectedConsoleErrors.push(message);errors.splice(at,1);
    });
    if(data.setup)await checkAgentSetup(page,size.width<500,phase=>page.screenshot({path:path.join(report,`${name}-${phase}.png`)}));
    if(data.gesture)await checkMobileNavigation(page,scene,items,async(phase,values)=>{
      if(values)contentLayout.push({phase,...values});
      await page.screenshot({path:path.join(report,`${name}-${phase}.png`)});
    },data.gesture===true?'browse':data.gesture);
  } catch (error) {
    errors.push(String(error).slice(0, 1200));
    await writeFile(path.join(report, name + '-body.txt'), (await page.locator('body').innerText()).slice(0, 20000));
  } finally {
    await page.screenshot({path: path.join(report, name + '.png')});
    await context.close();
  }
  return {name, geometry, contentLayout, workspaceLayout, listScroll, expectedConsoleErrors, errors};
}

let browser;
const results = [];
try {
  if (process.env.LAB_CHECKS === 'conversation-navigation') {
    for(const engine of ['chromium',...(process.env.QUALITY_ENGINES?.includes('webkit')?['webkit']:[])]) {
      browser=await (engine==='webkit'?webkit:chromium).launch({headless:engine!=='webkit'||process.env.QUALITY_HEADLESS!=='0'});
      for(const width of [320,390])for(const workspace of [false,true])for(const khor of [true,false]) {
        results.push(await checkConversationNavigation(browser,{port,report,engine,workspace,khor,width}));
        await writeFile(path.join(report,'results.json'),JSON.stringify(results,null,2));
      }
      await browser.close();browser=undefined;
    }
  } else if (process.env.LAB_CHECKS === 'child-tree') {
    for (const engine of ['chromium', ...(process.env.QUALITY_ENGINES?.includes('webkit') ? ['webkit'] : [])]) {
      browser = await (engine === 'webkit' ? webkit : chromium).launch({headless:engine !== 'webkit' || process.env.QUALITY_HEADLESS !== '0'});
      for (const size of [{width:1280,height:900},{width:390,height:844}]) {
        results.push(await inspect(browser,'list',size,{machines:3,engine}));
        await writeFile(path.join(report,'results.json'),JSON.stringify(results,null,2));
        results.push(await checkWorkspaceSessions(browser,{port,report,engine,mobile:size.width<500,khor:true}));
        await writeFile(path.join(report,'results.json'),JSON.stringify(results,null,2));
      }
      await browser.close();
      browser = undefined;
    }
  } else {
  for (let i = 0; i < 100; i++) {
    try {if ((await fetch(`http://127.0.0.1:${port}/lab/status`)).ok) break;} catch {}
    await new Promise(r => setTimeout(r, 100));
  }
  browser = await chromium.launch({headless: true});
  for(const workspace of [false,true])for(const khor of [true,false])results.push(await checkConversationNavigation(browser,{port,report,engine:'chromium',workspace,khor}));
  results.push(await checkWorkspaceBack(browser,{port,report}));
  for(const mobile of [false,true])results.push(await checkResourceFiles(browser,{port,report,mobile,engine:'chromium'}));
  results.push(await checkSettingsNavigation(browser,{port,report,engine:'chromium'}));
  results.push(await checkSettingsNavigation(browser,{port,report,engine:'chromium',mobile:true}));
  results.push(await checkGuiExperience(browser,{port,report,engine:'chromium'}));
  results.push(await checkWorkspaceSessions(browser,{port,report,engine:'chromium'}));
  results.push(await checkWorkspaceSessions(browser,{port,report,engine:'chromium',mobile:true}));
  results.push(await checkWorkspaceSessions(browser,{port,report,engine:'chromium',mobile:true,khor:true}));
  results.push(await inspect(browser,'workspaces',{width:1280,height:900},{machines:3,nativeMarker:true}));
  await writeFile(path.join(report,'results.json'),JSON.stringify(results,null,2));
  for (const size of [{width: 1280, height: 900}, {width: 390, height: 844}, {width: 320, height: 568}]) {
    for (const scene of ['gui', 'khor', 'codex', 'terminal', 'content', 'workspace', ...(size.width < 500 ? ['page'] : []), 'list', 'devices']) {
      results.push(await inspect(browser, scene, size));
      await writeFile(path.join(report, 'results.json'), JSON.stringify(results, null, 2));
    }
  }
  for (const machines of [0,3]) for (const scene of ['list','devices']) {
    results.push(await inspect(browser,scene,{width:390,height:844},{machines}));
    await writeFile(path.join(report,'results.json'),JSON.stringify(results,null,2));
  }
  results.push(await inspect(browser,'list',{width:1280,height:900},{machines:3}));
  await writeFile(path.join(report,'results.json'),JSON.stringify(results,null,2));
  for(const scene of ['devices','workspaces','quota']){
    results.push(await inspect(browser,scene,{width:390,height:844},{machines:6}));
    await writeFile(path.join(report,'results.json'),JSON.stringify(results,null,2));
  }
  for(const items of [2,3,5])for(const scene of ['terminal','gui','content']) {
    results.push(await inspect(browser,scene,{width:390,height:844},{items,gesture:true}));
    await writeFile(path.join(report,'results.json'),JSON.stringify(results,null,2));
  }
  for(const scene of ['terminal','gui','content']) {
    results.push(await inspect(browser,scene,{width:390,height:844},{items:2,gesture:'create'}));
    await writeFile(path.join(report,'results.json'),JSON.stringify(results,null,2));
  }
  results.push(await inspect(browser,'gui',{width:496,height:844},{fine:true}));
  await writeFile(path.join(report,'results.json'),JSON.stringify(results,null,2));
  for(const size of [{width:1280,height:900},{width:390,height:844}]){
    results.push(await inspect(browser,'terminal',size,{items:2,setup:true}));
    await writeFile(path.join(report,'results.json'),JSON.stringify(results,null,2));
  }
  for(const size of [{width:1280,height:900},{width:900,height:440},{width:390,height:844},{width:390,height:400}]){
    results.push(await inspect(browser,'devices',size,{machines:1,pairing:true}));
    await writeFile(path.join(report,'results.json'),JSON.stringify(results,null,2));
  }
  if (process.env.QUALITY_ENGINES?.includes('webkit')) {
    await browser.close();
    browser = await webkit.launch({headless:process.env.QUALITY_HEADLESS!=='0'});
    for(const workspace of [false,true])for(const khor of [true,false])results.push(await checkConversationNavigation(browser,{port,report,engine:'webkit',workspace,khor}));
    for(const mobile of [false,true]){results.push(await checkResourceFiles(browser,{port,report,mobile,engine:'webkit'}));await writeFile(path.join(report,'results.json'),JSON.stringify(results,null,2));}
    results.push(await checkSettingsNavigation(browser,{port,report,engine:'webkit'}));
    results.push(await checkSettingsNavigation(browser,{port,report,engine:'webkit',mobile:true}));
    results.push(await checkGuiExperience(browser,{port,report,engine:'webkit'}));
    results.push(await checkWorkspaceSessions(browser,{port,report,engine:'webkit'}));
    results.push(await checkWorkspaceSessions(browser,{port,report,engine:'webkit',mobile:true}));
    results.push(await checkWorkspaceSessions(browser,{port,report,engine:'webkit',mobile:true,khor:true}));
    results.push(await inspect(browser,'workspaces',{width:1280,height:900},{machines:3,nativeMarker:true,engine:'webkit'}));
    results.push(await inspect(browser,'list',{width:1280,height:900},{machines:3,engine:'webkit'}));
    await writeFile(path.join(report,'results.json'),JSON.stringify(results,null,2));
    for (const scene of ['workspaces','list','devices']) {
      results.push(await inspect(browser,scene,{width:390,height:844},{machines:scene === 'workspaces' ? 6 : 3,engine:'webkit'}));
      await writeFile(path.join(report,'results.json'),JSON.stringify(results,null,2));
    }
  }
  for(const mobile of [false,true])for(const short of [false,true]){
    results.push(await checkLongHistory(browser,{port,report,mobile,short,engine:process.env.QUALITY_ENGINES?.includes('webkit')?'webkit':'chromium'}));
    await writeFile(path.join(report,'results.json'),JSON.stringify(results,null,2));
  }
  }
  console.log(JSON.stringify(results.map(({name, errors}) => ({name, errors}))));
  if (results.some(result => result.errors.length)) process.exitCode = 1;
} finally {
  await browser?.close();
  server.kill('SIGTERM');
}
