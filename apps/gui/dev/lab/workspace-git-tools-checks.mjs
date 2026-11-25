import assert from 'node:assert/strict';
import path from 'node:path';

export async function checkWorkspaceGitTools(browser,{port,report,engine,width,theme='light'}){
  const mobile=width<500;
  const context=await browser.newContext({viewport:{width,height:mobile?844:900},isMobile:mobile,hasTouch:mobile,colorScheme:theme});
  const name=`workspace-git-tools-${engine}-${width}-${theme}`;
  const page=await context.newPage(),errors=[];
  page.on('pageerror',error=>errors.push(error.message));
  const activate=locator=>mobile?locator.tap():locator.click();
  const capture=part=>page.screenshot({path:path.join(report,`${name}-${part}.png`),animations:'disabled'});
  const picker=page.locator('[data-workspace-branch-picker]');
  const menu=page.locator('[data-popup-menu][data-state="open"]');
  const option=value=>menu.locator(`[data-popup-option="${value}"]`);
  const nav=page.locator('[data-workspace-navigation][data-seat="workspace"]');
  const choose=async value=>{await activate(picker);await activate(option(value));};
  const openTool=async label=>{await activate(picker);await activate(menu.getByRole('menuitem',{name:label,exact:true}));};
  const close=async dialog=>{await activate(dialog.locator('[data-task-action="leading"] button'));await dialog.waitFor({state:'hidden'});};
  const rowLayout=async (row,name)=>row.evaluate((element,selector)=>{
    const bounds=element.getBoundingClientRect();
    const label=element.querySelector(selector);
    return {
      height:bounds.height,
      labelInset:Math.round(label.getBoundingClientRect().left-bounds.left),
      iconCount:element.querySelectorAll('button svg').length,
      trailingSlot:!!element.querySelector('[data-tree-mark-slot]'),
      highlight:getComputedStyle(element,'::before').backgroundColor,
    };
  },name);
  try{
    await page.goto(`http://127.0.0.1:${port}/app/?scene=workspaces&machines=1&theme=${theme}`);
    await activate(page.locator('[data-ws-row]:visible').first());
    const live=page.locator('[data-file-tree]');
    const docs=live.locator('[data-tree-directory$="/docs"]');
    const readme=live.locator('[data-tree-entry][data-file="README.md"]');
    await docs.waitFor();
    await readme.waitFor();
    if(!mobile) await activate(readme.locator('button[data-file-path]'));
    const directoryLayout=await rowLayout(docs,'[data-dir-name]');
    const fileLayout=await rowLayout(readme,'[data-file-name]');
    await capture('workspace-tree');

    const topic='refs/heads/feat/workspace-preview';
    await choose(topic);
    const tree=page.locator('[data-git-snapshot-tree]');
    await tree.waitFor();
    assert.equal(await nav.getAttribute('data-view'),'files');
    if(!mobile) await page.locator('[data-git-snapshot-file="README.md"]').waitFor();
    const snapshotDocs=tree.locator('[data-git-snapshot-path="docs"]');
    await snapshotDocs.waitFor();
    await tree.locator('[data-git-snapshot-path="README.md"]').waitFor();
    assert.equal(await tree.locator('[data-tree-entry][data-tree-directory="docs"]').count(),1,'The branch uses the workspace directory row');
    assert.deepEqual(await rowLayout(tree.locator('[data-tree-directory="docs"]'),'[data-dir-name]'),directoryLayout);
    assert.deepEqual(await rowLayout(tree.locator('[data-tree-entry][data-file="README.md"]'),'[data-file-name]'),fileLayout);
    await activate(snapshotDocs);
    const nested=tree.locator('[data-tree-entry][data-file="branch-only.md"]');
    await nested.waitFor();
    assert.equal(await nested.getAttribute('data-depth'),'1');
    assert.equal(await nested.locator('[data-guide="0"]').count(),1);
    await capture('branch-tree');
    await activate(tree.locator('[data-git-snapshot-path="docs/branch-only.md"]'));
    const file=page.locator('[data-git-snapshot-file="docs/branch-only.md"]');
    const text=file.locator('.cm-content').filter({hasText:'分支新增的文件'});
    await text.waitFor();
    assert.notEqual(await text.getAttribute('contenteditable'),'true');
    assert.equal(await file.getAttribute('data-git-snapshot-commit'),'b'+'a'.repeat(38)+'1');
    if(mobile) await page.locator('[data-session-bar]').getByText('branch-only.md',{exact:true}).waitFor();
    await capture('snapshot');
    if(mobile) {
      await activate(page.locator('[data-ws-mid]').getByRole('button',{name:'返回',exact:true}).first());
      await tree.waitFor();
    }

    await activate(nav.locator('[data-workspace-view="history"]'));
    const first=page.locator('[data-workspace-log-commit]').first();
    await first.filter({hasText:'工作区分支预览 43'}).waitFor();
    assert.equal(await first.getAttribute('data-workspace-log-commit'),'b'+'a'.repeat(38)+'1');
    if(!mobile) await file.waitFor();
    await choose('refs/tags/v0.1.0');
    await first.filter({hasText:'修复移动导航的松手确认'}).waitFor();
    assert.equal(await nav.getAttribute('data-view'),'history');
    await choose(topic);
    await first.filter({hasText:'工作区分支预览 43'}).waitFor();

    await openTool('比较版本');
    const comparison=page.locator('[data-git-tool="compare"]');
    await activate(comparison.locator('[data-git-run]'));
    await comparison.locator('[data-git-compare-result]').waitFor();
    const titleLines=await comparison.getByRole('heading',{name:'比较版本',exact:true}).evaluate(title=>{
      const text=document.createRange();
      text.selectNodeContents(title);
      return new Set([...text.getClientRects()].map(rect=>Math.round(rect.y))).size;
    });
    assert.equal(titleLines,1,'The compare sheet title fits on one line');
    await activate(comparison.locator('[data-git-compare-file="README.md"]'));
    await comparison.getByText('共同祖先',{exact:false}).first().waitFor();
    await comparison.locator('.cm-mergeView, .cm-editor').first().waitFor();
    await capture('compare');
    await close(comparison);

    await openTool('合并预演');
    const merge=page.locator('[data-git-tool="merge"]');
    await activate(merge.locator('[data-git-run]'));
    await merge.locator('[data-git-merge-result]').getByText('README.md',{exact:true}).waitFor();
    await capture('merge');
    await close(merge);

    await openTool('查询远程分支');
    const remotes=page.locator('[data-git-tool="remotes"]');
    await activate(remotes.locator('[data-git-remote-query]'));
    const available=remotes.locator('[data-git-live-ref="refs/heads/release"]');
    await available.waitFor();
    assert(await remotes.locator('[data-git-live-ref="refs/heads/upcoming"]').isDisabled());
    await capture('remote');
    await activate(available);
    await first.filter({hasText:'远程发布准备 43'}).waitFor();
    assert.equal(await nav.getAttribute('data-view'),'history');

    await choose('');
    await first.filter({hasText:'保留工作区阅读位置'}).waitFor();
    await activate(nav.locator('[data-workspace-view="files"]'));
    await page.locator('[data-file-tree]').waitFor();
    assert.equal(await tree.count(),0);
    await openTool('关联工作树');
    const worktrees=page.locator('[data-git-tool="worktrees"]');
    await worktrees.locator('[data-git-worktree="/preview/project"]').waitFor();
    assert.match(await worktrees.locator('[data-git-worktree="/preview/project"]').innerText(),/当前工作区/);
    await capture('worktrees');
    await activate(worktrees.locator('[data-git-worktree="/preview/research-notes"]').getByRole('button',{name:'打开工作区'}));
    await page.locator('[data-file-tree][data-tree-root="/preview/research-notes"]').waitFor();
    assert.deepEqual(errors,[]);
    return {name,errors};
  }finally{
    if(errors.length)console.log(errors);
    await context.close();
  }
}
