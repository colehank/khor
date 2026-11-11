import {animateIcon} from './icon-story';
const clamp = (value: number) => Math.min(1, Math.max(0, value));
const ramp = (value: number, from: number, to: number) => {
  const t = clamp((value - from) / (to - from));
  return t * t * (3 - 2 * t);
};
const mix = (a: number, b: number, t: number) => a + (b - a) * t;

export function mountStory() {
  const opening = document.querySelector<HTMLElement>('[data-opening]');
  const handoff = document.querySelector<HTMLElement>('[data-handoff]');
  const assembly = document.querySelector<HTMLElement>('[data-assembly]');
  const stage = document.querySelector<HTMLElement>('[data-device-stage]');
  const screen = document.querySelector<HTMLDialogElement>('[data-screen]');
  const frame = document.querySelector<HTMLIFrameElement>('[data-demo-frame]');
  const launch = document.querySelector<HTMLButtonElement>('[data-launch]');
  if (!opening || !handoff || !assembly || !stage || !screen || !frame || !launch) return;
  const loading = document.querySelector<HTMLElement>('[data-demo-loading]')!;
  const lines = [...document.querySelectorAll<HTMLElement>('[data-line]')];
  const steps = [...document.querySelectorAll<HTMLButtonElement>('[data-step]')];
  const reduced = matchMedia('(prefers-reduced-motion: reduce)');
  const starts = [0, .18, .38, .55];
  const paintIcon = animateIcon(assembly);
  const iconImage = launch.querySelector<HTMLImageElement>('img')!;
  let iconHome = {left:0,top:0,width:0};
  const laptop = stage.querySelector<HTMLElement>('.laptop')!;
  const lid = stage.querySelector<HTMLElement>('.laptop-lid')!;
  const base = stage.querySelector<HTMLElement>('.laptop-base')!;
  let frameKeys: AbortController | null = null;
  let requested = false;
  let opened = false;
  let scheduled = false;
  let disposed = false;
  let handoffProgress = 0;
  let topOpening = 0, topHandoff = 0;
  let openingTravel = 1, handoffTravel = 1;
  const controller = new AbortController();
  const on = { signal: controller.signal };
  const property = (element: HTMLElement, name: string, value: number | string) => element.style.setProperty(name, String(value));

  function prepareDemo() {
    if (requested) return;
    requested = true;
    frame!.src = frame!.dataset.src!;
  }
  function openDemo(focus = false) {
    prepareDemo();
    opened = true;
    launch!.classList.add('is-open');
    launch!.tabIndex = -1;
    launch!.setAttribute('aria-hidden', 'true');
    frame!.removeAttribute('inert');
    if (focus) frame!.focus({ preventScroll: true });
  }
  frame.setAttribute('inert', '');
  frame.addEventListener('load', () => {
    if (!requested) return;
    loading.hidden = true;
    frameKeys?.abort();
    frameKeys = new AbortController();
    frame!.contentDocument?.addEventListener('keydown', event => {
      if (event.key === 'Escape' && screen!.classList.contains('is-expanded')) {
        event.preventDefault(); event.stopImmediatePropagation(); collapseDemo();
      }
    }, {capture:true, signal:frameKeys.signal});
    requestRender();
  }, on);
  launch.addEventListener('click', () => openDemo(true), on);

  function geometry(progress: number) {
    const width = stage!.clientWidth, height = stage!.clientHeight;
    const laptopWidth = Math.min(width * .82, height * 1.55);
    const laptopHeight = laptopWidth / 1.6;
    const left = (width - laptopWidth) / 2;
    const top = (height - laptopHeight) / 2;
    const bezel = Math.max(5, laptopWidth * .009);
    const phoneHeight = Math.min(height * .89, 630);
    const phoneWidth = phoneHeight / 2;
    const phoneLeft = (width - phoneWidth) / 2;
    const phoneTop = (height - phoneHeight) / 2;
    const change = ramp(progress, .50, .67);
    const phoneVisible = ramp(progress, .58, .73);
    handoff!.style.setProperty('--phone-progress', String(ramp(progress, .43, .58)));
    property(stage!, '--phone-progress', phoneVisible);
    property(stage!, '--laptop-opacity', 1 - ramp(progress, .68, .83));
    property(stage!, '--laptop-y', `${ramp(progress, .58, .83) * 16}%`);
    property(stage!, '--lid-angle', `${-ramp(progress, .40, .59) * 88}deg`);
    lid.style.cssText = `left:${left}px;top:${top}px;width:${laptopWidth}px;height:${laptopHeight}px`;
    base.style.cssText = `left:${left - laptopWidth * .1}px;top:${top + laptopHeight}px;width:${laptopWidth * 1.2}px`;
    const sx = mix(left + bezel, phoneLeft, change);
    const sy = mix(top + bezel * 1.6, phoneTop, change);
    const sw = mix(laptopWidth - bezel * 2, phoneWidth, change);
    const sh = mix(laptopHeight - bezel * 2.2, phoneHeight, change);
    property(stage!, '--screen-left', `${sx}px`);
    property(stage!, '--screen-top', `${sy}px`);
    property(stage!, '--screen-w', `${sw}px`);
    property(stage!, '--screen-h', `${sh}px`);
    property(stage!, '--screen-radius', `${mix(3, 30, change)}px`);
    const shutting = ramp(progress, .40, .53);
    property(stage!, '--screen-angle', `${progress < .55 ? -shutting * 88 : 0}deg`);
    screen!.style.opacity = String(progress < .55 ? 1 - ramp(progress, .48, .55) : ramp(progress, .58, .69));
    const full = screen!.classList.contains('is-expanded');
    const availableWidth = full ? screen!.clientWidth : sw;
    const availableHeight = full ? screen!.clientHeight : sh;
    const viewportWidth = full ? availableWidth : change < .5 ? 1200 : 390;
    const scale = full ? 1 : availableWidth / viewportWidth;
    property(screen!, '--frame-w', `${viewportWidth}px`);
    property(screen!, '--frame-h', `${availableHeight / scale}px`);
    property(screen!, '--frame-scale', scale);
    const appRoot = frame!.contentDocument?.documentElement;
    if (appRoot) {
      appRoot.style.setProperty('--safe-top', viewportWidth < 760 ? '32px' : '0px');
      appRoot.style.setProperty('--safe-bottom', viewportWidth < 760 ? '24px' : '0px');
    }
    const cursor = ramp(progress, .02, .13);
    property(stage!, '--cursor-x', `${mix(63, 50, cursor)}%`);
    property(stage!, '--cursor-y', `${mix(65, 50, cursor)}%`);
    property(stage!, '--cursor-opacity', ramp(progress, 0, .03) * (1 - ramp(progress, .16, .20)));
    property(stage!, '--cursor-scale', progress > .13 && progress < .15 ? .84 : 1);
  }

  function render() {
    scheduled = false;
    if (disposed) return;
    const scroll = window.scrollY;
    const progress = reduced.matches ? 1 : clamp((scroll - topOpening) / openingTravel);
    const active = starts.findLastIndex(start => progress >= start);
    lines.forEach((line, index) => {
      const start = starts[index], end = starts[index + 1] ?? 1.15;
      const visible = (index === 0 ? 1 : ramp(progress, start, start + .025)) * (1 - ramp(progress, end - .035, end));
      line.style.opacity = String(visible);
      line.style.transform = `translateY(${(1 - visible) * 14}px)`;
      line.classList.toggle('is-current', index === active);
    });
    steps.forEach((step, index) => {
      if (index === active) step.setAttribute('aria-current', 'step');
      else step.removeAttribute('aria-current');
    });
    paintIcon(progress);
    property(opening!,'--story-controls-opacity',1-ramp(progress,.9,1));
    handoffProgress = reduced.matches ? 0 : clamp((scroll - topHandoff) / handoffTravel);
    if (scroll + innerHeight > topHandoff - innerHeight * .5) prepareDemo();
    if (!reduced.matches && handoffProgress > .145 && scroll < topHandoff + handoffTravel + innerHeight) {
      if (!opened) openDemo();
    }
    geometry(handoffProgress);
    handoff!.querySelector<HTMLElement>('.handoff-copy')!.style.opacity = reduced.matches ? '1' : String(ramp(scroll, topHandoff-innerHeight*.25, topHandoff+handoffTravel*.07));
    const departure = topOpening + openingTravel;
    const travelling = !reduced.matches && scroll > departure;
    assembly!.classList.toggle('is-travelling', travelling);
    if(travelling){
      const target=iconImage.getBoundingClientRect();
      const t=ramp(scroll,departure,topHandoff+handoffTravel*.10);
      assembly!.style.left=`${mix(iconHome.left,target.left,t)}px`;
      assembly!.style.top=`${mix(iconHome.top,target.top,t)}px`;
      assembly!.style.width=`${mix(iconHome.width,target.width,t)}px`;
      property(stage!,'--launch-icon-opacity',0);
    }else{
      for(const name of ['left','top','width'])assembly!.style.removeProperty(name);
      property(stage!,'--launch-icon-opacity',1);
    }
    assembly!.style.opacity=opened&&scroll>=topHandoff?'0':'1';
  }
  function requestRender() {
    if (scheduled) return;
    scheduled = true;
    requestAnimationFrame(render);
  }
  function measure() {
    topOpening = opening!.getBoundingClientRect().top + scrollY;
    topHandoff = handoff!.getBoundingClientRect().top + scrollY;
    openingTravel = Math.max(1, opening!.offsetHeight - innerHeight);
    handoffTravel = Math.max(1, handoff!.offsetHeight - innerHeight);
    assembly!.classList.remove('is-travelling');
    for(const name of ['left','top','width'])assembly!.style.removeProperty(name);
    const iconRect=assembly!.getBoundingClientRect();
    const sticky=opening!.querySelector<HTMLElement>('.opening-sticky')!.getBoundingClientRect();
    iconHome={left:iconRect.left,top:iconRect.top-sticky.top,width:iconRect.width};
    requestRender();
  }
  steps.forEach((step, index) => step.addEventListener('click', () => {
    window.scrollTo({top: topOpening + openingTravel * (starts[index] + .045), behavior: reduced.matches ? 'instant' : 'smooth'});
  }, on));
  document.querySelector('[data-reset]')?.addEventListener('click', () => {
    loading.hidden = false;
    frame!.src = frame!.dataset.src!;
    if (!opened) openDemo();
  }, on);
  const expandButton = document.querySelector<HTMLButtonElement>('[data-fullscreen]')!;
  const closeExpanded = document.querySelector<HTMLButtonElement>('[data-close-expanded]')!;
  let previousOverflow = '';
  function collapseDemo() {
    screen!.close();
    screen!.classList.remove('is-expanded');
    screen!.show();
    closeExpanded.hidden = true;
    document.documentElement.style.overflow = previousOverflow;
    expandButton.focus({preventScroll:true});
    requestRender();
  }
  expandButton.addEventListener('click', () => {
    if (!opened) openDemo();
    previousOverflow = document.documentElement.style.overflow;
    screen!.close();
    screen!.classList.add('is-expanded');
    screen!.showModal();
    closeExpanded.hidden = false;
    closeExpanded.focus();
    document.documentElement.style.overflow = 'hidden';
    requestRender();
  }, on);
  closeExpanded.addEventListener('click', collapseDemo, on);
  screen.addEventListener('cancel', event => {
    if (screen!.classList.contains('is-expanded')) {event.preventDefault();collapseDemo();}
  }, on);
  document.querySelectorAll<HTMLButtonElement>('[data-copy]').forEach(button => button.addEventListener('click', async () => {
    const value = document.getElementById(button.dataset.copy!)?.textContent ?? '';
    try { await navigator.clipboard.writeText(value); button.textContent = '已复制'; }
    catch { button.textContent = '复制未成功'; }
    setTimeout(() => { if (button.isConnected) button.textContent = '复制'; }, 1800);
  }, on));
  window.addEventListener('scroll', requestRender, { passive: true, ...on });
  window.addEventListener('resize', measure, on);
  reduced.addEventListener('change', measure, on);
  const resize = new ResizeObserver(measure);
  resize.observe(stage);
  measure();
  window.addEventListener('pagehide', event => {
    if (event.persisted) return;
    disposed = true; controller.abort(); frameKeys?.abort(); resize.disconnect();
  }, on);
}
