# 挂账 (LEDGER)

一行一项，带状态和出处。新挂账只往这里写；车道报告和简报在 `/Volumes/disk0/khor-redesign-home/audit/`（仓库外），这里引它们。

状态：**定** = 已裁定待派 · **问** = 等负责人定 · **机** = 等真机/人工 · **记** = 记下不急

## 定 — 已裁定，待派

| 项 | 出处 | 备注 |
|---|---|---|
| Phase 5 只留广播，`@机器:路径` 直推，不在线就拒绝 | audit/STORE-BRIEF.md Phase 5 | 取消机器留言会话；机器页不留「发给这台的」；手机也支持打字写法 |
| Phase 5 行只戴最新一条的词 | 同上「the row's word」 | fate 按 (机器, digest)；非目标 offer 不待批；行上无角标、导航计数留 |
| Phase 5 死掉的 shell | 同上「a shell that died」 | 回滚环落盘；没环画命令账本；失败写明「退出码 N」/「宿主被结束」 |

## 问 — 等负责人定

| 项 | 出处 |
|---|---|
| 树的目录点 6 px 还是 8 px（lab 看过 6，现在是 8） | audit/ws-lab.md；cd94a69a |
| `apps/site` 官网走 GitHub Pages、镜像留境内？ | apps/site/README.md |
| 导入：ACP `session/list` 只问本机；没进程的 claude/codex 记录不给导入 | 10-12 讨论；ACP 可能取消，暂定 |

## 机 — 等真机或人工

| 项 | 出处 |
|---|---|
| kb-caret 三件：键盘升起露光标、拼音候选条遮不遮、选区端点 | audit/kb-caret.md |
| Mac 输入法（拼音/五笔）在 CM6 里的组合输入 | audit/cm-b.md §4 |
| 手机编辑文件大小上限（现在和桌面共用 64 MB） | audit/cm-b.md §3 |

## 记 — 不急

| 项 | 出处 |
|---|---|
| 「符号」列表不虚拟化、无快捷键 | audit/cm-d.md、cm-f.md |
| 提交页在手机上顶部那条「搜索全部」框不该留 | 10-12 lab 截图 |
| `check-face` 大文件遍历测试（>256 KiB 可编辑、截断句只在超上限） | audit/cm-c.md |
| 提交总量 2 MiB 上限在真实大提交上的节省没量过 | audit/cm-f.md |
| 窥视标题栏在手机上 44 高，可能要更密 | audit/cm-e.md |
| NET-ROADMAP：接入、拓扑、共享、容器 | audit/NET-ROADMAP.md |
| 终端 5.3 Windows ConPTY | audit/TERM-ROADMAP.md |
| 官网可交互演示（录制回放） | memory khor-site-sandbox |
| iroh-blobs 替换手写持有者+分片协议 | 10-12 讨论；持有者跑稳后评估 |
