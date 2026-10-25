# Q-020 独立审查：自动保存版本链与丢弃生命周期

审查者：backend_audit（非作者）。日期：2026-10-25。
目标提交：`4d3c52f979714a9b7ec4e870ad8744c83d55c700`，父提交为本轮 main 基线。
源目录：`/Volumes/disk0/khor-code-quality-wave1`。仅审 `apps/gui/src/lib/save-road.ts` 与 `apps/gui/scripts/check-save-road.mjs`，按调用链读取 FileView/WorkspacePane；未修改生产代码。

## 结论

**建议保留并集成 Q-020。没有发现由此提交引入、应阻止合入的缺陷。** 已独立运行现有模块检查，24/24 通过；另用同一真实模块转译入口执行 3 个独立边界 probe，均通过。类型检查与真实编辑器流程仍由 root 的集成验收负责，本报告不以模块 fixture 代替它们。

发现一个与版本推进边界有关的 **P2 既有问题**：显式“用我的覆盖”刚读取的 digest 恰是本编辑线以前出现过的内容时，仍会被 forward 改成旧 head，再次报冲突。在目标提交和父提交都复现，故不是本次回归，建议另立任务，不把它混进当前修复。

## 高风险：未发现本次新增阻断项

### epoch 撤销与迟到成功

[save-road.ts:213](/Volumes/disk0/khor-code-quality-wave1/apps/gui/src/lib/save-road.ts:213) 仅在 job.epoch 等于当前 road.epoch 时更新 lineage、清 trouble/failed、重置 tries、确认已保存；[309](/Volumes/disk0/khor-code-quality-wave1/apps/gui/src/lib/save-road.ts:309) 的 forget 递增 epoch、清待写/重试/确认状态。catch 的校验在 fetchFileText 的 await 之后，因此取消落在错误核对期间也不会恢复旧失败或重试。

迟到成功仍向 anyLanded 和 r.heard 发布，并兑现旧 Promise，这是对“磁盘确已接受”的真实通知；它不等于撤回已写入字节。只有 orphan 和确认提示被 epoch 撤销。对应 [FileView.tsx:125](/Volumes/disk0/khor-code-quality-wave1/apps/gui/src/views/FileView.tsx:125) 缓存更新及 [720](/Volumes/disk0/khor-code-quality-wave1/apps/gui/src/views/FileView.tsx:720) 的监听：磁盘快照可以更新，新 buffer 的 pin 只有 by 匹配才推进。重新读取在 518 行取得 newLine，避免新编辑线把旧成功误当作自己的确认。

额外 probe：旧 asked 请求飞行中 discard/reopen，注册新 onLanded listener 后提交 fresh line。旧成功只公布一次旧 by 的磁盘事实，confirmed 保持 false；fresh line 下一请求的 pin 仍为自己读到的 A，并因旧 B 已实际落盘而正确 clash，没有被旧结果推进。

### 丢弃后新编辑与同文本重开

[save-road.ts:178](/Volumes/disk0/khor-code-quality-wave1/apps/gui/src/lib/save-road.ts:178) 在合并“与飞行请求同文本”前检查 epoch；旧时代的相同文本不会吸走新编辑的 waiter。旧 finally 仍清掉自己飞行槽并启动新队列，这是正确的串行收尾；它没有清空新一代 next。后续新任务失败仍能建立自己的 trouble 与 retry。

现有测试覆盖 discard/forget/discardAll 的飞行失败、已排定重试、取消发生于错误核对 await、迟到成功、重新编辑、同文本重开、新代正常重试和显式保存。没有把“请求已取消”错误等同于“磁盘未改”。

### 多编辑线与真实冲突

[save-road.ts:192](/Volumes/disk0/khor-code-quality-wave1/apps/gui/src/lib/save-road.ts:192) 只合并相同 by + epoch 的待写槽；其他编辑线保留自己的请求和 waiters。forward 只在本 line 已确认 bases 内取该 line 的 head，不跨 line 接受别人的 digest。底层 compare-and-write 仍决定能否写入。

额外 probe：三个独立编辑线都读 A 后写 B/C/D，只有 B 成功，另两条都 clash，磁盘保持 B，只有一个 landed，且不重试冲突。另一个 probe 将第一条写置为网络失败，让第二条独立编辑成功 C，再推进第一条的待写 D；D 正确 clash，C 未被覆盖，旧失败也未复活。

flushAll 挂到当前队列最后任务的 waiter。相同 line 的待写合并会转移 waiters，其他 line 保留；已有测试确认 flushAll 不在前两条完成时提前兑现。

## 中风险：既有显式覆盖场景仍会假冲突（非本次回归）

**位置：** [save-road.ts:150](/Volumes/disk0/khor-code-quality-wave1/apps/gui/src/lib/save-road.ts:150) 的 forward 与 [211](/Volumes/disk0/khor-code-quality-wave1/apps/gui/src/lib/save-road.ts:211) 的统一推进入口；调用端 [FileView.tsx:853](/Volumes/disk0/khor-code-quality-wave1/apps/gui/src/views/FileView.tsx:853) 的 mine 在 fetchFileText 后沿用原 by，将 freshly read digest 交给 write。

**可复现顺序：** 同一 line 将 A 保存为 B；外部把文件内容改回 A；用户对待保存 C 选择“用我的覆盖”，mine 已重新读取磁盘 A；write 仍把 A 当作本 line 的历史 base，改成 head B。磁盘实际为 A，CAS 被拒绝，错误核对看到 A≠B 又报 clash。用户已经明确选择覆盖，仍无法在这个内容回环中完成该动作。

**证据与分类：** 独立 probe 对目标提交和 `4d3c52f9^` 各跑一次，均输出 `{result: "clash", pinnedToOldHead: true, disk: "A"}`。这是现有“陈旧同线 pin”与“刚读取的显式覆盖 pin”未区分的边界，不是此次 head/bases 替换新引入的问题。自动保存对真实外部修改的拒绝仍正确，不能为了这个入口让所有 asked 保存绕过推进或冲突保护。

**后续建议：** 为“明确重新读取后覆盖”区分新 pin 的来源/编辑线语义，补实际 mine → write 的行为门；至少验证外部 A→B→A、两个独立编辑线、确认后到写入前又变动。该任务应单独处理，不在当前只读审查改代码。

## 低风险与验证边界

- “epoch 撤销 UI callbacks”应理解为撤销 orphan/confirmation/retry 等旧工作权限；onLanded/Promise 成功仍会送达。这是实现有意保留的磁盘事实，而非所有 listener 都沉默。交接和后续测试应使用这个准确契约。
- 当前新增测试适合证明模块时序：使用真实生产模块、内容稳定 hash、内存 CAS、受控响应和假时钟；不是重写一个保存算法再测它。它们未直接挂载 FileView 的 React state、onDraft 与 merge 界面，建议后续补一次真实编辑器的 discard→重开→旧成功/失败的交互回归。
- road 与 line/bases 的历史保留仍为进程生命周期；之前 moved 也有同类保留。本轮没有据此提出新内存阻断项。

## 运行证据与范围核对

- `node apps/gui/scripts/check-save-road.mjs`：exit 0，24/24 passed（本审查独立执行）。
- `/tmp/q020-independent-probes.mjs`：exit 0；3 个补充场景通过，另对既有显式覆盖问题做父/子提交对照。该临时脚本只在内存运行，没有写用户文件、访问节点或启动服务。
- 审查时当前文件 blob 与目标提交完全一致：save-road `a36d6f62eebb531b6331c985f594b0ddc48fd98d`；check-save-road `b2e433775f47f3052040cef7d042d3723ea28135`。
- review-changes 工作流先跑 get_minimal_context，再 detect_changes/get_affected_flows/tests_for/impact。此 worktree 图为空（0 nodes）；tests_for 未找到目标。随后才读取 diff、精确文件和调用方；图的零 risk/flow 不作为安全结论。
- 没有修改 production、测试脚本或公共任务账，没有提交；本代理仅新增此 review 文档。root 继续完成统一脚本登记、全 GUI 类型检查与集成验收。
