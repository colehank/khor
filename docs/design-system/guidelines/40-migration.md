# 从 tokens.css 迁移

这份系统写的是 D1–D10 之后要做成的样子。下表是它与仓库里 `apps/gui/src/styles/tokens.css` **今天的值**的全部差异。改的时候，这份系统和 `tokens.css` 在同一个 commit 里改完，`check-boot` 负责比对。

| # | token | tokens.css 今天 | 这份系统 | 为什么 |
|---|---|---|---|---|
| D1 | `body` 字号 | `--text-base: 14px` | **13px** | 跟 macOS 对齐。`read` 保持 16，长读的地方不跟着降。 **已落地（第十五轮）**：八个界面档 + read/doc-h/mono/term 共 14 个 type style 进了 `tokens.css`，旧的 `xs/sm/base/lg`、`title/fact/aux`、`hero`、`code` 撤掉，`check-type` 按名拒收；`read` 的行高 1.5（16 上 24px），主人 2026-09-27 定，不再动；`tokens.json` 已跟上，`check-boot` 里原来那条具名例外撤了，两边不一致就红。 |
| D6 | 选中行 | `--accent`（16% 的淡底）+ `ink` | **`fill-strong` 灰块 + 全行 `ink`**，前台标题 600 | 第十五轮先落成 `brand-strong` 实心 + `on-brand`；2026-09-27 负责人嫌「太亮」，额度的环在实心绿上看不清，改成 Finder 侧边栏那种中性灰块。`app.css` 按 `data-selected` 画；`check-palette` 两主题 × 前台/失焦量，并且选中行任何时候是 `brand-strong` 就红。 |
| — | 主操作 | `--primary: #1a1a14`（墨） | **`brand-strong`** | 当初把主操作改成墨，是因为 `#93b04d` 对白只有 2.45。`brand-strong` 对白 4.88，岗位可以回到橄榄绿。 **已落地（第十五轮）**：`--primary: var(--brand-strong)`。 |
| — | 焦点环 | `--ring: #1a1a14`（墨） | **`ring: #5d7b06 / #99bb36`** | 同上。在 `card` 上亮 4.75 / 暗 7.98，都过 3:1。 **已落地（第十五轮）**。 |
| — | 控件高度 | `--ctl-sm/md/lg: 24 / 30 / 40` | **22 / 28 / 32** | 对齐 macOS 的控件高度；40 只在指针是手指时由 `touch` 44 接管。 **已落地（第十五轮）**：值改了；按钮与设置行按新高度重排在第十七轮。 |
| — | 圆角 | `--radius: 10px` 一档 + calc 派生 | **六档具名：4 / 6 / 8 / 10 / 12 / full** | 一档基数 + `calc` 的派生法让「按钮用几」这种问题没有答案。 **已落地（第十五轮）**：`@theme static` 里六个直写值，板用 `radius-xl`。 |
| — | 设置组间距 | `--gap-group: 18px` | **20px** | 让 4 的倍数不留例外。 **已落地（第十五轮）**。 |
| — | R0/L0 高度 | `--pane-head: calc(44 + 4) = 48` | **`pane-head: 48px` 直写** | 值一样，但它是一个设计决定，不该是两个别的数算出来的副产品。 **已落地（第十五轮）**；`traffic.rs` 的 48 由 `check-boot` 对。2026-09-29 起是 38，见 D15。 |
| D2 | 没有标签的页 R0 | 各页各拼 | **`title-sm` 标题 + 右端页面按钮，不带后退/前进** | 第十五轮先照「系统设置」加了 ‹ ›；2026-09-26 负责人撤掉：「机器等非标签页左上角不用有返回与前进，就是标题就行」。`PageHead`（`components/Region.tsx`）。 |
| D3 | L2 列表空白 | 能拖窗口 | **不能拖** | Finder 和系统设置都不能；竖栏空白照旧能拖。 **已落地（第十五轮）**：外壳在 L0、L1、R0 各标一次（`components/shell.ts`），face 拖动普查。 |
| D4 | 左块底 | `--side` 实色 | **macOS 用 `sidebar` 材质**，其余平台 `side` | 半透明透出桌面，是「随系统装的 app」最明显的一处。 **代码已落地（第十五轮），真窗口待领队看**：`transparent` + `Effect::Sidebar` + `macOSPrivateApi`，左块 `--side` 只在 `:root[data-material]` 的 `[data-side]` 里清空。 |
| D7–D10 | 页面结构 | 六种各自拼 | **三种页型 + sheet + R0 右端按钮** | 见《骨架与三种页型》。 **第十五轮只落了页型骨架**（`Region kind`、`data-region-kind`）；sheet、R0 右端按钮在第十七轮。 |
| **D11** | 竖栏打开态 | `--rail-on-ink: #303a19 / #dce9a6`，彩度只用掉该亮度上限的 64% | **`#5d7b06` / `#c6f560`**（亮色 09-25 先是 `#2d3e00`，09-27 负责人嫌「没有颜色」，门从「比未打开深 2.0」改成「按色相」，亮色换成 `brand-strong`） | **已落地**。`--rail-ink` 一个字没动——`check-rail` 的 `LINE_FLOOR`/`FLOOR` 把静止态钉死了。组件早就在读 `var(--rail-on-ink)`（`App.tsx:363/1430`），所以**只改 `tokens.css` 两个值**，不动 `app.css`。 |
| **D12** | `data-body` 的作用域 | `[data-rail-item] svg [data-body]`——只在竖栏里生效 | **`svg [data-body]` 普遍生效** | **已落地**（`app.css` 一行）。实心块是家族的**密度**规则，不是状态标记；状态标记是叠在它上面的 `fill-opacity: 1`，那一条仍然只在竖栏里。不放宽的话，重画后带实心块的 `gear` `shield` `network` `pulse`（出现在「…」菜单里）会是空心的。 |
| **D13** | 品牌绿家族 | `--lq1/2/3`、`--accent` 的现值 | **整族彩度 ×1.28**：`brand` #93b04d → **#8fb31e**，`brand-strong` #5f7a1f → **#5d7b06**，`lq1` #d5e595 → **#d4e77d**，`lq2` #bcd47b → **#b9d75e**（暗色同理） | 在 OKLCH 里锁死亮度和色相，只动彩度，所以**所有对比度一个没变**（白在 brand-strong 上 4.89 → 4.88）。原来那层灰扒掉之后，选中行才像个强调色而不是军绿。**已落地**，`tokens.css` 八个值 + `crates/core/src/avatar.rs` 二十八处（含那段用 hex 写的选色推演）。按 `pick()` 复算过：ground 仍是 `liquid[2]`、first 仍是 `#3f4482`，**选色顺序完全没变**，只是换了写法。 |
| **D14** | 暗色浮层的底 | `--popover: #191917`（与 `card` 同值） | **`#242422`**，比 `card` 高 5.4 L\*；亮色不动 | 2026-09-28 审计：暗色里新会话 sheet 和板同色、看不出影子，读成平的。`float` 的黑影落在近黑的板上等于没有，macOS 暗色的菜单、弹层、sheet 都比底下的窗口亮。量过的代价：`ink` 12.39、`secondary` 5.72、`destructive` 4.85、最低状态词 6.09，`tertiary` 3.55（在它的例外里）。**已落地**：`tokens.css` 一个值；sheet 从 `bg-card shadow-lg`、侧栏搜索结果从 `bg-card shadow-md`、两处补全菜单从 `shadow-md` 改穿 `popover` + `float`。`check-palette` 守「暗色至少高 4 L\*」和三对 4.5，`check-face` 4I 在暗色里量真的 sheet、对话框、右键菜单、搜索结果。**2026-09-30 被 D16 重推**：板变成最黑的纸（`#0f0f0e`），框升到 `#242422`（就是这里的旧值），浮层再上一档到 `#2e2e2c`，比框高 4.74、比纸高 14.58。 |
| **D15** | 第一行（L0 + R0） | `--pane-head: 48px`，`--tab-lip: 4px`（在标签条上方，标签站在行底，离窗口上沿 20） | **`pane-head` 38、`tab-lip` 5**；指针是手指时 48 / 2 | 2026-09-29，负责人：「怎么标签页到app上沿有空白空间 可以整体顶到上沿，为显示内容腾出空间」，选了「整行收紧到 38」——macOS 紧凑工具栏的高度，每页多 10 px。三颗窗口按钮、搜索框、标题、标签的签、页面按钮在一条中线上（y 19）：标签是 28 的签落在中线上，下面 5 的脚接进板，离上沿也是 5。工作区里三栏的头跟着到 38（同一个 token，彼此对齐）。两行标题字号不变，行距各收一档（18 + 14）。**行高跟指针，不跟宽度**：手指（`pointer: coarse`）回到 48，签 44 上下各 2；窄档的 Mac 窗口用鼠标，照样 38，三颗按钮才一直在中线上。**已落地**：`tokens.css` 两个值 + coarse 段两个值、`traffic.rs` `PANE_HEAD` 38（关闭按钮中心 y 24 → 19）、`TabStrip` 的签与脚；`check-boot` 守「行 = 签 + 上下各一脚」两种指针，`check-face` §12（`FACE_ONLY=row`）量中线，在 48 的旧代码上先红过（标签的字在 y 34，行中线 24）。 |
| **D16** | 四层底：板是纸，底是框 | 亮 `canvas #f4f4ef` · `side #ebebe5` · `card #fcfcfa` · `popover #fcfcfa`；暗 `canvas #0f0f0e` · `side #151513` · `card #191917` · `popover #242422`；终端 `--term-bg: var(--background)` | 亮 **`canvas #ebebe5` · `side #f4f4ef` · `card #ffffff` · `popover #ffffff`**；暗 **`canvas #242422` · `side #191917` · `card #0f0f0e` · `popover #2e2e2c`**；终端 `--term-bg: var(--card)`；暗色 `destructive` `#d4756b` → **`#e07f75`**；亮色 `read-ink` 0.90 → **0.88** | 2026-09-30，负责人：「用灰色背景，黑色内容怎么样」，又「大改，浅色也类似的去做。这是抽象的更改，要作用到整个 app 上」。病在终端：选中的标签是板的灰，下面的终端是底的黑，两者不成一体；浅色的底只比板浅 2.85 L\*，读不出是框。**一条规矩**：纸（`card`）是这个主题里最极端的那一档，所有内容落在它上面；框（`canvas`）是中间的灰，只有 R0、板四周的 8、窗口脚、设置页的底；左块（`side`）在两者之间；浮层在暗色里是最亮的一层。**数怎么来的**（OKLCH，色相 106.6，彩度 ≤ 0.008）：框离纸的那一步取 macOS 自己窗框与内容之间的一步（浅 6.6、深 9.5）——亮 7.10 L\*、暗 9.84 L\*；亮色的框落在旧 `side` 的值上，也是它能到的最灰（忙碌、空闲在上面都是 4.52，再深 0.005 OKLCH L 就跌破 4.5），暗色的框落在旧浮层的值上（每一支墨都在上面量过）；亮色的纸是纯白（L 1.0 上没有彩度可留，暖色留给框和左块），暗色的纸取旧 `canvas` `#0f0f0e`（暗色每一支墨都是对着它调的）。左块在纸与框之间：亮 离纸 3.93 / 离框 3.17，暗 4.40 / 5.44——亮色不能比框更深，框已经在六个词的地板上。浮层：亮色与纸同值（纸上靠 `float`，框上还亮 7.10）；暗色 OKLCH L .300，比框高 4.74、比纸高 14.58。**代价，量过**：暗色浮层上 `destructive` 从 4.85 跌到 4.25，于是只动 OKLCH 亮度（.666 → .700）提成 `#e07f75`，回到 4.84；浮层里选中那一行（`fill-strong`）上的 `secondary` 跌到 4.11，于是 ⌘K 结果、补全、斜杠命令、导入列表选中的那一行和列表选中行一样一律 `ink`（`app.css`）；亮色 `read-ink` 在白纸上读 13.10，调到 0.88 读 12.24，和暗色的 12.23 回到一对。六个词最差处亮 4.52 / 暗 4.67，`tertiary` 最低亮 2.79（框）/ 暗 3.37（浮层），在它的例外里。**已落地**：`tokens.css` 与 `tokens.json` 同一个 commit；`index.html` 首帧与 `src-tauri` 窗口底跟着改（`check-boot` 对）；板里原来画 `background` 的——终端（`TerminalPane` `--term-bg`、标题的底）、「一天」在终端上的淡出（`DetailPane` `--day-under`）、机器对话的输入框、用量表两端的淡出（`SideCut` 的默认）、模型带的渐淡（`lib/model-band`）、24 小时块的 `--bg`——一律改成 `card`；故意留下的是框：`[data-main]`、锁屏与锁屏前那一帧、状态栏、标签条两端的淡出、设置式「…」页（`MorePage`）。标签条上未打开的标签，脸的光晕缝改成框的颜色（`--halo-under`）。`check-palette` 新守：纸在一端、框离纸 ≥ 6、左块离两边 ≥ 3、浮层离框 ≥ 4；ink / secondary / destructive / ring 在四层上逐一量，六个词加上左块与浮层两层；「终端的底 = 纸」（盒子、交给 xterm 的颜色、板、打开的标签四样一色）；「浮层里选中的一行只有一种墨」。三条在旧代码上都红过（见《行为与门》）。`check-face` 4I 在暗色里另量浮层比框高 4。 |

## 字形

**已落地**。九个字形在 2026-09-25 重画了：`gear` `shield` `network` `pulse` `filter` `branch` `pin` `chat` `reload`。它们原来只是描边符号，没走家族的两条手艺（一块实心 + 遮罩切缝）。逐个的改动、量到的数和理由在 `components/Icon/README.md`；新的路径在 `components/bundle.js` 里，仓库里 `apps/gui/src/components/icons.tsx` 已经同步（`Mark` 加了 16 画布的遮罩参数 `CUT16`），两边路径数据逐字一致。

## 新增的 token

这份系统比 `tokens.css` 多出下面这些，都是今天已经在用、只是没有名字的东西：

- `brand-strong` `brand-ink` `on-brand` `on-destructive` `on-badge`——「落在某个实心上的字」以前是写死的 `#fff`，暗色主题下那是错的。
- `rail-on-ink`——竖栏打开态的墨。仓库里本来就有这个 token，这份系统补的是它的**用途说明和它受制于哪三道门**。
- `size` 一族 27 个：`pane-head` `rail-w` `row-1` `row-2` `tab-lip` `dialog-w` `narrow` 等等，把散在组件里的尺寸收回一处。
- `duration` `easing` 两族：`dur-120/180/240/420`、`ease-out` `ease-move`，原来在 `tokens.css` 里有值但没有用途说明。
- `radius-*` 六档具名。
- `segment`（2026-09-28）——分段控件选中那一段的面：亮 `#ffffff`、暗 `rgba(244,244,239,0.16)`。不用 `card`，因为暗色的 `card` 比落在 sheet 上的槽还暗。`tokens.css` 同一个 commit 加的，`check-boot` 对，`check-palette` 量「比槽亮 ≥ 4 L\*、上面的字 ≥ 4.5」。

## 没有搬过来的

- `--git-mark` 和 `--day-unwatched` 在 `tokens.css` 里是 `color-mix(...)`，这套 token 格式读不了函数嵌套，改写成了等价的 `rgba()` 字面值。
- `--cw-*`（进程表每一列的宽度）、`--part-*`、`--top-*`、`--tip-cmd-max` 这些只服务一张表的尺寸没有收进来：它们是那张表的实现细节，不是系统级的东西。要改就在 `tokens.css` 里改。
- `--seat-menu-inset` `--menu-inset` `--side-inset` 是从别的值算出来的，留在 `tokens.css`。

## 2026-09-30 撤掉的两样（没有 token 变）

D7–D10 那一行里的「sheet + R0 右端按钮」，这一天负责人撤了两半，都没动 `tokens.css`：

- **R0 右端按钮 → 一个动作一个家（D9 改写）**：会让新东西出现的动作只在 L0 的 ＋（两个以上落菜单，宽窄一样），R0 右端只放打开的那一样东西的动词。`PageActions` 删了；会话、机器、额度三个总览的 R0 右端是空的，网络页只剩 ×。
- **sheet → 一种弹窗（D10 改写）**：`DialogContent` 的 `sheet` 变体、`data-sheet`、角上的 × 都删了；表单是居中的对话框，`DialogContent form` 宽 32rem（就是 sheet 原来的 `max-w-lg`，写在 `ui/dialog.tsx` 里，`ui/` 在 `check-px` 的白名单里，所以没有新 token）、按在外面不关。`ui/sheet-form.tsx` 改名 `ui/form-grid.tsx`（`FormGrid`）。文件视图那张 `<pre>` 以前也叫 `data-sheet`，改叫 `data-file-text`，好让「`data-sheet` 为零」这道门量得准。
