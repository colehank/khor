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
  page.on('request', request => {
    const url = new URL(request.url());
    if (/^https?:$/.test(url.protocol) && url.origin !== origin) foreign.push(url.href);
  });
  const open = format => page.goto(origin+'/app/?scene=workspace&machines=1&theme='+(mobile?'dark':'light')+
    '&file='+encodeURIComponent('office/'+files[format]));
  const ready = async format => {
    const viewer = page.locator(`[data-office-preview="${format}"][data-ready]`);
    await viewer.waitFor({timeout:20_000});
    await viewer.locator('canvas').first().waitFor();
    if (!mobile) {
      const chrome = await page.locator('[data-ws-mid] [data-tab-bar]').boundingBox();
      const document = await viewer.boundingBox();
      assert(document.y >= chrome.y + chrome.height - 0.5,
        `Office content must not paint behind the shared tab capsule (${format}: ${document.y}, chrome bottom: ${chrome.y + chrome.height})`);
    }
    return viewer;
  };
  const visibleCanvas = viewer => viewer.locator('canvas').first();
  const image = viewer => visibleCanvas(viewer).evaluate(canvas => {
    const {width,height} = canvas;
    const context = canvas.getContext('2d');
    return [width,height,context ? canvas.toDataURL() : ''];
  });
  try {
    await open('docx');
    const word = await ready('docx');
    assert.match((await word.textContent()).replace(/\s+/g,''),/项目进度周报/);
    const [wordWidth,wordHeight] = await image(word);
    assert(wordWidth > 150 && wordHeight > 200,'Word paints document-sized pages');
    assert(await word.evaluate(viewer => {
      const scroller = [...viewer.querySelectorAll('div')].find(node =>
        node.scrollHeight > node.clientHeight && ['auto','scroll'].includes(getComputedStyle(node).overflowY));
      if (!scroller) return false;
      scroller.scrollTop = scroller.scrollHeight - scroller.clientHeight;
      return scroller.scrollTop > 0;
    }), 'A multi-page Word document scrolls inside the pane');
    await page.screenshot({path:path.join(report,name+'-word.png')});

    if (mobile) await open('pptx');
    else await page.locator('[data-file-path$="/'+files.pptx+'"]').click();
    const slides = await ready('pptx');
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
    await excel.getByText('验收汇总',{exact:true}).click();
    await page.waitForFunction(before => {
      const canvas = document.querySelector('[data-office-preview="xlsx"] canvas');
      return canvas && canvas.toDataURL() !== before;
    },firstSheet[2]);
    await page.screenshot({path:path.join(report,name+'-excel.png')});
    assert.deepEqual(foreign,[], 'Preview, including fonts, stays on the Lab origin');

    await page.route(/_parser_bg[^/]*\.wasm(?:\?|$)/, route =>
      route.request().resourceType() === 'fetch' ? route.abort() : route.continue());
    await open('docx');
    await page.locator('[data-doc-note="doc"]').waitFor({timeout:20_000});
    assert.match(await page.locator('[data-look="doc"]').textContent(),/项目周报/);
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
