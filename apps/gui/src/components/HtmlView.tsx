// Full browser rendering from an isolated, read-only file origin. Scripts,
// modules, styles, fonts, relative URLs and root URLs use normal browser rules.
import {useEffect,useMemo,useState} from 'react';
import {ResourcePreview} from '@/components/ResourcePreview';

export function onDisk(dir: string, rel: string): string | null {
  const r = rel.trim();
  if (r === "" || r.startsWith("#") || r.startsWith("//") || /^[a-z][a-z0-9+.-]*:/i.test(r)) return null;
  const clean = r.split(/[?#]/)[0];
  let decoded: string;
  try {
    decoded = decodeURIComponent(clean);
  } catch {
    decoded = clean;
  }
  const parts = (decoded.startsWith("/") ? decoded : `${dir}/${decoded}`).split("/");
  const out: string[] = [];
  for (const p of parts) {
    if (p === "" || p === ".") continue;
    if (p === "..") out.pop();
    else out.push(p);
  }
  return `/${out.join("/")}`;
}

/** Keep the existing preview-ground preference inside the browser document.
 * This adds a theme hint only; resources and scripts load through its origin. */
function useAppScheme(): 'light'|'dark' {
  const read=(): 'light'|'dark'=>document.documentElement.dataset.theme==='dark'?'dark':'light';
  const [scheme,setScheme]=useState(read);
  useEffect(()=>{const observer=new MutationObserver(()=>setScheme(read()));observer.observe(document.documentElement,{attributes:true,attributeFilter:['data-theme']});return()=>observer.disconnect();},[]);
  return scheme;
}

export function HtmlView({machine,path,text,ground}:{machine:string;path:string;text:string;ground:'auto'|'light'|'dark'}) {
  const app=useAppScheme(),scheme=ground==='auto'?app:ground;
  const html=useMemo(()=>{
    const document=new DOMParser().parseFromString(text,'text/html');
    let hint=document.querySelector<HTMLMetaElement>('meta[name="color-scheme"]');
    if(!hint){hint=document.createElement('meta');hint.name='color-scheme';hint.content=scheme;document.head.prepend(hint);}
    else if(ground!=='auto')hint.content=scheme;
    const doctype=document.doctype?new XMLSerializer().serializeToString(document.doctype):'';
    return doctype+document.documentElement.outerHTML;
  },[text,ground,scheme]);
  return <div className="flex min-h-0 flex-1 flex-col" style={{colorScheme:scheme}}>
    <ResourcePreview machine={machine} path={path} html={html}/>
  </div>;
}
