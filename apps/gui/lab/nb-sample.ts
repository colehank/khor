// Lab only (lane nb-view): notebooks made up for the gate and for eyes —
// every kind of cell and output the preview draws, the attacks an output
// could carry, and a big one for timing. Nothing here is anybody's real
// notebook; the pictures are drawn on a canvas when asked for.

/** A small plot on a clear ground with black ink, the way matplotlib's
    inline backend saves one — what the light backing is for. */
export function plotPng(w = 320, h = 200, seed = 1): string {
  const c = document.createElement("canvas");
  c.width = w;
  c.height = h;
  const g = c.getContext("2d")!;
  g.strokeStyle = "#000";
  g.lineWidth = 1;
  g.strokeRect(30.5, 10.5, w - 40, h - 40);
  g.font = "10px sans-serif";
  g.fillStyle = "#000";
  g.fillText("x", w / 2, h - 10);
  g.beginPath();
  for (let x = 0; x < w - 40; x++) {
    const y = (h - 40) / 2 + Math.sin((x + seed * 13) / 17) * ((h - 60) / 2);
    if (x === 0) g.moveTo(31 + x, 11 + y);
    else g.lineTo(31 + x, 11 + y);
  }
  g.strokeStyle = "#1f77b4";
  g.stroke();
  return c.toDataURL("image/png").slice("data:image/png;base64,".length);
}

const lines = (s: string) => s.split(/(?<=\n)/);

const ESC = "\x1b";

/** IPython's colours on a traceback, plus a 256-colour and an RGB run. */
const TRACEBACK = [
  `${ESC}[0;31m---------------------------------------------------------------------------${ESC}[0m`,
  `${ESC}[0;31mZeroDivisionError${ESC}[0m                         Traceback (most recent call last)`,
  `Cell ${ESC}[0;32mIn[9], line 1${ESC}[0m`,
  `${ESC}[0;32m----> 1${ESC}[0m ${ESC}[38;5;208m1${ESC}[39m ${ESC}[38;5;241m/${ESC}[39m ${ESC}[38;2;10;120;200m0${ESC}[39m`,
  "",
  `${ESC}[1;31mZeroDivisionError${ESC}[0m: division by zero`,
];

export const PWN_HTML = [
  '<style scoped>.dataframe tbody tr th { vertical-align: top; } body { display: none }</style>',
  '<script>window.__nbPwned = "script"</script>',
  '<img src="x" onerror="window.__nbPwned = \'onerror\'" alt="坏图">',
  '<a href="javascript:window.__nbPwned=\'href\'">链接</a>',
  '<iframe src="https://example.com/html-frame"></iframe>',
  '<img src="https://example.com/html-https.png" alt="远程">',
  '<img src="http://example.com/html-http.png" alt="明文">',
  '<form action="https://example.com/html-form"><input name="q"></form>',
  '<table border="1" class="dataframe">',
  "  <thead>",
  '    <tr style="text-align: right;"><th></th><th>a</th><th>b</th><th>c</th></tr>',
  "  </thead>",
  "  <tbody>",
  "    <tr><th>0</th><td>1</td><td>0.5</td><td>甲</td></tr>",
  "    <tr><th>1</th><td>2</td><td>1.5</td><td>乙</td></tr>",
  "  </tbody>",
  "</table>",
  "<p>3 rows × 3 columns</p>",
].join("\n");

const SVG = '<svg xmlns="http://www.w3.org/2000/svg" width="80" height="40" onload="window.__nbPwned=\'svg-onload\'"><script>window.__nbPwned="svg-script"</script><rect width="80" height="40" fill="#e15759"/><text x="8" y="25">svg</text></svg>';

/** What each cell's outputs should be drawn as, in order — the gate
    compares `data-nb-out` against this. */
export const EXPECTED: string[][] = [];

export function sample(png: string): string {
  EXPECTED.length = 0;
  const cells: unknown[] = [];
  const md = (source: string, attachments?: object) => {
    cells.push({ cell_type: "markdown", metadata: {}, source: lines(source), ...(attachments ? { attachments } : {}) });
    EXPECTED.push([]);
  };
  const code = (source: string, count: number | null, outputs: object[], expect: string[]) => {
    cells.push({ cell_type: "code", execution_count: count, metadata: {}, source: lines(source), outputs });
    EXPECTED.push(expect);
  };
  md(
    [
      "# 一份样例 notebook",
      "",
      "欧拉恒等式 $e^{i\\pi}+1=0$，以及",
      "",
      "$$\\int_0^1 x\\,dx = \\tfrac{1}{2}$$",
      "",
      "附件里的图：![附图](attachment:dot%20one.png)",
      "",
      "| 列 | 值 |",
      "|---|--:|",
      "| a | 1 |",
      "",
      "<script>window.__nbPwned = 'markdown'</script>",
    ].join("\n"),
    { "dot one.png": { "image/png": png } },
  );
  code("import pandas as pd\ndf = pd.DataFrame({'a': [1, 2]})\ndf", 1, [
    { output_type: "execute_result", execution_count: 1, metadata: {}, data: { "text/html": lines(`${PWN_HTML}\n<img src="data:image/png;base64,${png}" alt="内嵌">`), "text/plain": ["   a\n0  1"] } },
  ], ["html"]);
  code("import matplotlib.pyplot as plt\nplt.plot(range(10))\nprint('done')", 2, [
    { output_type: "stream", name: "stdout", text: ["done\n"] },
    { output_type: "display_data", metadata: { "image/png": { width: 320, height: 200 } }, data: { "image/png": png, "text/plain": ["<Figure size 640x400 with 1 Axes>"] } },
  ], ["stdout", "image"]);
  code("from IPython.display import SVG\nSVG(s)", 3, [
    { output_type: "execute_result", execution_count: 3, metadata: {}, data: { "image/svg+xml": lines(SVG), "text/plain": ["<IPython.core.display.SVG object>"] } },
  ], ["image"]);
  code("fig = px.line(df)  # with a png backup\nfig.show()", 4, [
    { output_type: "display_data", metadata: {}, data: { "application/vnd.plotly.v1+json": { data: [], layout: {} }, "text/html": ["<div id='p'></div><script>window.__nbPwned='plotly'</script>"], "image/png": png } },
  ], ["image"]);
  code("fig = px.line(df)  # no backup\nfig", 5, [
    { output_type: "execute_result", execution_count: 5, metadata: {}, data: { "application/vnd.plotly.v1+json": { data: [], layout: {} }, "text/html": ["<div id='q'></div><script>window.__nbPwned='plotly2'</script>"], "text/plain": ["Figure({\n    'data': [], 'layout': {}\n})"] } },
  ], ["text"]);
  code("widgets.IntSlider(3)", 6, [
    { output_type: "execute_result", execution_count: 6, metadata: {}, data: { "application/vnd.jupyter.widget-view+json": { model_id: "abc", version_major: 2, version_minor: 0 }, "text/plain": ["IntSlider(value=3)"] } },
  ], ["text"]);
  code("output_notebook()", 7, [
    { output_type: "display_data", metadata: {}, data: { "application/javascript": ["window.__nbPwned = 'bokeh'"], "application/vnd.bokehjs_load.v0+json": "" } },
  ], ["script"]);
  code("1 / 0", 9, [{ output_type: "error", ename: "ZeroDivisionError", evalue: "division by zero", traceback: TRACEBACK }], ["error"]);
  code("import warnings\nwarnings.warn('old')\nfor i in range(3): print(i, end='\\r')", 10, [
    { output_type: "stream", name: "stderr", text: [`${ESC}[33m/tmp/x.py:2: FutureWarning: old${ESC}[0m\n`] },
    { output_type: "stream", name: "stdout", text: ["10%\r50%\r100%\n"] },
  ], ["stderr", "stdout"]);
  code("display(Markdown('**粗体** 与 $x^2$'), Latex(r'\\begin{equation}a^2+b^2=c^2\\end{equation}'), JSON({'k': [1, 2]}))", 11, [
    { output_type: "display_data", metadata: {}, data: { "text/markdown": ["**粗体** 与 $x^2$"], "text/plain": ["<IPython.core.display.Markdown object>"] } },
    { output_type: "display_data", metadata: {}, data: { "text/latex": ["\\begin{equation}a^2+b^2=c^2\\end{equation}"], "text/plain": ["<IPython.core.display.Latex object>"] } },
    { output_type: "display_data", metadata: {}, data: { "application/json": { k: [1, 2] }, "text/plain": ["<IPython.core.display.JSON object>"] } },
  ], ["markdown", "latex", "json"]);
  cells.push({ cell_type: "raw", metadata: {}, source: lines("raw *text* <b>as is</b>") });
  EXPECTED.push([]);
  code("# never run\nx = 1", null, [], []);
  code(`wide = ${"'" + "w".repeat(400) + "'"}\nprint(wide)`, 12, [{ output_type: "stream", name: "stdout", text: ["w".repeat(400) + "\n"] }], ["stdout"]);
  return JSON.stringify(
    {
      nbformat: 4,
      nbformat_minor: 5,
      metadata: { kernelspec: { name: "python3", display_name: "Python 3", language: "python" }, language_info: { name: "python", version: "3.12.0" } },
      cells,
    },
    null,
    1,
  );
}

/** A long notebook for timing: `n` cells, every third a plot. */
export function big(n: number, png: string): string {
  const cells: unknown[] = [];
  for (let i = 0; i < n; i++) {
    if (i % 3 === 0) cells.push({ cell_type: "markdown", metadata: {}, source: lines(`## 第 ${i} 节\n\n一段说明，带一个公式 $x_${i}$ 和 **粗体**。`) });
    else if (i % 3 === 1)
      cells.push({
        cell_type: "code",
        execution_count: i,
        metadata: {},
        source: lines(`import numpy as np\nx = np.linspace(0, 1, ${i})\ny = np.sin(x) * ${i}\nprint(y.sum())`),
        outputs: [{ output_type: "stream", name: "stdout", text: [`${i * 1.5}\n`] }],
      });
    else
      cells.push({
        cell_type: "code",
        execution_count: i,
        metadata: {},
        source: lines(`plt.plot(x, y)\nplt.title('${i}')`),
        outputs: [{ output_type: "display_data", metadata: {}, data: { "image/png": png, "text/plain": ["<Figure>"] } }],
      });
  }
  return JSON.stringify({ nbformat: 4, nbformat_minor: 5, metadata: { language_info: { name: "python" } }, cells }, null, 1);
}

export const V3 = JSON.stringify({ nbformat: 3, nbformat_minor: 0, metadata: {}, worksheets: [{ cells: [] }] });
