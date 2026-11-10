# 长历史与消息交互（2026-11-10）

用户在通知精简处理中追加，原通知改动保留；当前串行进行，不派智能体，不发布。本机source-only；构建／执行仍走公开托管CI。

## 本次要求

1. Lab加入很长的结构化对话历史，验证此前GUI会话初始加载过短、自动向上补页与阅读锚点问题。
2. 回到底部统一成右下角仅下箭头的玻璃按钮。
3. 移除消息旁常驻“复制／带回输入”。手机长按菜单；桌面悬停出现复制与修改图标。
4. 检查任意历史turn编辑能力，不支持时可以不实现，但不能把追加新消息冒充编辑。
5. 正在等异步澄清：“选择文件”是否是选择文本；编辑是否只指用户消息并从那里重新生成。

## 计划

先定位共享历史渲染／滚动及消息动作，复用正式组件制作长历史数据；检查对应agent/ACP能力；完成前端明确项，覆盖真实App Lab宽窄滚动／操作回归。保留通知新改动，最后更新同一Lab供用户验收。必要能力缺口如实列出。

## 通知并行状态（同一根线程串行，不是另一智能体）

当前HEAD13bd01ecf65d02b1deced0ea6370280ffa32a8ac，tree d89c8ad77a762af378011096b7c1d583f2826bef，分支integrate/settings-gui-20261109。Quality42760880544前端/GUI已绿，完整App仍在跑。通知已去掉状态词，桌面hover/focus才显X，手机无X；这轮已有代码、检查，保留。

LIVE23950仍是0286，ownedPID57328/exec92502。新GUI证据notification-clean-13b-gui已下载，6组results待核对。高效可信下载helper /Volumes/disk0/Khor/evidence/more-network-gui-20261109/fetch_review_cached.py --run RUN [--name ARTIFACT] --previous PREVIOUS_ROOT --destination NEW_ROOT；从官方ZIP目录校验每项CRC/size，复用已验证文件，只下载变化项，再核对manifest/source/tree。旧包基线notification-demo-028-transfer/extracted-fast。发布HOLD，实际iPhone/原生浏览器叠层等验证仍未完成。

## 已写入（尚未验证）

- 新components/ScrollToBottom.tsx：GlassCircle+已有IconArrow down，右下角，保留既有data选择器／fresh辅助语义；ChatView、MachineChat、TerminalPane三处复用。Terminal的fold-lift CSS未改，保留8px原间距。
- dev/lab/long-history.ts：240轮，每轮20流式片段；每12轮包含思考/工具/表格/代码/链接，每40轮图片。fixture options.longHistory在第0GUI会话使用整个长回放；bootstrap query history=long并进入identity。
- 即将补实际App long-history/short-pages检查。现有useHistoryPrefetch visit最多8wirepages；HistoryWindow也最多8wirepages/512entries/2MiB。在4-entry小页+流式碎片下，很可能只有1轮可见就触顶，需要先测红，不能靠抬高整体内存限制解决。可能方案：合并相邻可连接小wire页到128entry/512KiB逻辑页，再保留8逻辑页与512entry总界限；需先证据再实现。
- 待做消息操作组件。AgentContent内部、ChatView用户泡泡和MachineChat普通消息需要共用；旧CopyText在AgentContent和ChatView，BringBackToInput在ChatView1767；Markdown代码块自身CopyText是独立代码复制，用户说不要常驻复制应只针对整条消息，保留代码块语义除非后续指示。

## 通知交付更新

13bd所有验证已绿且LIVE23950，serverPID33675/exec66857；通知无状态词、桌面hover/focus X、手机无X截图已看。来源notification-clean-13b-review/developer-lab，tree d89c8ad77a762af378011096b7c1d583f2826bef。旧0286与98a等快照保留。无需重新为13bd跑检查；后续新chat代码再跑。
