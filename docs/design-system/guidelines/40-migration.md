# 从 tokens.css 迁移

这份系统写的是 D1–D10 之后要做成的样子。下表是它与仓库里 `apps/gui/src/styles/tokens.css` **今天的值**的全部差异。改的时候，这份系统和 `tokens.css` 在同一个 commit 里改完，`check-boot` 负责比对。

| # | token | tokens.css 今天 | 这份系统 | 为什么 |
|---|---|---|---|---|
| D1 | `body` 字号 | `--text-base: 14px` | **13px** | 跟 macOS 对齐。`read` 保持 16，长读的地方不跟着降。 |
| D6 | 选中行 | `--accent`（16% 的淡底）+ `ink` | **`brand-strong` 实心 + `on-brand`** | 原生 app 的选中行是实心强调色。淡底那一版在 `side` 上让副标题只剩 4.40。 |
| — | 主操作 | `--primary: #1a1a14`（墨） | **`brand-strong`** | 当初把主操作改成墨，是因为 `#93b04d` 对白只有 2.45。`brand-strong` 对白 4.88，岗位可以回到橄榄绿。 |
| — | 焦点环 | `--ring: #1a1a14`（墨） | **`ring: #5d7b06 / #99bb36`** | 同上。在 `card` 上亮 4.75 / 暗 7.98，都过 3:1。 |
| — | 控件高度 | `--ctl-sm/md/lg: 24 / 30 / 40` | **22 / 28 / 32** | 对齐 macOS 的控件高度；40 只在指针是手指时由 `touch` 44 接管。 |
| — | 圆角 | `--radius: 10px` 一档 + calc 派生 | **六档具名：4 / 6 / 8 / 10 / 12 / full** | 一档基数 + `calc` 的派生法让「按钮用几」这种问题没有答案。 |
| — | 设置组间距 | `--gap-group: 18px` | **20px** | 让 4 的倍数不留例外。 |
| — | R0/L0 高度 | `--pane-head: calc(44 + 4) = 48` | **`pane-head: 48px` 直写** | 值一样，但它是一个设计决定，不该是两个别的数算出来的副产品。 |
| D2 | 没有标签的页 R0 | 各页各拼 | **后退/前进 + `title-sm` 标题** | 照「系统设置」。 |
| D3 | L2 列表空白 | 能拖窗口 | **不能拖** | Finder 和系统设置都不能；竖栏空白照旧能拖。 |
| D4 | 左块底 | `--side` 实色 | **macOS 用 `sidebar` 材质**，其余平台 `side` | 半透明透出桌面，是「随系统装的 app」最明显的一处。 |
| D7–D10 | 页面结构 | 六种各自拼 | **三种页型 + sheet + R0 右端按钮** | 见《骨架与三种页型》。 |
| **D11** | 竖栏打开态 | `--rail-on-ink: #303a19 / #dce9a6`，彩度只用掉该亮度上限的 64% | **`#2d3e00` / `#c6f560`**，同色相取最大彩度（+59% / +107%） | **已落地**。`--rail-ink` 一个字没动——`check-rail` 的 `LINE_FLOOR`/`FLOOR` 把静止态钉死了。组件早就在读 `var(--rail-on-ink)`（`App.tsx:363/1430`），所以**只改 `tokens.css` 两个值**，不动 `app.css`。 |
| **D12** | `data-body` 的作用域 | `[data-rail-item] svg [data-body]`——只在竖栏里生效 | **`svg [data-body]` 普遍生效** | **已落地**（`app.css` 一行）。实心块是家族的**密度**规则，不是状态标记；状态标记是叠在它上面的 `fill-opacity: 1`，那一条仍然只在竖栏里。不放宽的话，重画后带实心块的 `gear` `shield` `network` `pulse`（出现在「…」菜单里）会是空心的。 |
| **D13** | 品牌绿家族 | `--lq1/2/3`、`--accent` 的现值 | **整族彩度 ×1.28**：`brand` #93b04d → **#8fb31e**，`brand-strong` #5f7a1f → **#5d7b06**，`lq1` #d5e595 → **#d4e77d**，`lq2` #bcd47b → **#b9d75e**（暗色同理） | 在 OKLCH 里锁死亮度和色相，只动彩度，所以**所有对比度一个没变**（白在 brand-strong 上 4.89 → 4.88）。原来那层灰扒掉之后，选中行才像个强调色而不是军绿。**已落地**，`tokens.css` 八个值 + `crates/core/src/avatar.rs` 二十八处（含那段用 hex 写的选色推演）。按 `pick()` 复算过：ground 仍是 `liquid[2]`、first 仍是 `#3f4482`，**选色顺序完全没变**，只是换了写法。 |

## 字形

**已落地**。九个字形在 2026-09-25 重画了：`gear` `shield` `network` `pulse` `filter` `branch` `pin` `chat` `reload`。它们原来只是描边符号，没走家族的两条手艺（一块实心 + 遮罩切缝）。逐个的改动、量到的数和理由在 `components/Icon/README.md`；新的路径在 `components/bundle.js` 里，仓库里 `apps/gui/src/components/icons.tsx` 已经同步（`Mark` 加了 16 画布的遮罩参数 `CUT16`），两边路径数据逐字一致。

## 新增的 token

这份系统比 `tokens.css` 多出下面这些，都是今天已经在用、只是没有名字的东西：

- `brand-strong` `brand-ink` `on-brand` `on-destructive` `on-badge`——「落在某个实心上的字」以前是写死的 `#fff`，暗色主题下那是错的。
- `rail-on-ink`——竖栏打开态的墨。仓库里本来就有这个 token，这份系统补的是它的**用途说明和它受制于哪三道门**。
- `size` 一族 27 个：`pane-head` `rail-w` `row-1` `row-2` `tab-lip` `dialog-w` `narrow` 等等，把散在组件里的尺寸收回一处。
- `duration` `easing` 两族：`dur-120/180/240/420`、`ease-out` `ease-move`，原来在 `tokens.css` 里有值但没有用途说明。
- `radius-*` 六档具名。

## 没有搬过来的

- `--git-mark` 和 `--day-unwatched` 在 `tokens.css` 里是 `color-mix(...)`，这套 token 格式读不了函数嵌套，改写成了等价的 `rgba()` 字面值。
- `--cw-*`（进程表每一列的宽度）、`--part-*`、`--top-*`、`--tip-cmd-max` 这些只服务一张表的尺寸没有收进来：它们是那张表的实现细节，不是系统级的东西。要改就在 `tokens.css` 里改。
- `--seat-menu-inset` `--menu-inset` `--side-inset` 是从别的值算出来的，留在 `tokens.css`。
