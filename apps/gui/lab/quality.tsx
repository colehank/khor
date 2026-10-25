import { useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import "@/styles/tokens.css";
import "./quality.css";

document.title = "khor · 滚动与焦点验收";

type Scene = "acp" | "khor";
const sessions: Record<Scene, string> = { acp: "gui/ai-elements-review", khor: "wide/all" };
const labels: Record<Scene, string> = { acp: "ACP 对话", khor: "Khor 对话" };
const chatUrl = (scene: Scene) => `/?${new URLSearchParams({
  phone: "1", labSession: sessions[scene],
  labTabs: Object.values(sessions).join(","),
})}`;
const commitUrl = `/lab/commit.html?${new URLSearchParams({
  id: "1".repeat(40), subject: "滚动与焦点验收",
})}`;
const longReply = "\n\n## 长回复验收\n\n" + Array.from({ length: 18 }, (_, index) =>
  `**第 ${index + 1} 段**　这是一段用于检查阅读位置的示例文字。停在末尾时，键盘升起后最新内容应留在输入框上方；上滑阅读历史后，键盘变化应保留正在看的位置。`
).join("\n\n") + "\n\n**长回复结束 · 检查这句话是否仍在输入框上方。**";

async function fixture(path: string, body?: unknown) {
  const response = await fetch(path, body === undefined ? { cache: "no-store" } : {
    method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body),
  });
  if (!response.ok) throw new Error("示例数据未载入，稍后重试");
  return response.json();
}

/** Only this loaded fixture document is changed. Its App and scroll owner stay mounted. */
function coverKeyboard(frame: HTMLIFrameElement) {
  const win = frame.contentWindow, doc = frame.contentDocument, viewport = win?.visualViewport;
  if (!win || !doc || !viewport) throw new Error("当前浏览器不支持键盘模拟，使用全屏入口验收");
  const before = Object.getOwnPropertyDescriptor(viewport, "height");
  const cover = Math.round(win.innerHeight * 0.4);
  Object.defineProperty(viewport, "height", { configurable: true, get: () => win.innerHeight - cover });
  const keyboard = doc.createElement("div");
  keyboard.dataset.qualityKeyboard = "";
  keyboard.setAttribute("aria-hidden", "true");
  keyboard.textContent = "键盘区域 · 浏览器模拟";
  keyboard.style.cssText = "position:fixed;inset:auto 0 0;block-size:var(--kb);display:grid;place-items:center;background:var(--fill-strong);color:var(--muted-foreground);font:inherit;border-top:var(--hair) solid var(--border);pointer-events:none";
  doc.body.append(keyboard);
  viewport.dispatchEvent(new Event("resize"));
  return () => {
    if (before) Object.defineProperty(viewport, "height", before);
    else Reflect.deleteProperty(viewport, "height");
    keyboard.remove();
    viewport.dispatchEvent(new Event("resize"));
  };
}

function QualityLab() {
  const [scene, setScene] = useState<Scene>("acp");
  const [ready, setReady] = useState(false), [keyboard, setKeyboard] = useState(false);
  const [pending, setPending] = useState(false), [generation, setGeneration] = useState(0);
  const [note, setNote] = useState("载入长回复后，在预览里滚到最下面开始验收");
  const [error, setError] = useState("");
  const [loaded, setLoaded] = useState<Record<Scene, boolean>>({ acp: false, khor: false });
  const preview = useRef<HTMLIFrameElement>(null);
  const releaseKeyboard = useRef<(() => void) | null>(null);
  const readyObserver = useRef<MutationObserver | null>(null);

  const lowerKeyboard = () => {
    releaseKeyboard.current?.(); releaseKeyboard.current = null; setKeyboard(false);
  };
  useEffect(() => () => { releaseKeyboard.current?.(); readyObserver.current?.disconnect(); }, []);

  const frameLoaded = () => {
    readyObserver.current?.disconnect();
    const doc = preview.current?.contentDocument;
    if (!doc) return;
    const check = () => {
      if (!doc.querySelector("[data-chat-scroll], [data-machine-chat-scroll]")) return;
      readyObserver.current?.disconnect(); setReady(true);
    };
    readyObserver.current = new MutationObserver(check);
    readyObserver.current.observe(doc, { childList: true, subtree: true }); check();
  };
  const selectScene = (next: Scene) => {
    if (next === scene) return;
    lowerKeyboard(); setReady(false); setScene(next); setError("");
    setNote("切换了示例会话，先检查当前阅读位置，再升起模拟键盘");
  };
  const toggleKeyboard = () => {
    setError("");
    if (keyboard) { lowerKeyboard(); setNote("模拟键盘已收起，继续检查当前阅读位置"); return; }
    try {
      if (!preview.current) return;
      releaseKeyboard.current = coverKeyboard(preview.current); setKeyboard(true);
      setNote("模拟键盘已升起，对话没有重新载入");
    } catch (cause) { setError(cause instanceof Error ? cause.message : "键盘模拟不可用"); }
  };
  const loadLongReply = async () => {
    setPending(true); setError("");
    try {
      if (scene === "acp") await fixture("/__ai_preview/acp", {
        id: sessions.acp,
        frames: [
          { kind: "answered", ask: 101, option: "deny" },
          { kind: "note", update: { update: { sessionUpdate: "agent_message_chunk", content: { type: "text", text: longReply } } } },
          { kind: "turn", stop: "EndTurn" },
        ],
      });
      else await fixture("/__ai_preview/khor-long", { text: longReply });
      setLoaded((current) => ({ ...current, [scene]: true }));
      setNote("长回复已载入。先滚到末尾，再升起、收起模拟键盘；然后上滑一段历史重复检查");
    } catch (cause) { setError(cause instanceof Error ? cause.message : "长回复未载入"); }
    finally { setPending(false); }
  };
  const reset = async () => {
    setPending(true); setError(""); lowerKeyboard();
    try {
      await fixture("/__ai_preview/reset");
      setLoaded({ acp: false, khor: false }); setReady(false); setGeneration((value) => value + 1);
      setNote("示例已重置，可以重新载入长回复开始验收");
    } catch (cause) { setError(cause instanceof Error ? cause.message : "示例未重置"); }
    finally { setPending(false); }
  };

  return <main className="quality-page">
    <header className="quality-intro">
      <p className="quality-eyebrow">khor · 验收 Lab</p>
      <h1>滚动与焦点</h1>
      <p>两条真实页面流程，一组隔离示例。这里不会连接真实智能体或修改工作区。</p>
      <nav className="quality-links" aria-label="验收项目">
        <a href="#chat-check">对话与键盘</a><a href="#commit-check">提交文件跳转</a><a href="#quality-finish">验收记录</a>
      </nav>
    </header>

    <section id="chat-check" className="quality-section" aria-labelledby="chat-title">
      <div className="quality-section-head"><div><p className="quality-eyebrow">01 · 对话</p><h2 id="chat-title">键盘变化，阅读位置留下来</h2></div></div>
      <p>先在末尾检查最新消息，再上滑阅读历史重复检查。ACP 与 Khor 都走一遍。</p>
      <div className="quality-controls">
        <div className="quality-segment" role="group" aria-label="选择对话">
          {(["acp", "khor"] as const).map((value) => <button key={value} type="button" aria-pressed={scene === value} disabled={pending} onClick={() => selectScene(value)}>{labels[value]}</button>)}
        </div>
        <button type="button" disabled={!ready || pending || loaded[scene]} onClick={loadLongReply}>{loaded[scene] ? "长回复已载入" : "载入长回复"}</button>
        <button type="button" aria-pressed={keyboard} disabled={!ready || pending} onClick={toggleKeyboard}>{keyboard ? "收起模拟键盘" : "升起模拟键盘"}</button>
      </div>
      <p className="quality-note" role="status">{pending ? "正在准备示例" : note}</p>
      {error && <p className="quality-error" role="alert">{error}</p>}
      <div className="quality-chat-stage">
        <iframe key={generation} ref={preview} onLoad={frameLoaded} src={chatUrl(scene)} title="对话验收预览" />
      </div>
      <div className="quality-instructions">
        <p><strong>真机键盘验收</strong>　用手机打开下方全屏入口，点击输入框唤起系统键盘。上面的按钮仅模拟可见区域变化，不等同于 iOS 真机键盘。</p>
        <div className="quality-links"><a href={chatUrl("acp")} target="_blank" rel="noreferrer">打开 ACP 全屏</a><a href={chatUrl("khor")} target="_blank" rel="noreferrer">打开 Khor 全屏</a></div>
        <ol>
          <li>停在长回复末尾，升起并收起键盘三次。末尾文字应始终留在输入框上方。</li>
          <li>上滑到“第 8 段”附近，再升起并收起键盘。正在读的段落应留在原位，不跳回末尾。</li>
          <li>在输入框写一段测试草稿，重复切换键盘。草稿应保留。再用另一个对话重复上述步骤。</li>
        </ol>
      </div>
    </section>

    <section id="commit-check" className="quality-section" aria-labelledby="commit-title">
      <div className="quality-section-head"><div><p className="quality-eyebrow">02 · 提交</p><h2 id="commit-title">选择文件后，停在目标位置</h2></div><a href={commitUrl} target="_blank" rel="noreferrer">打开提交全屏</a></div>
      <p>这条示例提交有 8 个文件，每个文件有多处修改。使用页面自己的“跳转到文件”菜单。</p>
      <ol>
        <li>选择第 6 个文件 module-6.ts，松开鼠标或手指后等待两秒。目标标题应留在可见位置，不弹回页首。</li>
        <li>滚回页首，再选择第 8 个文件 module-8.ts，重复检查。也用键盘打开菜单并选择一次。</li>
        <li>只打开菜单后按 Esc 取消。页面应保留打开菜单前的位置，焦点回到菜单按钮。</li>
      </ol>
      <iframe className="quality-commit-stage" key={`commit-${generation}`} src={commitUrl} title="提交文件跳转验收预览" loading="lazy" />
    </section>

    <footer id="quality-finish" className="quality-section quality-finish">
      <h2>记录这次验收</h2>
      <p>记录设备、浏览器、ACP 或 Khor、键盘升降前后的阅读位置。浏览器模拟与真机结果分开记录。</p>
      <ul><li>最新消息跟随可见区域</li><li>阅读历史时位置稳定，草稿保留</li><li>第 6、8 个文件跳转后不回弹</li></ul>
      <button type="button" disabled={pending} onClick={reset}>重置演示</button>
      <p className="quality-note">重置示例消息与预览页面，清除本轮未发送草稿。全屏页面需要刷新后重新验收。</p>
      <a href="#chat-check">回到对话验收</a>
    </footer>
  </main>;
}

const root = createRoot(document.getElementById("quality-root")!);
root.render(<QualityLab />);
if (import.meta.hot) import.meta.hot.dispose(() => root.unmount());
