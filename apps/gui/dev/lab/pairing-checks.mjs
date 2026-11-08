import assert from 'node:assert/strict';

export async function checkMachinePairing(page, capture) {
  const fixture=async body=>page.evaluate(async body=>{
    const response=await fetch('/api/__ai_preview/pairing',{method:'POST',body:JSON.stringify(body)});
    if(!response.ok)throw new Error('Pairing fixture unavailable');
    return response.json();
  },body);
  const plus=()=>page.locator('[data-pane-new]:visible').first();
  const choose=async kind=>{
    await plus().click();
    assert.equal(await page.locator('[role="dialog"]:visible').count(),0,'+ first opens a nearby menu');
    assert.equal(await page.locator('[data-new-item="khor"]:visible').count(),1);
    assert.equal(await page.locator('[data-new-item="ssh"]:visible').count(),1);
    await capture('choices');
    await page.locator(`[data-new-item="${kind}"]:visible`).click();
    return page.locator('[role="dialog"]:visible');
  };
  const fit=async dialog=>{
    const facts=await dialog.evaluate(async el=>{
      let previous=null,stable=0;
      for(let frame=0;frame<120;frame++) {
        await new Promise(resolve=>requestAnimationFrame(resolve));
        const rect=el.getBoundingClientRect(),next=[rect.left,rect.top,rect.right,rect.bottom];
        stable=previous&&next.every((value,index)=>Math.abs(value-previous[index])<.01)?stable+1:0;
        if(stable>=3)break;
        previous=next;
      }
      const r=el.getBoundingClientRect();
      return {left:r.left,right:r.right,top:r.top,bottom:r.bottom,width:innerWidth,height:innerHeight,overflow:el.scrollWidth>el.clientWidth};
    });
    assert(!facts.overflow&&facts.left>=-1&&facts.right<=facts.width+1&&facts.top>=-1&&facts.bottom<=facts.height+1,
      `Pairing form stays inside its visible viewport: ${JSON.stringify(facts)}`);
  };
  for(const theme of ['dark','light']) {
    await page.evaluate(theme=>{document.documentElement.dataset.theme=theme;},theme);
    let dialog=await choose('khor');
    const own=dialog.locator('[data-ticket]');await own.waitFor();
    assert.equal(await own.getAttribute('readonly'),'');
    assert.match(await own.inputValue(),/^khorLAB_ONLY_/);
    assert.equal(await dialog.locator('[data-ticket-input]').count(),0,'Invite never exposes an entry field');
    assert.equal(await dialog.locator('[data-ticket-qr]').count(),1);
    await fit(dialog);await capture(theme+'-invite');
    if(theme==='dark') {
      await page.context().grantPermissions(['clipboard-read','clipboard-write']);
      await dialog.getByRole('button',{name:'复制配对凭据',exact:true}).click();
      await dialog.getByRole('button',{name:'已复制',exact:true}).waitFor();
      assert.equal(await page.evaluate(()=>navigator.clipboard.readText()),await own.inputValue());
    }
    const before=await fixture({});
    await dialog.getByRole('radio',{name:'加入网络',exact:true}).click();
    assert.equal(await dialog.locator('[data-ticket]').count(),0);
    assert.equal(await dialog.locator('[data-ticket-qr]').count(),0,'Join never displays the local invitation');
    const join=dialog.locator('[data-join]');assert(await join.isDisabled());
    const input=dialog.locator('[data-ticket-input]');
    await input.fill('invalid-credential');await join.click();
    await dialog.locator('[data-dialog-error]').waitFor();
    assert.match(await dialog.locator('[data-dialog-error]').innerText(),/配对凭据无效/);
    assert.equal(await input.inputValue(),'invalid-credential','Failed pairing retains the credential to fix');
    await dialog.getByRole('radio',{name:'邀请机器',exact:true}).click();
    await own.waitFor();assert.equal((await fixture({})).invitations,before.invitations,'Returning to Invite reuses its current invitation');
    await dialog.getByRole('radio',{name:'加入网络',exact:true}).click();
    assert.equal(await input.inputValue(),'invalid-credential','Mode changes retain the unsent join input');
    await fit(dialog);await capture(theme+'-join');
    const {remoteTicket}=await fixture({pending:true});
    await input.fill(remoteTicket);await join.click();
    const outcome=dialog.locator('[data-pair-outcome]');await outcome.waitFor();
    assert.match(await outcome.innerText(),/等待网络批准/,'A pairing response may require approval, never call it connected');
    await capture(theme+'-pending');
    await dialog.getByRole('button',{name:'完成',exact:true}).click();
    await fixture({pending:false});
    dialog=await choose('khor');await dialog.locator('[data-ticket]').waitFor();
    await dialog.getByRole('radio',{name:'加入网络',exact:true}).click();
    await dialog.locator('[data-ticket-input]').fill(remoteTicket);await dialog.locator('[data-join]').click();
    await dialog.locator('[data-pair-outcome]').waitFor();
    assert.equal(await dialog.locator('[data-pair-outcome]').innerText(),'已加入示例网络');
    await capture(theme+'-joined');
    await dialog.getByRole('button',{name:'完成',exact:true}).click();
    dialog=await choose('ssh');
    await dialog.locator('[data-add-group="ssh"]').waitFor();
    assert.equal(await dialog.locator('[data-pairing-mode]').count(),0,'SSH opens its own form directly');
    await fit(dialog);await capture(theme+'-ssh');
    await dialog.locator('[data-dialog-cancel]').click();
  }
}
