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

## 新状态

- dae42cfe已push，Quality42764941090是长历史／箭头基线，等待实际红例；未发布或合并。
- 消息actions开始写：正式共享MessageActions使用已有RowMenu长按／右键，桌面hover/focus显复制icon，手机无常驻按钮；复制失败保留原位置重试。选择文本使用共享TaskOverlay＋原生可选文本。`选择文件`暂按常见的选择文本处理，已向用户说明，仍可纠正。
- 任意turn修改暂不能可靠实现：当前crates/acp control仅标准会话生命周期；apps/agent ACP没有锚点回退／truncate，GUI的HistoryEntry也没有vendorUUID映射。官方ACP schema没有anchored rewind；不显示虚假修改入口，不把追加新prompt冒充编辑。用户允许“不行就算了”。
- 原CopyText仍留Markdown代码块，不是整条消息常驻动作；ChatView用户/AgentContent/网络普通消息复用新动作。

## 实际验收清单

- `scene=gui&machines=3&history=long`：240轮结构化历史，停在最新，首屏自动补足；向上自动补页时不跳回最新；回到底部只见右下角下箭头玻璃，按下回到第240轮，抵达后消失。
- 追加 `short-pages=1`：4-entry服务分页，仍能自动填充，不靠点“加载更早内容”；后台缓存保持512entries／2MiB／8逻辑页，不因页变小而丢掉最新尾部。
- 桌面消息：闲置不显示常驻复制；hover和键盘focus显示icon；拒绝clipboard后原位可重试，成功才报已复制。
- 手机消息：正常拖动滚动不弹菜单，长按复制／选择文本；选择页沿用TaskOverlay，内容可由系统选择。浏览器触摸模拟不等于物理iPhone系统选择手柄验证。
- 代码块自身复制保持原有行为；图片仍由正式AgentResource显示，绝不把base64当正文。
- 普通Khor网络消息同样使用共享操作，终端／Khor／GUI的回到底部复用同一GlassCircle组件。
- 任意turn修改不做前端伪造；当前缺可靠host能力，需后续按agent的真实会话分支／回退接口接入再开放。

## 已看到红例

Quality42764941090（dae42cfe）：前端构建和GUI绿，完整App56既有案例绿。长历史两种小页场景都Timeout15000ms，尚不能在DOM重建第240轮；普通页两例已通过fill/read步骤，到玻璃data属性断言`true!==''`才失败（检查已修为hasAttribute）。红例原始日志chat-history-red-dae-app.log已保存，官方artifact正在校验下载。

修复：receiveHistoryPage只合并边界游标严格相等、各自有bodyentries的相邻页，合并结果仍<=128entries/256refs/512KiB；reference-only原替换行为保留。总8逻辑页/512entries/2MiB不变，容量满仍按原方向逐页淘汰并保留reload游标。预读每visit最多512wire读（与最低1-entry页填满现有entry预算一致），重复游标／失败仍停止，实际向上输入重置visit。

## 官方红例进一步定位（不能仅归因8页）

Artifact已校验564项CRC/size。小页diagnosis只有2次backward（tail+1次older），724/792px视口仅1个残缺agent条目，之后22次无新增的newer轮询。直接原因是useHistoryPrefetch中`const older=...&&canPrefetchOlder()`最后是boolean，seen键一直`older:true`，第一次补页后误当重复游标停止。新增historyPrefetchTarget保留真实opaque cursor，hook用direction+cursor去重；新增模型反例。前一轮pack和IO预算仍解决后续小页容量问题，不能说红例已走到8页。正常页红例读取锚点/自动补页已经工作；玻璃data属性测试错误已修。

## 原生游标边界核查

进一步核对apps/gui/core/src/chat.rs和crates/node/src/adaptor/transcript_page.rs：原生before/after游标可用不同record position表示相邻边界，不能只靠字符串相等合并。receiveHistoryPage新增可选requestedCursor，由单pending的historyReader传入当次真实游标；只在匹配当前window边界时允许新入页和既有边页合并，旧页间仍需要一致边界。新增不同opaque编码与stale请求模型反例。不解码前端游标。该补充当前dirty，等5daCI返回一并处理实际结果后再提交最终版。

## 已复现阅读位置回归并修复

5da/Quality42766916003：frontend、GUI units/types/static/4交互组绿；App在既有checkGuiExperience的切换标签后恢复锚点严格<3px检查失败，尚未运行新长历史案例。日志chat-history-5da-app.log，官方artifact transfer42853进行中。原因：原rememberReading保存匹配页的newer；合并页后这个newer可能距离当前条目很多个wirepages，小页恢复读回最后4entries，缺少原条目。缓存合并现在保留每个原始wirepage的checkpoint（仅游标与已有entrykey引用，数量<=512entry上限）；historyResumeCursor恢复精确原wirepage，ChatView复用。模型验证400entries全部100个原wire恢复边界及原生不同opaque编码。既有严格3px检查保留；harness失败会捕获诊断并继续其余App案例，不把单一失败掩盖其余结果。e9d4/Quality42767913845只含原生边界适配和本地化，目前运行；新checkpoint修复尚dirty。
