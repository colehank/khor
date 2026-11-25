import assert from 'node:assert/strict';
import path from 'node:path';

export async function checkWorkspaceBranches(browser, {port,report,engine,width}) {
  const mobile=width<500;
  const context=await browser.newContext({viewport:{width,height:mobile?844:900},isMobile:mobile,hasTouch:mobile});
  const name=`workspace-branches-${engine}-${width}`;
  const page=await context.newPage(), errors=[];
  page.on('pageerror',error=>errors.push(error.message));
  const activate=locator=>mobile?locator.tap():locator.click();
  try {
    await page.goto(`http://127.0.0.1:${port}/app/?scene=workspaces&machines=1`);
    await activate(page.locator('[data-ws-row]:visible').first());
    const nav=page.locator('[data-workspace-navigation][data-seat="workspace"]');
    const picker=nav.locator('[data-workspace-branch-picker]');
    const label=picker.locator('[data-popup-value]');
    const option=value=>page.locator(`[data-popup-option="${value}"]`);
    const commits=page.locator('[data-workspace-log-commit]');
    const first=commits.first();
    await page.locator('[data-file-tree] [data-tree-entry]:visible').first().waitFor();
    await label.filter({hasText:/^main$/}).waitFor();
    const filesBefore=await page.locator('[data-file-tree] button[data-file-path]').evaluateAll(rows=>rows.map(row=>row.getAttribute('data-file-path')));
    const spacing=await picker.evaluate(async button=>{
      // On the narrow face the panel slides in beneath the stationary Back button.
      await Promise.allSettled(document.getAnimations()
        .filter(animation=>animation.effect?.getTiming().iterations===1)
        .map(animation=>animation.finished));
      const icon=button.querySelector('[data-workspace-branch-icon] svg');
      const name=button.querySelector('[data-popup-value]');
      const back=[...document.querySelectorAll('[data-ws-list-back],[data-screen-head] [data-back]')].find(el=>el.getClientRects().length);
      if(!icon||!name||!back)return null;
      const iconBox=icon.getBoundingClientRect(),backBox=back.getBoundingClientRect();
      return {
        gap:name.getBoundingClientRect().left-iconBox.right,
        token:parseFloat(getComputedStyle(button).getPropertyValue('--gap-in')),
        offset:iconBox.left+iconBox.width/2-backBox.left-backBox.width/2,
      };
    });
    assert(spacing && Math.abs(spacing.gap-spacing.token)<=1,'The branch icon and name must share a trigger and one control gap');
    assert(Math.abs(spacing.offset)<=1,'The branch icon must align with the Back button');
    await activate(nav.locator('[data-workspace-view="history"]'));
    await first.waitFor();
    const original=await first.getAttribute('data-workspace-log-commit');
    assert(original.startsWith('a'));

    await activate(picker.locator('[data-workspace-branch-icon] svg'));
    await option('refs/heads/feat/workspace-preview').waitFor();
    assert.equal(await option('').getAttribute('aria-checked'),'true');
    assert.match(await option('').innerText(),/当前工作区|Current workspace/);
    await page.screenshot({path:path.join(report,name+'-menu.png')});
    await activate(option('refs/heads/feat/workspace-preview'));
    await first.filter({hasText:'工作区分支预览 43'}).waitFor();
    assert.equal(await label.textContent(),'feat/workspace-preview');
    assert((await first.getAttribute('data-workspace-log-commit')).startsWith('b'));
    await page.screenshot({path:path.join(report,name+'-history.png')});
    await activate(page.locator('[data-workspace-log-older]'));
    await first.filter({hasText:'工作区分支预览 3'}).waitFor();

    await activate(picker);
    assert.equal(await option('refs/heads/feat/workspace-preview').getAttribute('aria-checked'),'true');
    await activate(option('refs/remotes/origin/release'));
    await first.filter({hasText:'远程发布准备 43'}).waitFor();
    assert.equal(await label.textContent(),'origin/release');
    assert((await first.getAttribute('data-workspace-log-commit')).startsWith('c'));

    await activate(picker);
    await activate(option(''));
    await page.locator(`[data-workspace-log-commit="${original}"]`).waitFor();
    assert.equal(await label.textContent(),'main');
    await activate(nav.locator('[data-workspace-view="files"]'));
    await page.locator('[data-file-tree] [data-tree-entry]:visible').first().waitFor();
    assert.deepEqual(await page.locator('[data-file-tree] button[data-file-path]').evaluateAll(rows=>rows.map(row=>row.getAttribute('data-file-path'))),filesBefore);

    await activate(picker);
    await activate(option('refs/heads/feat/workspace-preview'));
    await page.locator('[data-git-snapshot-tree]').waitFor();
    assert.equal(await nav.getAttribute('data-view'),'files','Picking a version keeps the file page');
    await activate(nav.locator('[data-workspace-view="history"]'));
    await activate(first.filter({hasText:'工作区分支预览 43'}));
    await page.locator('[data-commit-page]:visible').getByText('工作区分支预览 43',{exact:true}).first().waitFor();
    assert.deepEqual(errors,[]);
    return {name,errors};
  } finally {
    await context.close();
  }
}
