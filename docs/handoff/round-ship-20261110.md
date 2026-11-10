# 本轮收口与候选发布（2026-11-10）

用户最新明确授权：`做完这一轮，直到ship完毕`。解除先前main/ship HOLD。根智能体串行；不新派智能体；本机继续source-only，所有编译／执行验证走GitHub托管CI。既有用户App/会话优先保留，原生验收先用隔离环境，不随意重启真实宿主。

## 目标和顺序

1. 集成main的App图标12d89226，保留本轮源码与规范；最新主机文案一起纳入。已合并为7ca9bd5f，仅两处文档冲突，双方新增内容均保留。
2. 整轮Quality(all/all)跑出明确结果，处理旧WebKit滚动／窗口缩放红项；检查跨平台文件重命名／移动、更新修复和原生UI承载边界。
3. 补齐原生通知覆盖浏览器/PDF的链路；利用可信CI/隔离原生探针验证实际可用路径。物理iPhone/不可访问硬件不伪称已测，有必要的硬件限制明确保留；不把浏览器模拟当真机。
4. 更新最终真实App Lab，包含新App图标资源与Khor主机/SSH主机文案；定向与整轮检查通过后合并main。
5. 沿用现有v0.2.0候选线，准备下一个不可变tag v0.2.0-rc.12（已核对rc.1–11存在、12未占用）。按docs/RELEASE.md由GitHub执行全Quality、四CLI/两MacApp/iOS归档、签名公证、TestFlight处理、更新清单签名和GitHub预发布；直到分发完成。当前没有创建tag或发布。
6. 稳定tag会额外提交App Store正式审核；本次已向用户明确沿用RC线，目标GitHub安装包/更新清单/TestFlight。成功RC不等同Apple外部Beta审核或实际安装。

## 已知完整验证缺口

- scoped60App+hot通过，不代表fullQuality；旧scroll-lifecycle WebKit reader-resize需复验。
- 原生通知在browser/PDF siblingview之上的承载未闭环，需要先查清现有menu_overlay/native-page-yield再决定最小修复。
- 文件操作已有3平台隔离Rust+SSH脚本证据与实际FileTree浏览器拖动；仍需原生集成边界验证。Air更新已有Linux/macOS/Windows及真实WebKit自动化证据，未在Air真实安装；本轮发布后可检查更新可见性，不虚构安装。
- 任意用户历史消息编辑+重新生成，继续按用户“不行就算了”明确延期，不阻塞本次发布。

## 当前状态

分支integrate/settings-gui-20261109（原HEAD5eb971f5，已合并main为7ca9bd5f）。源码main的未跟踪lab-preview保留。旧Lab23950仍dc946c83/PID6019/exec99444，后续替换仅核对过归属的静态服务并保留旧版本资产。

后续精确SHA/run/tag/receipt持续写入本文件及外部evidence状态，compact从本节和实际git状态继续，不重走旧HOLD。


## 首次全量回归

main图标同步后effe2793/Quality42779150845前端在catalog生成时红：最新SSH主机文案和已有gui.add-ssh-host同文，目录禁止两个key同文本。已改为复用原有add-khor-host/add-ssh-host，删除新增choice别名，App菜单/弹窗标题共用同一词，Khor统一大小写。没有绕过去重门。原生通知确认BrowserPane子WebView不能由DOM盖住，正在复用既有menu_overlay可信浮层通道，实际通知状态/React事件仍由主界面拥有，不造第二套业务或外部系统通知。
