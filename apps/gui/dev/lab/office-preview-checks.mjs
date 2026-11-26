import assert from 'node:assert/strict';
import path from 'node:path';
import {writeFile} from 'node:fs/promises';

const files = {docx:'项目周报.docx', pptx:'阶段汇报.pptx', xlsx:'验收台账.xlsx'};

export async function checkOfficePreviews(browser, {port, report, width, engine}) {
  const name = `office-preview-${engine}-${width}`;
  const mobile = width < 500;
  const context = await browser.newContext({viewport:{width,height:mobile?844:900}, isMobile:mobile, hasTouch:mobile});
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
  const open = format => page.goto(origin+'/app/?scene=workspace&machines=1&theme=dark'+
    '&file='+encodeURIComponent('office/'+files[format])+
    (mobile ? '' : '&environment='+encodeURIComponent(JSON.stringify(environment))));
  const ready = async format => {
    const viewer = page.locator(`[data-office-preview="${format}"][data-ready]`);
    await viewer.waitFor({timeout:20_000});
    await viewer.locator('canvas').first().waitFor();
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
  const visibleCanvas = viewer => viewer.locator('canvas').first();
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
    assert.match((await word.textContent()).replace(/\s+/g,''),/项目进度周报/);
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
      const samples = await sampleLine(x,Math.ceil((await word.boundingBox()).y)+1);
      assert(samples.every(sample=>sample>215), `Scrolled Word paper keeps its colour up to the clip edge (${samples.join(', ')})`);
      await scrollWord(word,0);
      const returned = await sampleLine(x,paperTop);
      assert(returned.every(sample=>sample>215), `Returning to the top keeps the paper clear (${returned.join(', ')})`);
    }
    assert(await scrollWord(word,null)>0,'A multi-page Word document scrolls inside the pane');
    await page.screenshot({path:path.join(report,name+'-word.png')});

    if (mobile) await open('pptx');
    else await page.locator('[data-file-path$="/'+files.pptx+'"]').click();
    const slides = await ready('pptx');
    assert.equal(await slides.evaluate(viewer=>getComputedStyle(viewer).maskImage),'none',
      'A presentation that does not scroll has no top fade');
    assert.match(await slides.textContent(),/1 \/ 3/);
    await slides.getByRole('button',{name:/下一页|Next page/i}).click();
    await slides.getByText('2 / 3',{exact:true}).waitFor();
    assert.match((await slides.textContent()).replace(/\s+/g,''),/阶段进度/);
    const slideRect = await visibleCanvas(slides).boundingBox();
    const pane = await slides.boundingBox();
    assert(slideRect.width <= pane.width + 1 && slideRect.height < pane.height,'The slide fits the pane');
    await page.screenshot({path:path.join(report,name+'-powerpoint.png')});

    if (mobile) await open('xlsx');
    else await page.locator('[data-file-path$="/'+files.xlsx+'"]').click();
    const excel = await ready('xlsx');
    const firstSheet = await image(excel);
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
    await excel.getByText('验收汇总',{exact:true}).click();
    await page.waitForFunction(before => {
      const canvas = document.querySelector('[data-office-preview="xlsx"] canvas');
      return canvas && canvas.toDataURL() !== before;
    },firstSheet[2]);
    await page.screenshot({path:path.join(report,name+'-excel.png')});
    assert.equal(officeAssets.length,3,'Each renderer requests its original once');
    assert.deepEqual(foreign,[], 'Preview, including fonts, stays on the Lab origin');

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
