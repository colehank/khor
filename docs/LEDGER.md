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

## 机 — 等真机或人工

| 项 | 出处 |
|---|---|
| kb-caret 三件：键盘升起露光标、拼音候选条遮不遮、选区端点 | audit/kb-caret.md |
| Mac 输入法（拼音/五笔）在 CM6 里的组合输入 | audit/cm-b.md §4 |
| 手机发出的推送撑过锁屏宽限、超时写「发送中断：手机已挂起」，真机九条 | audit/p5-at.md「Phone checklist」 |
| 手机编辑文件大小上限（现在和桌面共用 64 MB） | audit/cm-b.md §3 |

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
| NET-ROADMAP：接入、拓扑、共享、容器 | audit/NET-ROADMAP.md |
| 浏览器暂存的上传，草稿放弃后留在 `.khor/uploads`（拒绝时故意留着让人重发） | audit/p5-at.md |
| 新旧 khor 混跑没实测：老端轮里点名 `chat/<机器>`，新端答 unknown-doc，老端那一轮停在那 | audit/p5-at.md「Old khor」 |
| 推送的目标判定用一轮同步：目标多时发送前要等每台一轮 | audit/p5-at.md |
| `smoke.mjs` 走不通：第 4 步要的机器搜索框藏在 glass 头的搜索钮后面，p5 之前就停 | audit/p5-at.md |
| 终端 5.3 Windows ConPTY | audit/TERM-ROADMAP.md |
| 官网可交互演示（录制回放） | memory khor-site-sandbox |
| iroh-blobs 替换手写持有者+分片协议 | 10-12 讨论；持有者跑稳后评估 |
