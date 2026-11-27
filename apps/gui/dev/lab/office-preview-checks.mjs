import assert from 'node:assert/strict';
import path from 'node:path';
import {writeFile} from 'node:fs/promises';

const files = {docx:'项目周报.docx', pptx:'阶段汇报.pptx', xlsx:'验收台账.xlsx'};

export async function checkOfficePreviews(browser, {port, report, width, engine}) {
  const name = `office-preview-${engine}-${width}`;
  const mobile = width < 500;
  const context = await browser.newContext({viewport:{width,height:mobile?844:900}, isMobile:mobile, hasTouch:mobile});
  await context.addInitScript(() => {
    window.officeCopied = "";
    Object.defineProperty(navigator,'clipboard',{configurable:true,value:{
      writeText: async text => { window.officeCopied = text; },
    }});
  });
  const page = await context.newPage();
  const errors = [];
  const origin = `http://127.0.0.1:${port}`;
  const foreign = [];
  const officeAssets = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => {
    const url = new URL(request.url());
    if (url.pathname.includes('/assets/preview-fixtures/office/')) officeAssets.push(url.pathname);
    if (/^https?:$/.test(url.protocol) && url.origin !== origin) foreign.push(url.href);
  });
  const environment = {platform:'desktop',theme:'dark',material:'frost',opacity:60,
    background:'dark',keyboard:0,safeTop:0,safeBottom:0};
  const open = (format, file = files[format]) => page.goto(origin+'/app/?scene=workspace&machines=1&theme=dark'+
    '&file='+encodeURIComponent('office/'+file)+
    (mobile ? '' : '&environment='+encodeURIComponent(JSON.stringify(environment))));
  const ready = async format => {
    const viewer = page.locator(`[data-office-preview="${format}"][data-ready]`);
    await viewer.waitFor({timeout:20_000});
    await visibleCanvas(viewer).waitFor();
    const icons = await page.locator('[data-office-toolbar]:visible button:visible svg').evaluateAll(nodes =>
      nodes.map(icon => {
        const style = getComputedStyle(icon);
        const token = icon.closest('[data-glass-item]') ? '--ctl-glyph' : '--affordance';
        return [style.width, style.height, style.getPropertyValue(token).trim()];
      }));
    assert(icons.length > 0 && icons.every(([width,height,token]) => width === token && height === token),
      `Office tools use the same glyph size as the file reader (${JSON.stringify(icons)})`);
    assert.equal(await page.locator('[data-file-face-toggle]:visible').count(),0,`${format} opens without a preview switch`);
    if (!mobile) {
      const chrome = await page.locator('[data-ws-mid] [data-tab-bar]').boundingBox();
      const document = await viewer.boundingBox();
      assert(document.y >= chrome.y + chrome.height - 0.5,
        `Office content must not paint behind the shared tab capsule (${format}: ${document.y}, chrome bottom: ${chrome.y + chrome.height})`);
    }
    return viewer;
  };
  const text = page.locator('[data-look="doc"]:visible');
  const tool = async key => page.locator(`[data-${key}]`).click();
  const togglePane = async (key, pane, open) => {
    const trigger = page.locator(`[data-${key}]`);
    await tool(key);
    await pane.waitFor({state:open?'visible':'detached'});
    assert.equal(await trigger.getAttribute('aria-pressed'),String(open));
    assert.equal(await page.getByRole('dialog').count(),0,'Reader tools do not open dialogs');
  };
  const rightComments = async viewer => {
    const pane = viewer.locator('[data-office-annotations="comments"]');
    await togglePane('office-review',pane,true);
    const document = await viewer.locator('[data-office-document]').boundingBox();
    assert((await pane.boundingBox()).x >= document.x + document.width - 0.5,
      'Comments dock to the right of the document');
    return pane;
  };
  const search = async value => {
    await page.locator('[data-office-find-open]:visible').click();
    const field = page.getByRole('searchbox',{name:/搜索此文件|Search this file/i});
    await field.fill(value);
    await page.waitForFunction(() => {
      const count = document.querySelector('[data-office-find-count]')?.textContent;
      return count && (/\d+ \/ \d+/.test(count) || /没有匹配项|No matches/i.test(count));
    });
    return field;
  };
  const searchCount = () => page.locator('[data-office-find-count]').innerText();
  const visibleCanvas = viewer => viewer.locator('[data-office-document] canvas').first();
  const image = viewer => visibleCanvas(viewer).evaluate(canvas => {
    const {width,height} = canvas;
    const context = canvas.getContext('2d');
    return [width,height,context ? canvas.toDataURL() : ''];
  });
  const scrollWord = (viewer, top) => viewer.evaluate((node, offset) => {
    const scroller = [...node.querySelectorAll('div')].find(child =>
      child.scrollHeight > child.clientHeight && ['auto','scroll'].includes(getComputedStyle(child).overflowY));
    if (!scroller) return null;
    scroller.scrollTop = offset ?? scroller.scrollHeight - scroller.clientHeight;
    return scroller.scrollTop;
  },top);
  const sampleLine = async (x,y) => {
    const screenshot = await page.screenshot({clip:{x,y,width:1,height:28},animations:'disabled'});
    return page.evaluate(async png => {
      const image = new Image();
      image.src = 'data:image/png;base64,'+png;
      await image.decode();
      const canvas = document.createElement('canvas');
      canvas.width = image.width;
      canvas.height = image.height;
      const ctx = canvas.getContext('2d');
      ctx.drawImage(image,0,0);
      return [0,16,27].map(offset => ctx.getImageData(0,offset,1,1).data[0]);
    },screenshot.toString('base64'));
  };
  try {
    await open('docx');
    const word = await ready('docx');
    // A visible canvas can precede the selectable Word text in WKWebView.
    await word.filter({hasText:/项\s*目\s*进\s*度\s*周\s*报/}).waitFor({timeout:20_000});
    const [wordWidth,wordHeight] = await image(word);
    assert(wordWidth > 150 && wordHeight > 200,'Word paints document-sized pages');
    if (!mobile) {
      await page.locator('html[data-theme="dark"][data-lab-environment]').waitFor({state:'attached'});
      const paper = await visibleCanvas(word).boundingBox();
      const x = Math.floor(paper.x+paper.width*0.7);
      const paperTop = Math.ceil(paper.y)+3;
      const atTop = await sampleLine(x,paperTop);
      assert(atTop.every(sample=>sample>215), `The first page stays clear at the top (${atTop.join(', ')})`);
      assert(await scrollWord(word,72)>0,'The Word viewport scrolls inside its own pane');
      await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
      const samples = await sampleLine(x,Math.ceil((await word.locator('[data-office-document]').boundingBox()).y)+1);
      assert(samples.every(sample=>sample>215), `Scrolled Word paper keeps its colour up to the clip edge (${samples.join(', ')})`);
      await scrollWord(word,0);
      const returned = await sampleLine(x,paperTop);
      assert(returned.every(sample=>sample>215), `Returning to the top keeps the paper clear (${returned.join(', ')})`);
    }
    assert(await scrollWord(word,null)>0,'A multi-page Word document scrolls inside the pane');
    await page.screenshot({path:path.join(report,name+'-word.png')});
    const bodyTop = (await word.locator('[data-office-document]').boundingBox()).y;
    const searchWord = await search('验收');
    assert.equal((await word.locator('[data-office-document]').boundingBox()).y,bodyTop,'Searching uses the same toolbar row');
    assert.match(await searchCount(), /1 \/ [2-9]|1 \/ \d{2,}/,'Word starts at the first match');
    await page.locator('[data-office-find]').getByRole('button',{name:/下一个|Next match/i}).click();
    await page.waitForFunction(() => document.querySelector('[data-office-find-count]')?.textContent?.startsWith('2 /'));
    await searchWord.press('Escape');
    await page.locator('[data-office-find]').waitFor({state:'detached'});

    const wordReview = word.locator('[data-word-review]');
    await togglePane('office-review',wordReview,true);
    const first = wordReview.locator('article').filter({hasText:'核对第一阶段验收范围。'});
    const second = wordReview.locator('article').filter({hasText:'确认这一项的验收记录。'});
    await first.waitFor();
    assert.deepEqual(await first.locator('header span').allTextContents(),['Khor Lab','批注']);
    assert.equal(await first.locator('time:visible').getAttribute('datetime'),'2026-11-26T16:39:54Z');
    const wordBody = word.locator('[data-office-document]');
    const wordBox = await wordBody.boundingBox();
    assert((await wordReview.boundingBox()).x >= wordBox.x + wordBox.width - 0.5);
    const [commentInk,secondCommentInk] = await Promise.all([first,second].map(card =>
      card.evaluate(card => getComputedStyle(card).borderInlineStartColor)));
    const expectAnchor = (text,ink,selected = false) => page.waitForFunction(({text,ink,selected}) => {
      const documentBody = document.querySelector('[data-office-preview="docx"] [data-office-document]');
      const ranges = [...document.querySelectorAll('[data-word-comment-range]')];
      const highlight = ranges.find(node => getComputedStyle(node).stroke === ink);
      const connector = [...document.querySelectorAll('[data-word-review-connector]')].find(path =>
        getComputedStyle(path).stroke === ink && path.hasAttribute('stroke-dasharray') === !selected);
      const run = [...documentBody?.querySelectorAll('[data-ooxml-selection-run="docx"]') ?? []]
        .find(node => node.textContent === text);
      if (ranges.length !== 2 || !highlight || !connector || !run) return false;
      const mark = highlight.getBoundingClientRect();
      const line = run.getBoundingClientRect();
      const stroke = getComputedStyle(highlight);
      const origin = connector.getPointAtLength(0);
      const viewport = connector.ownerSVGElement.getBoundingClientRect();
      return Math.min(mark.right, line.right) - Math.max(mark.left, line.left) >= line.width * 0.75 &&
        mark.top < line.bottom && line.top < mark.bottom &&
        Number(stroke.fillOpacity) >= 0.30 && (Number(stroke.fillOpacity) > 0.40) === selected &&
        Math.abs(viewport.left + origin.x - mark.right) < 1 &&
        viewport.top + origin.y >= mark.top - 1 && viewport.top + origin.y <= mark.bottom + 1;
    },{text,ink,selected},{timeout:5_000});
    await expectAnchor('第一阶段验收',commentInk);
    await expectAnchor('需求确认',secondCommentInk);
    assert.equal(await wordReview.locator('[aria-pressed="true"]').count(),0,
      'All comment anchors are filled as soon as the pane opens');
    assert.equal(await word.locator('[data-word-review-trunk]').count(),0,
      'Comments on different text lines keep separate connections');
    await page.screenshot({path:path.join(report,name+'-word-comments.png'), animations:'disabled'});
    await second.getByText('确认这一项的验收记录。',{exact:true}).click();
    await expectAnchor('需求确认',secondCommentInk,true);
    await expectAnchor('第一阶段验收',commentInk);
    assert.equal(await first.locator('[data-word-review-select]').getAttribute('aria-pressed'),'false');
    await first.locator('[data-word-review-select]').press('Enter');
    await expectAnchor('第一阶段验收',commentInk,true);
    await wordBody.click({position:{x:4,y:4}});
    await expectAnchor('第一阶段验收',commentInk,true);
    await expectAnchor('需求确认',secondCommentInk);
    await togglePane('office-review',wordReview,false);
    assert.equal(await word.locator('[data-word-comment-range]').count(),0,
      'Closing comments hides the document markup');
    await togglePane('office-revisions',wordReview,true);
    await wordBody.getByText('联调尚未开始。',{exact:true}).waitFor({state:'attached'});
    const change = await search('联调尚未开始');
    assert.match(await searchCount(),/1 \/ 1/,'The deletion is findable with revision markup');
    await change.press('Escape');
    const deletion = wordReview.locator('[data-revision="deletion"]');
    const insertion = wordReview.locator('[data-revision="insertion"]');
    await deletion.getByText('联调尚未开始。',{exact:true}).waitFor();
    await insertion.getByText('本周已完成联调。',{exact:true}).waitFor();
    const [insertionInk,deletionInk] = await Promise.all([insertion,deletion].map(card =>
      card.evaluate(node => getComputedStyle(node).borderInlineStartColor)));
    const needPaint = async (text, ink) => {
      await page.waitForFunction(({text,ink}) => {
        const run = [...document.querySelectorAll('[data-ooxml-selection-run="docx"]')]
          .find(node => node.textContent === text);
        const source = run?.closest('[data-ooxml-selection-surface]')?.parentElement.querySelector('canvas');
        if (!run || !source) return false;
        const rect = run.getBoundingClientRect();
        const page = source.getBoundingClientRect();
        const xScale = source.width / page.width;
        const yScale = source.height / page.height;
        const left = Math.max(0, Math.floor((rect.left - page.left) * xScale));
        const top = Math.max(0, Math.floor((rect.top - page.top - 1) * yScale));
        const width = Math.min(source.width - left, Math.ceil(rect.width * xScale));
        const height = Math.min(source.height - top, Math.ceil((rect.height + 5) * yScale));
        if (width <= 0 || height <= 0) return false;
        const sample = document.createElement('canvas');
        sample.width = width;
        sample.height = height;
        const ctx = sample.getContext('2d', {willReadFrequently:true});
        ctx.fillStyle = ink;
        ctx.fillRect(0,0,1,1);
        const expected = ctx.getImageData(0,0,1,1).data;
        const expectedRG = expected[0] - expected[1];
        const expectedGB = expected[1] - expected[2];
        const expectedChroma = Math.hypot(expectedRG,expectedGB);
        ctx.drawImage(source,left,top,width,height,0,0,width,height);
        const paint = ctx.getImageData(0,0,width,height).data;
        for (let i=0; i<paint.length; i+=4) {
          // A rasterised underline may be blended with white. Compare its
          // channel differences, which keep their direction after blending.
          const rg = paint[i] - paint[i+1];
          const gb = paint[i+1] - paint[i+2];
          const chroma = Math.hypot(rg,gb);
          if (chroma > 20 && (rg*expectedRG + gb*expectedGB) / (chroma*expectedChroma) > 0.995) return true;
        }
        return false;
      },{text,ink},{timeout:5000});
    };
    if (!mobile) {
      await needPaint('本周已完成联调。',insertionInk);
      await needPaint('联调尚未开始。',deletionInk);
    }
    assert.equal(insertionInk,commentInk,'The same author keeps their colour across comments and revisions');
    assert.notEqual(deletionInk,insertionInk,'Different authors have distinct review colours');
    for (const [card,author,action] of [[insertion,'Khor Lab','插入了'],[deletion,'林岚','删除了']]) {
      assert.deepEqual(await card.locator('header span').allTextContents(),[author,action],
        'The review header shows the author first, then the action');
      assert.equal(await card.locator('time:visible').getAttribute('datetime'),'2026-11-26T15:00:00Z');
    }
    const removed = await deletion.boundingBox();
    const added = await insertion.boundingBox();
    assert(removed.y + removed.height <= added.y || added.y + added.height <= removed.y,
      'Revision balloons at the same anchor stack without overlap');
    assert.equal(await word.locator('[data-word-review-connector]').count(),2,
      'Each change has its own connector');
    const inks = await word.locator('[data-word-review-connector]').evaluateAll(paths =>
      paths.map(path => getComputedStyle(path).stroke));
    assert.deepEqual(inks.sort(),[insertionInk,deletionInk].sort(),
      'Each connector is drawn once in the matching card colour');
    const trunk = word.locator('[data-word-review-trunk]');
    await trunk.waitFor({state:'attached'});
    assert.equal(await trunk.count(),1,'The revisions on one text line share one connector');
    assert(![insertionInk,deletionInk].includes(await trunk.evaluate(path => getComputedStyle(path).stroke)),
      'The shared line uses neutral ink for multiple authors');
    const branchesMeetAtEdge = await word.evaluate(viewer => {
      const trunk = viewer.querySelector('[data-word-review-trunk]');
      const branches = [...viewer.querySelectorAll('[data-word-review-branch]')];
      const width = viewer.querySelector('[data-office-document]').clientWidth;
      const start = trunk.getPointAtLength(0);
      const end = trunk.getPointAtLength(trunk.getTotalLength());
      return branches.length === 2 && start.y === end.y && start.x < end.x &&
        end.x > width * 0.8 && end.x <= width && branches.every(branch => {
          const joint = branch.getPointAtLength(0);
          return Math.hypot(joint.x - end.x,joint.y - end.y) < 0.1;
        });
    });
    assert(branchesMeetAtEdge,'All branches start at the end of the horizontal line near the right edge');
    const insertButton = insertion.locator('[data-word-review-select]');
    await insertButton.press('Enter');
    await insertion.locator('[aria-pressed="true"]').waitFor();
    const originalInsertion = await wordBody.getByText('本周已完成联调。',{exact:true}).boundingBox();
    const documentViewport = await wordBody.boundingBox();
    assert(originalInsertion.y >= documentViewport.y - 1 &&
      originalInsertion.y < documentViewport.y + documentViewport.height,
    'Selecting a revision reveals the changed text inside the document viewport');
    await deletion.locator('[data-word-review-select]').click();
    await deletion.locator('[aria-pressed="true"]').waitFor();
    assert.equal(await insertButton.getAttribute('aria-pressed'),'false');
    await page.screenshot({path:path.join(report,name+'-word-revisions.png'),animations:'disabled'});
    await togglePane('office-revisions',wordReview,false);

    const wordPages = word.locator('[data-office-thumbnails]');
    await togglePane('office-pages',wordPages,true);
    await wordPages.locator('[data-office-thumbnail="2"] canvas').waitFor();
    await page.waitForFunction(() => {
      const thumbnail = document.querySelector('[data-office-thumbnail="2"] canvas');
      return thumbnail instanceof HTMLCanvasElement && thumbnail.width > 80;
    });
    const pageNumber = wordPages.getByRole('spinbutton',{name:'页码'});
    const previousPage = wordPages.getByRole('button',{name:'上一页'});
    const followingPage = wordPages.getByRole('button',{name:'下一页'});
    await wordPages.locator('[data-office-thumbnail="2"]').click();
    await wordPages.locator('[data-office-thumbnail="2"][aria-current="page"]').waitFor();
    assert(await previousPage.isEnabled() && await followingPage.isDisabled());
    await previousPage.click();
    await wordPages.locator('[data-office-thumbnail="1"][aria-current="page"]').waitFor();
    assert(await previousPage.isDisabled() && await followingPage.isEnabled());
    await followingPage.click();
    await wordPages.locator('[data-office-thumbnail="2"][aria-current="page"]').waitFor();
    await pageNumber.fill('1');
    await pageNumber.press('Enter');
    await wordPages.locator('[data-office-thumbnail="1"][aria-current="page"]').waitFor();
    await togglePane('office-pages',wordPages,false);

    if (mobile) await open('pptx');
    else await page.locator('[data-file-path$="/'+files.pptx+'"]').click();
    const slides = await ready('pptx');
    assert.equal(await slides.evaluate(viewer=>getComputedStyle(viewer).maskImage),'none',
      'A presentation that does not scroll has no top fade');
    assert.match(await slides.textContent(),/1 \/ 3/);
    const notes = slides.locator('[data-office-annotations="notes"]');
    await togglePane('office-notes',notes,true);
    await notes.getByText(/先介绍项目目标/).waitFor();
    const slidesBody = slides.locator('[data-office-document]');
    const slidesBodyBox = await slidesBody.boundingBox();
    assert((await notes.boundingBox()).y >= slidesBodyBox.y + slidesBodyBox.height - 0.5,
      'Presenter notes dock below the slide');
    const slideComments = await rightComments(slides);
    await slideComments.getByText('在第一页强调验收目标。',{exact:true}).waitFor();
    await slideComments.getByRole('button',{name:'定位批注'}).click();
    await slides.locator('[data-office-comment-anchor] span').waitFor();
    await page.screenshot({path:path.join(report,name+'-powerpoint-comments.png'),animations:'disabled'});
    await slides.getByRole('button',{name:/下一页|Next page/i}).click();
    await slides.getByText('2 / 3',{exact:true}).waitFor();
    await slideComments.getByText('此处没有批注',{exact:true}).waitFor();
    await slides.locator('[data-office-comment-anchor]').waitFor({state:'detached'});
    await togglePane('office-review',slideComments,false);
    await togglePane('office-notes',notes,false);
    assert.match((await slides.textContent()).replace(/\s+/g,''),/阶段进度/);
    await page.waitForFunction(() => {
      const body = document.querySelector('[data-office-preview="pptx"] [data-office-document]');
      const canvas = body?.querySelector('canvas')?.getBoundingClientRect();
      const pane = body?.getBoundingClientRect();
      return canvas && pane && Math.abs(canvas.x + canvas.width/2 - pane.x - pane.width/2) < 1.5 &&
        Math.abs(canvas.y + canvas.height/2 - pane.y - pane.height/2) < 1.5;
    });
    const slideRect = await visibleCanvas(slides).boundingBox();
    const pane = await slides.locator('[data-office-document]').boundingBox();
    assert(slideRect.width <= pane.width + 1 && slideRect.height <= pane.height + 1,'The slide fits the pane');
    const left = slideRect.x - pane.x;
    const right = pane.x + pane.width - slideRect.x - slideRect.width;
    const top = slideRect.y - pane.y;
    const bottom = pane.y + pane.height - slideRect.y - slideRect.height;
    assert(Math.abs(left - right) < 1.5 && Math.abs(top - bottom) < 1.5,'The slide is centered in the document area');
    await page.screenshot({path:path.join(report,name+'-powerpoint.png')});
    const slidePages = slides.locator('[data-office-thumbnails]');
    await togglePane('office-pages',slidePages,true);
    await slidePages.locator('[data-office-thumbnail="3"]').click();
    await slides.getByText('3 / 3',{exact:true}).waitFor();
    await slidePages.locator('[data-office-thumbnail="3"][aria-current="page"]').waitFor();
    await togglePane('office-pages',slidePages,false);
    if (mobile) {
      assert.equal(await slides.locator('[data-preview-zoom], [data-office-zoom]').count(),0,
        'A touch preview uses pinch without mounting zoom buttons');
      await visibleCanvas(slides).evaluate(canvas => {
        const box = canvas.getBoundingClientRect();
        const start = box.left + box.width * .4, y = box.top + box.height * .5;
        const send = (type, touches) => {
          const event = new Event(type,{bubbles:true,cancelable:true});
          Object.defineProperty(event,'touches',{value:touches.map(clientX => ({target: canvas,clientX,clientY:y}))});
          canvas.dispatchEvent(event);
        };
        send('touchstart',[start - 30,start + 30]);
        send('touchmove',[start - 55,start + 55]);
        send('touchend',[]);
      });
    } else {
      const plus = slides.locator('[data-preview-zoom-in]');
      const box = await plus.boundingBox();
      await page.mouse.move(box.x + box.width / 2,box.y + box.height / 2);
      await plus.click();
    }
    await page.waitForFunction(initial => {
      const canvas = document.querySelector('[data-office-preview="pptx"] canvas');
      return canvas && canvas.getBoundingClientRect().width > initial;
    },slideRect.width);

    if (mobile) await open('xlsx');
    else await page.locator('[data-file-path$="/'+files.xlsx+'"]').click();
    const excel = await ready('xlsx');
    const previousSheet = excel.getByRole('button',{name:'上一张工作表',exact:true});
    const nextSheet = excel.getByRole('button',{name:'下一张工作表',exact:true});
    const checkSheetArrows = index => page.waitForFunction(index => {
      const previous = document.querySelector('[data-xlsx-tab-nav="prev"]');
      const next = document.querySelector('[data-xlsx-tab-nav="next"]');
      return previous?.disabled === (index === 0) && next?.disabled === (index === 1) &&
        getComputedStyle(index === 0 ? next : previous).pointerEvents === 'auto';
    },index,{timeout:5000});
    await checkSheetArrows(0);
    const firstSheet = await image(excel);
    const cellComments = await rightComments(excel);
    await cellComments.getByText('这里记录第一项验收内容。',{exact:true}).waitFor();
    await cellComments.getByRole('button',{name:'B2',exact:true}).click();
    await cellComments.locator('[data-office-comment-thread][data-active]').waitFor();
    await page.screenshot({path:path.join(report,name+'-excel-comments.png'),animations:'disabled'});
    await togglePane('office-review',cellComments,false);
    const searchCells = await search('预览验收任务 001');
    assert.match(await searchCount(),/\d+ \/ \d+/);
    await searchCells.press('Escape');
    const sheetGrid = await visibleCanvas(excel).boundingBox();
    await page.mouse.click(sheetGrid.x+100,sheetGrid.y+42);
    assert.equal(await page.locator('[data-office-copy]').count(),0,'The sheet uses the system copy action');
    await page.keyboard.press('Meta+c');
    await page.waitForFunction(() => window.officeCopied.trim().length > 0);
    const copiedCells = await page.evaluate(() => {
      const text = window.officeCopied;
      window.officeCopied = '';
      document.body.dispatchEvent(new ClipboardEvent('copy',{bubbles:true,cancelable:true}));
      return text;
    });
    await page.waitForFunction(expected => window.officeCopied === expected,copiedCells);
    if (!mobile) {
      const mask = () => excel.evaluate(viewer=>getComputedStyle(viewer).maskImage);
      const waitForScroll = scrolled => page.waitForFunction(expected => {
        const viewer = document.querySelector('[data-office-preview="xlsx"]');
        const port = [...(viewer?.querySelectorAll('div') ?? [])].find(child =>
          child.scrollHeight > child.clientHeight && ['auto','scroll'].includes(getComputedStyle(child).overflowY));
        return port && (port.scrollTop > 0) === expected;
      },scrolled);
      assert.equal(await mask(),'none','The worksheet opens with its headings clear');
      const grid = await visibleCanvas(excel).boundingBox();
      await page.mouse.move(grid.x+grid.width/2,grid.y+grid.height/2);
      await page.mouse.wheel(0,200);
      await waitForScroll(true);
      assert.equal(await mask(),'none','Scrolling leaves the worksheet headings clear');
      await page.mouse.wheel(0,-2000);
      await waitForScroll(false);
      assert.equal(await mask(),'none','Returning to the first row keeps the worksheet clear');
    }
    const waitForSheetPaint = before => page.waitForFunction(before => {
      const canvas = document.querySelector('[data-office-preview="xlsx"] canvas');
      return canvas && canvas.toDataURL() !== before;
    },before);
    await excel.getByText('验收汇总',{exact:true}).click();
    await checkSheetArrows(1);
    await waitForSheetPaint(firstSheet[2]);
    const summarySheet = await image(excel);
    await previousSheet.click();
    await checkSheetArrows(0);
    await waitForSheetPaint(summarySheet[2]);
    const restoredSheet = await image(excel);
    await nextSheet.click();
    await checkSheetArrows(1);
    await waitForSheetPaint(restoredSheet[2]);
    await page.screenshot({path:path.join(report,name+'-excel.png')});
    assert.equal(officeAssets.length,3,'Each renderer requests its original once');
    await open('docx','加密周报.docx');
    const unlock = page.locator('[data-office-unlock]');
    await unlock.waitFor();
    const password = unlock.locator('input[type="password"]');
    await password.fill('wrong-password');
    await unlock.getByRole('button',{name:'解锁文档'}).click();
    await unlock.getByText('文档密码不正确',{exact:true}).waitFor();
    await password.fill('khor-lab');
    await unlock.getByRole('button',{name:'解锁文档'}).click();
    await ready('docx');
    assert.deepEqual(foreign,[], 'Preview, including fonts, stays on the Lab origin');

    const sessionEnvironment = mobile ? {...environment,platform:'ios',safeTop:59,safeBottom:34} : environment;
    await page.goto(origin+'/app/?scene=content&machines=1'+
      '&environment='+encodeURIComponent(JSON.stringify(sessionEnvironment)));
    const backToFiles = mobile ? '[data-file-deck] [data-file-back]:visible' :
      '[data-content-toolbar="file"] [data-content-back]:visible';
    await page.locator(backToFiles).click();
    const picker = page.locator('[data-content-picker]:visible');
    await picker.locator('[data-file-path="/preview/project/office"]').click();
    await picker.locator(`[data-file-path="/preview/project/office/${files.docx}"]`).click();
    const sessionOffice = page.locator('[data-file-page] [data-office-toolbar]:visible');
    await sessionOffice.waitFor({timeout:20_000});
    const gap = () => sessionOffice.evaluate((toolbar,mobile) => {
      const file = toolbar.closest('[data-file-page]');
      const location = file.querySelector('[data-content-toolbar="file"]');
      if (mobile) return toolbar.getBoundingClientRect().top - file.getBoundingClientRect().top;
      const office = file.querySelector('[data-office-preview][data-ready]');
      return office.getBoundingClientRect().top - location.getBoundingClientRect().bottom;
    },mobile);
    if (!mobile) {
      // The desktop browser does not inherit AppKit's window buttons.
      await page.locator('html').evaluate(root => root.setAttribute('data-traffic',''));
    }
    const initialGap = await gap();
    assert(Math.abs(initialGap)<=1,`The Office toolbar follows the content edge (${initialGap}px gap)`);
    await page.screenshot({path:path.join(report,name+'-session.png')});
    if (mobile) {
      const sheet = page.locator('[data-mobile-content-sheet] [data-content-panel]');
      await sheet.locator('[data-place-grab]').click();
      await page.waitForFunction(() => {
        const panel = document.querySelector('[data-content-panel][data-detent="full"]');
        return panel && Math.abs(panel.getBoundingClientRect().top - panel.closest('[data-session-screen]').getBoundingClientRect().top) <= 1;
      });
      assert(Math.abs(await gap())<=1,'The full sheet reserves the status bar once');
      assert((await sessionOffice.boundingBox()).y>=sessionEnvironment.safeTop,'Office controls clear the status bar');
      await page.screenshot({path:path.join(report,name+'-session-full.png')});
    } else {
      assert.equal(await page.locator('[data-content-reading-tools] [data-office-toolbar]').count(),1,
        'Office actions share the session file location row');
    }

    await page.route(/_parser_bg[^/]*\.wasm(?:\?|$)/, route =>
      route.request().resourceType() === 'fetch' ? route.abort() : route.continue());
    await open('docx');
    await page.locator('[data-doc-note="doc"]').waitFor({timeout:20_000});
    assert.match(await text.textContent(),/项目进度周报/);
    assert.equal(await page.locator('[data-office-preview]').count(),0,'Initial rendering failures release the Office viewer');
  } catch (error) {
    errors.push(String(error));
    await writeFile(path.join(report,name+'-failure.txt'),(await page.locator('body').innerText()).slice(0,16_000));
    await page.screenshot({path:path.join(report,name+'-failure.png')});
  } finally {
    await context.close();
  }
  return {name, errors};
}
