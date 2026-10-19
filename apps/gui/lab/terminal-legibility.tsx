import { useEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { Terminal } from '@xterm/xterm';
import { WebglAddon } from '@xterm/addon-webgl';
import { protectTerminal } from '@/lib/term-legibility';
import { keepTerminalGlass } from '@/lib/term-glass';
import { terminalAppearance, setTermTheme, termTheme, onTermTheme } from '@/lib/term-theme';
import { Button } from '@/components/ui/button';
import { TooltipProvider } from '@/components/ui/tooltip';
import '@/styles/tokens.css';
import '@/styles/app.css';
import '@xterm/xterm/css/xterm.css';
import './terminal-legibility.css';
// One stream, including explicit colours cached by a dark TUI.
const fixture = '\x1b[?1049h\x1b[?25h\x1b[0m主题切换后，程序仍输出深色块\r\n\x1b[2mDIM 默认背景：次要文字应当可读\x1b[0m\r\n\x1b[48;2;30;30;30m\x1b[38;2;26;26;20m深色块里的深色字\x1b[K\r\n\x1b[2mDIM 深色背景：忙碌状态与工具提示\x1b[K\x1b[0m\r\n\x1b[48;2;244;244;239m\x1b[38;2;238;238;231m浅色块里的浅色字\x1b[K\r\n\x1b[2mDIM 浅色背景：历史摘要与提示\x1b[K\x1b[0m\r\n\x1b[7;2m反色 DIM 文字\x1b[0m\r\n\x1b[48;2;30;30;30m\x1b[38;2;26;26;20m> 光标停在深色输入块\x1b[K\x1b[8;3H';
type Shape = 'block' | 'bar' | 'underline';
type Seat = HTMLDivElement & {
  __xterm?: Terminal;
};
function screens() { return [...document.querySelectorAll<Seat>('.legibility-terminal')].map(el => el.__xterm).filter((t): t is Terminal => !!t); }
function Screen({ guard, webgl, shape }: {
  guard: boolean;
  webgl: boolean;
  shape: Shape;
}) {
  const mount = useRef<Seat>(null), ref = useRef<Terminal | null>(null);
  useEffect(() => { const el = mount.current!, t = new Terminal({ cols: 46, rows: 10, fontSize: 17, fontFamily: 'monospace', allowProposedApi: true, ...terminalAppearance(el), cursorBlink: false, cursorStyle: shape, cursorInactiveStyle: shape, allowTransparency: false }); ref.current = t; el.__xterm = t; t.open(el); const stopGlass = keepTerminalGlass(t, el); const stop = guard ? protectTerminal(t) : () => { }; t.write(fixture, () => t.focus()); const off = onTermTheme(() => { t.options = terminalAppearance(el); t.refresh(0, t.rows - 1); }); return () => { off(); stop(); stopGlass(); t.dispose(); ref.current = null; delete el.__xterm; }; }, [guard]);
  // Replace only the renderer: exercise fallback and reattachment.
  useEffect(() => { const t = ref.current; if (!t || !webgl)
    return; const addon = new WebglAddon(); t.loadAddon(addon); const lost = addon.onContextLoss(() => addon.dispose()); return () => { lost.dispose(); addon.dispose(); }; }, [webgl]);
  useEffect(() => { const t = ref.current; if (t) {
    t.options.cursorStyle = shape;
    t.options.cursorInactiveStyle = shape;
  } }, [shape]);
  return <section data-legibility={guard ? 'protected' : 'before'}><h2>{guard ? '完整保护' : '现有呈现'}</h2><div ref={mount} className="legibility-terminal"/></section>;
}
function inspect() { return [...document.querySelectorAll<HTMLElement>('[data-legibility]')].map(el => { const seat = el.querySelector<Seat>('.legibility-terminal')!, t = seat.__xterm!, core = (t as any)._core, r = core._renderService._renderer.value, b = t.buffer.active, model = r._model?.cells; const spans = [...seat.querySelectorAll<HTMLElement>('.xterm-rows>div>span')].filter(s => s.textContent?.includes('DIM')).map(s => ({ text: s.textContent, color: getComputedStyle(s).color, bg: getComputedStyle(s).backgroundColor })); const cursor = seat.querySelector<HTMLElement>('.xterm-cursor'), css = cursor ? getComputedStyle(cursor) : null; return { name: el.dataset.legibility, renderer: r._rowFactory ? 'dom' : 'webgl', ratio: t.options.minimumContrastRatio, spans, cursor: css ? { shape: cursor!.className, color: css.color, bg: css.backgroundColor, outline: css.outlineColor, shadow: css.boxShadow, border: css.borderBottomColor } : null, cursorPosition: [b.cursorX, b.cursorY], cursorVars: t.element?.getAttribute('style'), cursorModel: model ? Array.from(model.slice((b.cursorY * t.cols + b.cursorX) * 4, (b.cursorY * t.cols + b.cursorX + 1) * 4)) : null, cursorRect: r._rectangleRenderer?.value ? Array.from(r._rectangleRenderer.value._verticesCursor.attributes.slice(0, 8)) : null, raw: Array.from({ length: 8 }, (_, y) => b.getLine(y)?.getCell(0)).map(c => ({ fg: c?.getFgColor(), bg: c?.getBgColor(), dim: c?.isDim() })), model: model ? Array.from({ length: 8 }, (_, y) => Array.from(model.slice(y * t.cols * 4, y * t.cols * 4 + 4))) : null, palette: { cursor: r._themeService.colors.cursor.css, accent: r._themeService.colors.cursorAccent.css }, selection: t.getSelection() }; }); }
function Lab() { const [dark, setDark] = useState(false), [webgl, setWebgl] = useState(false), [shape, setShape] = useState<Shape>('block'), [report, setReport] = useState(''); useEffect(() => { document.documentElement.dataset.theme = dark ? 'dark' : 'light'; }, [dark]); const move = (row: number) => screens().forEach(t => { t.clearSelection(); t.write(`\x1b[${row};3H`); }); return <TooltipProvider><main className="legibility-lab"><header><h1>终端文字与光标</h1><Button onClick={() => setDark(!dark)}>明暗</Button><Button onClick={() => setWebgl(!webgl)}>{webgl ? 'WebGL' : 'DOM'}</Button>{(['block', 'bar', 'underline'] as const).map(s => <Button key={s} onClick={() => setShape(s)}>{s}</Button>)}{[1, 3, 4.5, 7].map(n => <Button key={n} onClick={() => setTermTheme({ ...termTheme(), contrast: n })}>{n === 1 ? '关闭' : n}</Button>)}<Button onClick={() => setReport(JSON.stringify(inspect(), null, 2))}>检查呈现</Button></header><nav><Button onClick={() => move(8)}>深色格</Button><Button onClick={() => move(5)}>浅色格</Button><Button onClick={() => move(1)}>默认格</Button><Button onClick={() => move(7)}>反色格</Button><Button onClick={() => screens().forEach(t => t.select(0, 3, 20))}>选区</Button></nav><p>两侧接收完全相同的终端数据，设置与渲染器原位更新</p><div className="legibility-columns"><Screen guard={false} webgl={webgl} shape={shape}/><Screen guard webgl={webgl} shape={shape}/></div><output data-legibility-report>{report}</output></main></TooltipProvider>; }
const root = createRoot(document.getElementById('root')!);
root.render(<Lab />);
if (import.meta.hot) import.meta.hot.dispose(() => root.unmount());
