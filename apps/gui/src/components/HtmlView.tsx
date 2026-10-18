// An HTML file, rendered (CM-BRIEF step 3, the owner 2026-10-10: 「HTML
// 文件可以渲染」). The file's 渲染 face, behind the same switch Markdown's
// is (`rendererOf`'s "html"): the text in the editor, the page here.
//
// **Nothing from the file reaches the app's own window.** It is drawn in
// an `<iframe sandbox>` with an empty allow-list — no scripts, no same
// origin (so no reach into khor's storage or the bridge), no top
// navigation, no forms, no pop-ups — from `srcdoc`, never a URL. Scripts
// stay off; a 「允许脚本」 switch is a later decision (CM-BRIEF) and is
// not drawn. A policy goes in at the head too: nothing is fetched from
// the network (a page that phones home on being read would say who read
// it), only what is inlined below.
//
// **Its relative pictures and style sheets come from the machine it is
// on**, read through the file API khor already has (`fetchPreview` for a
// picture, `fetchFileText` for a sheet) and inlined as `data:` and
// `<style>` before the page is drawn — cheap because there are few, and
// capped so a page with hundreds is drawn with what fits. The parse that
// finds them is a `DOMParser` document, which runs no script and loads
// nothing. A picture or a sheet that could not be read stays broken.
import { useEffect, useState } from "react";

import { fetchFileText, fetchPreview } from "@/api";

/** At most this many pictures and sheets are fetched for one page. */
const FETCH_MAX = 24;

/** No network: pictures, fonts and sheets only from `data:` and what is
    inline. */
const POLICY = "default-src 'none'; img-src data:; media-src data:; font-src data:; style-src 'unsafe-inline' data:";

/** `rel` against the file's own directory, or null for anything that is
    not a path on that machine (a URL with a scheme, a `data:`, `//host`,
    an anchor). The Markdown preview reads its pictures the same way. */
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

/** The page as it will be drawn: the policy, the colour scheme, and what
    could be read inlined. */
async function prepared(machine: string, path: string, html: string, scheme: "light" | "dark"): Promise<string> {
  const doc = new DOMParser().parseFromString(html, "text/html");
  const dir = path.slice(0, path.lastIndexOf("/")) || "/";
  let budget = FETCH_MAX;
  const jobs: Promise<void>[] = [];
  for (const img of doc.querySelectorAll<HTMLImageElement>("img[src]")) {
    const at = onDisk(dir, img.getAttribute("src") ?? "");
    if (at === null || budget <= 0) continue;
    budget -= 1;
    img.removeAttribute("srcset");
    jobs.push(
      fetchPreview(machine, at).then(
        (p) => {
          if (!p.cut && p.data !== "") img.setAttribute("src", `data:${p.media};base64,${p.data}`);
        },
        () => {},
      ),
    );
  }
  for (const link of doc.querySelectorAll<HTMLLinkElement>('link[rel~="stylesheet"][href]')) {
    const at = onDisk(dir, link.getAttribute("href") ?? "");
    if (at === null || budget <= 0) continue;
    budget -= 1;
    jobs.push(
      fetchFileText(machine, at).then(
        (f) => {
          if (f.binary) return;
          const style = doc.createElement("style");
          style.textContent = f.text;
          link.replaceWith(style);
        },
        () => {},
      ),
    );
  }
  await Promise.all(jobs);
  const policy = doc.createElement("meta");
  policy.setAttribute("http-equiv", "Content-Security-Policy");
  policy.setAttribute("content", POLICY);
  const colours = doc.createElement("meta");
  colours.setAttribute("name", "color-scheme");
  colours.setAttribute("content", scheme);
  doc.head.prepend(policy, colours);
  return `<!doctype html>${doc.documentElement.outerHTML}`;
}

/** The app's own theme as it is now, following its changes. */
function useAppScheme(): "light" | "dark" {
  const read = (): "light" | "dark" => (document.documentElement.dataset.theme === "dark" ? "dark" : "light");
  const [scheme, setScheme] = useState(read);
  useEffect(() => {
    const mo = new MutationObserver(() => setScheme(read()));
    mo.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    return () => mo.disconnect();
  }, []);
  return scheme;
}

export function HtmlView({
  machine,
  path,
  text,
  ground,
}: {
  machine: string;
  /** Absolute, on that machine: where its relative paths start. */
  path: string;
  /** The text in the editor — what is drawn is the buffer, not the disk,
      as Markdown's face is. */
  text: string;
  /** 预览背景: the ground it is read on, or 「跟随」 the app's theme. */
  ground: "auto" | "light" | "dark";
}) {
  const app = useAppScheme();
  const scheme = ground === "auto" ? app : ground;
  const [page, setPage] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    // A pause while typing, so a page is not re-read per keystroke.
    const t = setTimeout(() => {
      void prepared(machine, path, text, scheme).then((p) => live && setPage(p));
    }, page === null ? 0 : 300);
    return () => {
      live = false;
      clearTimeout(t);
    };
    // `page` only picks the first delay.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [machine, path, text, scheme]);
  if (page === null) return <div className="flex-1" />;
  return (
    <iframe
      data-html-frame
      // The whole allow-list: empty. Every one of scripts, same origin,
      // top navigation, forms, pop-ups and modals is refused.
      sandbox=""
      srcDoc={page}
      title={path.split("/").pop() ?? path}
      referrerPolicy="no-referrer"
      className="min-h-0 w-full flex-1 border-0"
      style={{ colorScheme: scheme }}
    />
  );
}
