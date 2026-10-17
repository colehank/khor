# 挂账 (LEDGER)

一行一项，带状态和出处。新挂账只往这里写；车道报告和简报在 `/Volumes/disk0/khor-redesign-home/audit/`（仓库外），这里引它们。

状态：**定** = 已裁定待派 · **问** = 等负责人定 · **机** = 等真机/人工 · **记** = 记下不急

## 派 — 车道在跑（10-13 起）

| 项 | 出处 | 车道 |
|---|---|---|

## 问 — 等负责人定

| 项 | 出处 |
|---|---|
| 树的目录点 6 px 还是 8 px（lab 看过 6，现在是 8） | audit/ws-lab.md；cd94a69a |
| `apps/site` 官网走 GitHub Pages、镜像留境内？ | apps/site/README.md |
| ship 走 CI：要推远端（本地 main 比 origin 多近 1900 个提交，推前扫敏感内容）、Mac 公证要 Developer ID + App 专用密码、iOS 要 App Store Connect API key；没有就按 docs/SHIP.md 手工 | 10-13 讨论 |
| 导入：ACP `session/list` 只问本机；没进程的 claude/codex 记录不给导入 | 10-12 讨论；ACP 可能取消，暂定 |

## 定 — 文件编辑器光标的高

| 项 | 出处 |
|---|---|
| **已定（10-16 负责人）：照 VS Code，等于行高**（`--text-mono--line-height`，13 号字 20；`editor.cursorHeight` 默认 0 = 行高）。原先是汉字墨迹的高（0.88 字号，13 号字约 11.4，10-13 定）。已改：`lib/code/theme` 的 `.cm-cursor::before` 以 CM 给的盒子为中心、高一个行高，盖住所在（视觉）行的行框；`npm run caret` ⑤ 守 | 10-16 caret-h |

## 定 — Codex 终端适配边界

| 项 | 出处 |
|---|---|
| 默认 shared daemon 的 TUI PID → 当前线程仍无协议映射；设置 → 终端可显式配置原生 thread-id 标题，下一次启动后用标题和活写入锁关联；已验证客户端的运行时标题契约按 PID/启动时间保留；未配置或证据不足回退普通 shell，不以 cwd/启动时间猜消息 | 10-14；本地 Codex 0.162.1 协议 schema；docs/HOOKS.md |
| tmux 原 session 的 mouse=off 时，Codex 启动会禁用鼠标捕获；khor grouped attach 的 mouse=on 必须在 Codex 进入 TUI 前生效。已运行客户端需用户退出/resume，不能靠查询节流补回 native mouse | 10-14；真实 Codex/tmux 隔离验证 |

## 定 — Claude hook 入口

| 项 | 出处 |
|---|---|
| 从 GUI 安装 Claude hook 写入 GUI executable 的 `state --hook`，但 GUI 入口只分流 `_console`，没有处理状态上报；需要独立补齐无窗口入口。用户已撤销 Claude hook，Codex 原生标题识别与其无关 | 10-14 hook 排查；`adaptor/claude.rs::hook_command`、GUI `run` |

## 机 — 等真机或人工

| 项 | 出处 |
|---|---|
| 连通新界面和文件默认源码：iOS 实机验收地图点选/relay 头像、矩阵反向、机器页连接详情滚动与文件预览切换 | 10-13 connectivity-ui；浏览器 1200/390/375 已验 |
| kb-caret 三件：键盘升起露光标、拼音候选条遮不遮、选区端点 | audit/kb-caret.md |
| Mac 输入法（拼音/五笔）在 CM6 里的组合输入 | audit/cm-b.md §4 |
| 手机发出的推送撑过锁屏宽限、超时写「发送中断：手机已挂起」，真机九条 | audit/p5-at.md「Phone checklist」 |
| 手机编辑文件大小上限（现在和桌面共用 64 MB） | audit/cm-b.md §3 |
| 文件编辑器的光标改由 CM6 画（`drawSelection`）：真 WKWebView 里已量到只剩 CM 那根（`caret-color: transparent` 藏住了系统光标），Mac app 上还要看光标高度、拼音组合时光标与候选条、选区的灰；iPhone 上 CM 画的两个手柄能不能拖、放大镜、候选条、主选区仍是系统的 | 10-13 cm-caret |
| iPhone 文件页点屏幕靠下的行：键盘升起后这一行要停在甲板上方（不在键或甲板下），升起过程不来回跳；点靠上的行不动；打字时一直在甲板上方；带拼音候选条再看一次（候选条算不算进 visualViewport）；键盘收起不跳。无头里只用替身 visualViewport 量过（`npm run file-face`），WebKit 自己的「露出焦点」和真动画只有真机有 | 10-14 键盘顶行 |
| 合并冲突里「保留 / 放弃」的两颗按钮，在**真 app 的窄窗口**上再看一眼：现在它们是不透明的 `popover` 面，压在删除行的文字上；无头里两个主题都量过（`npm run merge-face`），但「这两颗摆在这里读起来顺不顺、遮不遮住它压着的那行字」是布局判断，机器量不出——机器只守了底不半透、面等于 `--popover`、标签在面上 ≥ 4.5。真 app 里要看的是：长行伸到按钮底下的那几个文件、以及窄到 900 以下时按钮是否会盖住行尾的字 | 10-14 merge-face |
| 文件编辑器光标和行一样高（10-16 定）：Mac app 和 iPhone 上，浅深两个主题、字号 ×1 / ×1.5、界面字号放大各看一眼——那一竖上下正好到行顶行底，不压到上一行、下一行的选区灰和当前行的底色、`.cm-anchored` 的品牌底上；折行的每一视觉行、空行、行末带 commit 水印的行一样。10-13 负责人嫌过系统光标「行的 20 高过 13 号字」，这次是有意改回整行高，真机上看读起来是否还那样扎眼 | 10-16 caret-h；`npm run caret` ⑤ |
| 文件编辑器光标照 VS Code（2 宽、亮 500 灭 500、打字时常亮）在 Mac app 和 iPhone 上看一眼显不显眼、浅深两个主题、字号 ×1 / ×1.5；**「行末光标偶发消失」无头里没复现**（WebKit / Chrome、带不带水印、760 / 390 宽、1x / 2x，几百个行末位置都画着），真 app 里接着用，再碰到记下文件、是点的还是方向键到的、那一行有没有水印、窗口是不是透明材质；点行末 commit 水印光标到行末、之后打字落在行末；悬停水印仍出 commit 全文、iPhone 长按水印仍出卡片、轻点水印光标到行末不弹卡片 | 10-16 caret-vs；`npm run caret` |
| 选路修到根上（10-16/17 path-fix，vendor/iroh 补丁 17–22、vendor/noq-proto 补丁 3–5）装上之后，在 Mac ↔ turing 上看两小时的连接记录：①「本机网卡地址变化」不再每 40–60 秒一次（turing 的公网 IPv6 既是网卡地址又是 QAD 报的）；②「改走」从两小时 20 多次降到个位，同一条路不来回跳；③「连接收回」的「多久没听到」现在是 ms，不再有一边判死、另一边连进来还在说话的那条也被关掉；④ 打洞轮数（`khor doctor` 的 punch_rounds）比改前少；⑤ 打字时（负责人卡的正是这个）Tailscale 和局域网之间不再因为一个几秒的假 RTT 样本被挪走（noq 补丁 5）；⑥ Mac 忙的时候（编译、跑测试）也不来回改走：改走前两条路都要有此刻的样本（iroh 补丁 17 的 confirm_move），「不答」要 300 ms 什么也没收到（10-17 第三棒）；每轮打洞打通的新路不再当场把连接拉走，也要确认 2 秒（10-17 第四棒），所以打洞后同一秒里来回几次的「改走」应当没有了，打洞轮数在忙时仍可能比闲时多（上游 60 秒一查、RTT 过 10 ms 就打，不动）。手机上（iOS 默认不留备用直连，`KHOR_SPARE_ROAD`）看耗电和流量有没有变化：noq 补丁 4 让每个保活 PING 的 ACK 走它自己的路，中继路径排在直连后面时保活改为 15 秒（iroh 补丁 18），netlab 里量的数在 vendor/iroh/KHOR.md 补丁 18、vendor/noq-proto/KHOR.md 补丁 4；Mac 上拔一次网线或关 Wi-Fi，看是否不经中继就换到另一条直连、正在打的字是否停顿不到一秒；手机从 Wi-Fi 走到蜂窝（没有备用直连，只能落中继），看停顿是否仍在一两秒内（中继作备用时保活 15 秒，但它被选中时马上回到 5 秒） | 10-16 path-fix；netlab p1、p1j、p2、p2i、p3、p4 |

## 记 — 不急

| 项 | 出处 |
|---|---|
| 别的机器看死掉的 shell 没有原因和最后一屏（要加线上字段和远读 ring.bin） | audit/p5-shell.md |
| 托管的 tui 行也写 ring.bin，但只有 shell 行路由到死屏页 | audit/p5-shell.md |
| 「符号」列表不虚拟化、无快捷键 | audit/cm-d.md、cm-f.md |
| 提交页在手机上顶部那条「搜索全部」框不该留 | 10-12 lab 截图 |
| `check-face` 大文件遍历测试（>256 KiB 可编辑、截断句只在超上限） | audit/cm-c.md |
| 提交总量 2 MiB 上限在真实大提交上的节省没量过 | audit/cm-f.md |
| 窥视标题栏在手机上 44 高，可能要更密 | audit/cm-e.md |
| 远端 roads 仍是低频持久化快照，不等同即时双向探测；本轮已屏蔽失联上报者旧线路，独立保留反向观测。后续需要明确每条线路采样时间/刷新策略 | 10-13 连通讨论；dial.ts |
| NET-ROADMAP：接入、拓扑、共享、容器 | audit/NET-ROADMAP.md |
| 浏览器暂存的上传，草稿放弃后留在 `.khor/uploads`（拒绝时故意留着让人重发） | audit/p5-at.md |
| 新旧 khor 混跑没实测：老端轮里点名 `chat/<机器>`，新端答 unknown-doc，老端那一轮停在那 | audit/p5-at.md「Old khor」 |
| 推送的目标判定用一轮同步：目标多时发送前要等每台一轮 | audit/p5-at.md |
| `smoke.mjs` 走不通：第 4 步要的机器搜索框藏在 glass 头的搜索钮后面，p5 之前就停 | audit/p5-at.md |
| 终端 5.3 Windows ConPTY | audit/TERM-ROADMAP.md |
| 官网可交互演示（录制回放） | memory khor-site-sandbox |
| iroh-blobs 替换手写持有者+分片协议 | 10-12 讨论；持有者跑稳后评估 |
