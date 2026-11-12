// Local-only isolation modes for a real-device Safari zoom reproduction.
// Normal visits and production builds retain the complete presentation.
export function renderProbe() {
  if(!import.meta.env.DEV)return null;
  const value=new URLSearchParams(location.search).get('render-probe');
  if(value!=='poster'&&value!=='no-fog'&&value!=='no-glass'&&value!=='no-stream'&&value!=='no-backdrop')return null;
  document.documentElement.dataset.renderProbe=value;
  return value;
}

export function disablePreviewGlass(doc:Document) {
  if(doc.getElementById('khor-render-probe-glass'))return;
  const style=doc.createElement('style');
  style.id='khor-render-probe-glass';
  style.textContent='*,*::before,*::after{-webkit-backdrop-filter:none!important;backdrop-filter:none!important}.site-fog-surface{filter:none!important}';
  doc.head.append(style);
}
