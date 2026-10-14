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

## 定 — Codex 终端适配边界

| 项 | 出处 |
|---|---|
| 默认 shared daemon 的 TUI PID → 当前线程仍无协议映射；设置 → 终端可显式配置原生 thread-id 标题，下一次启动后用标题和活写入锁关联；未配置或证据不足仍显示识别受限，不以 cwd/启动时间猜消息 | 10-14；本地 Codex 0.162.1 协议 schema；docs/HOOKS.md |
| tmux 原 session 的 mouse=off 时，Codex 启动会禁用鼠标捕获；khor grouped attach 的 mouse=on 必须在 Codex 进入 TUI 前生效。已运行客户端需用户退出/resume，不能靠查询节流补回 native mouse | 10-14；真实 Codex/tmux 隔离验证 |

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
