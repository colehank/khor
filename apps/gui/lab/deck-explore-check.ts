/** Production DeckBand handlers and CSS; synthetic capture, not an iPhone test. */
export async function checkExplore(){
 const wait=(ms:number)=>new Promise(r=>setTimeout(r,ms));
 const b=document.querySelector<HTMLElement>('[data-deck-band]')!,r=b.getBoundingClientRect(),x=r.left+r.width/2,y=r.top+r.height/2;
 const original={set:b.setPointerCapture,has:b.hasPointerCapture,release:b.releasePointerCapture};let held=false;
 b.setPointerCapture=()=>{held=true};b.hasPointerCapture=()=>held;b.releasePointerCapture=()=>{held=false};
 const p=(type:string,dx=0)=>b.dispatchEvent(new PointerEvent(type,{bubbles:true,cancelable:true,pointerId:98,isPrimary:true,pointerType:'touch',button:0,clientX:x+dx,clientY:y}));
 const results:Array<{label:string;pass:boolean;detail?:unknown}>=[];
 const check=(pass:boolean,label:string,detail?:unknown)=>results.push({label,pass,detail});
 const count=()=>Number(document.querySelector('[data-proof-count]')!.textContent);
 try{
  const before=count(),active=document.querySelector('[data-proof-open]')!.textContent;
  p('pointerdown');await wait(430);
  const overlay=document.querySelector<HTMLElement>('[data-deck-selector]')!;
  check(!!overlay&&!overlay.classList.contains('glass')&&!overlay.querySelector('.deck-selector-focus'),'探索使用压暗内容、无玻璃底板或额外待选圈');
  const box=()=>{const t=document.querySelector<HTMLElement>('.deck-selector-title')!,c=getComputedStyle(t);return {width:t.clientWidth,height:t.clientHeight,font:c.fontSize,line:c.lineHeight,weight:c.fontWeight};};
  const title=box(),cs=getComputedStyle(b),step=parseFloat(cs.getPropertyValue('--glass-btn'))+3*parseFloat(cs.getPropertyValue('--gap-pane'));
  const samples=[title];for(const shift of [.12,.26,.34,0]){p('pointermove',-step*shift);await wait(25);samples.push(box());}
  check(new Set(samples.map(v=>JSON.stringify(v))).size===1,'拖动不改变标题排版',samples);
  check(title.font===getComputedStyle(b.querySelector('[data-deck-title]')??b).fontSize,'中央名称使用 body');
  check(title.weight==='400','中央名称使用常规字重');
  p('pointercancel');await wait(30);
  p('pointerdown');await wait(430);p('pointermove',-r.width*.3);await wait(1640);
  const candidate=b.dataset.bandPreview;p('pointermove',-r.width*.3+14);await wait(250);
  check(b.dataset.bandDirection==='0'&&b.dataset.bandPreview!==candidate,'同侧反向回上一项并停止自动');
  await wait(1640);check(b.dataset.bandDirection==='-1','停留后沿最新滑动方向自动');
  check(count()===before,'探索期间未切换内容');
  p('pointerup');await wait(165);
  check(!!document.querySelector('[data-deck-selector]')&&count()===before,'停靠后淡出期间尚未确认');
  window.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}));await wait(180);
  check(count()===before&&!document.querySelector('[data-deck-selector]'),'淡出期间 Escape 可取消');
  check(b.querySelector('[data-band-seg=cur]')?.textContent===active||b.querySelector('[data-band-seg=cur]')?.textContent?.endsWith(active??''),'取消恢复实际打开的药丸');
  p('pointerdown');await wait(430);p('pointermove',-80);await wait(250);window.dispatchEvent(new Event('blur'));await wait(30);
  check(!document.querySelector('[data-deck-selector]')&&count()===before,'后台中断不会迟到确认');
  return {pass:results.every(v=>v.pass),results};
 }catch(e){return {pass:false,error:String(e),results};}
 finally{p('pointercancel');b.setPointerCapture=original.set;b.hasPointerCapture=original.has;b.releasePointerCapture=original.release;}
}
