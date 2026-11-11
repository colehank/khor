// Full browser rendering from an isolated, read-only file origin. Scripts,
// modules, styles, fonts, relative URLs and root URLs use normal browser rules.
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

export function HtmlView({machine,path,text,ground}:{machine:string;path:string;text:string;ground:'auto'|'light'|'dark'}) {
  return <div className="flex min-h-0 flex-1 flex-col" style={{colorScheme:ground==='auto'?'light dark':ground}}>
    <ResourcePreview machine={machine} path={path} html={text}/>
  </div>;
}
