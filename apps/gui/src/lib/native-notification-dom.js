/** Private native presenter. It mirrors resolved DOM and relays input only.
 * All notification state, timing, actions and gesture policy stay in React.
 * Self-contained so the identical bridge can be embedded as a WKUserScript. */
export function notificationDOM(scope, emit) {
  const doc = scope.document, stops = [];
  let observer, captured = null;
  function size() {
    const root = doc.querySelector('[data-notification-banners]');
    if (root) emit({h: root.getBoundingClientRect().height, radius: 0, dark: doc.documentElement.style.colorScheme === 'dark'});
  }
  function sync(old, next) {
    if (old.nodeType !== next.nodeType || old.nodeName !== next.nodeName) { old.replaceWith(next.cloneNode(true)); return; }
    if (old.nodeType === 3) { if (old.data !== next.data) old.data = next.data; return; }
    if (old.nodeType !== 1) return;
    for (const a of [...old.attributes]) if (!next.hasAttribute(a.name)) old.removeAttribute(a.name);
    for (const a of [...next.attributes]) if (old.getAttribute(a.name) !== a.value) old.setAttribute(a.name, a.value);
    // Preserve the identity of a held banner when an earlier notice leaves.
    const keyed = next.hasAttribute('data-notification-banners');
    const available = keyed ? new Map([...old.children].filter(n => n.hasAttribute('data-notification-banner')).map(n => [n.getAttribute('data-notification-banner'), n])) : null;
    [...next.childNodes].forEach((n, i) => {
      const key = n.nodeType === 1 && n.getAttribute('data-notification-banner');
      const match = key && available.get(key);
      if (match && old.childNodes[i] !== match) old.insertBefore(match, old.childNodes[i] || null);
      const current = old.childNodes[i];
      if (current) sync(current, n); else old.append(n.cloneNode(true));
    });
    while (old.childNodes.length > next.childNodes.length) old.lastChild.remove();
  }
  function input(event) {
    const banner = event.target.closest?.('[data-notification-banner]');
    const key = captured?.key || banner?.getAttribute('data-notification-banner');
    if (!key) return;
    if ((event.type === 'pointerover' || event.type === 'pointerout') && banner?.contains(event.relatedTarget)) return;
    if (event.type === 'keydown' && event.key !== 'Escape') return;
    const action = event.target.closest?.('[data-notification-dismiss]') ? 'dismiss' : 'open';
    if (event.type === 'pointerdown') captured = {key, element:banner, id:event.pointerId};
    if (event.type === 'pointermove' && captured && captured.id === event.pointerId && !captured.element.hasPointerCapture(event.pointerId)) {
      try { captured.element.setPointerCapture(event.pointerId); } catch {}
    }
    emit({input:{type:event.type,key,action,x:event.clientX||0,y:event.clientY||0,pointerId:event.pointerId||0,pointerType:event.pointerType||'mouse',button:event.button||0,isPrimary:event.isPrimary!==false,detail:event.detail||0}});
    if (event.type === 'pointerup' || event.type === 'pointercancel') captured = null;
    if (event.type === 'click' || event.type === 'keydown') event.preventDefault();
  }
  for (const type of ['pointerdown','pointermove','pointerup','pointercancel','pointerover','pointerout','click','keydown','focusin','focusout']) {
    doc.addEventListener(type,input); stops.push(() => doc.removeEventListener(type,input));
  }
  return {
    update(html) {
      const next = new scope.DOMParser().parseFromString(html,'text/html');
      doc.documentElement.style.colorScheme = next.documentElement.style.colorScheme;
      const current = doc.querySelector('[data-notification-banners]'), incoming = next.querySelector('[data-notification-banners]');
      if (!incoming) return;
      if (current) sync(current,incoming);
      else { doc.head.replaceChildren(...next.head.childNodes); doc.body.replaceChildren(...next.body.childNodes); }
      observer?.disconnect(); observer = new scope.ResizeObserver(size); observer.observe(doc.querySelector('[data-notification-banners]')); size();
    },
    dispose() { observer?.disconnect(); stops.forEach(stop=>stop()); }
  };
}
