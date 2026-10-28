# 下一轮独立审查

状态 working；owner round2_reviewer。只读源码与证据，不写产品；可写本卡，报告root。
基线e7c335c8。作者树 khor-round2-input 与 khor-round2-history；集成 khor-round2。
先独立核E/H失败链及接口/测试方法，作者首个片段后审diff。重点输入组词/迟到读取/外部版本归属、历史首次全量工作与单条大记录、取消与线程/资源、测试是否有效、Clean Code职责与命名。
图谱优先。禁止接管作者源码/自审，禁止启动App/native/Cargo、操作真实用户正文或会话。证据位于 /Volumes/disk0/Khor/evidence/round2。

## 2026-10-29 首次独立核查

已读STATE/执行计划/ENGINEERING，main e7c335c8图谱后定向核查；未改产品、测试，未启动App/native/Cargo。

- E：again仅发起时查dirty，回包直接setEdit；CodeEditor外部text直接dispatch，存在迟到读取/组词接纳候选。已建议作者覆盖编辑后撤销、读取乱序与卸载，按身份/代次/composition接纳，不能compositionend无条件回放旧text。onLanded保留当前edit.text，未据此认定覆盖。
- H：Claude/Codex transcript_since按len-start读到末尾，from0全量分配；巨型单行在无换行返回前已分配。已建议显式扫描/单条预算、游标进展与原文按需读取，跨页工具关联及等长替换分别处理/标缺口。
- 只读结果已报告root和两作者。尚无作者首个提交可审；下一步审实际接口与失败→修复证据，不据候选直接宣布用户故障根因。

## 首片段源码与证据审查

H固定提交8bb3c4596cdd9a7a0069ff002a2feed02a4230d1：未发现阻断。独立阅读read_window/records及Claude/Codex wrapper；扫描包含左前导字节且不超过256KiB，完整内联记录64KiB，before为exclusive并在0结束，fragment与oversized显式区分。读8项原语测试日志并核源SHA31c70191065804f6d5e867b9ddc9ec160cfb37e8736b87054400670991424bd6一致。legacy为同IO算法独立probe，不是链接旧provider执行。未编译wrapper所在crate，未接Node/IPC/UI、文件身份、跨页拼接/工具关联及大正文展开；不能计S1完成。

E待最终提交：当前四文件diff未见阻断。setEdit调度状态前同步invalidate，onText使用同入口；onLanded/merge/theirs亦经wrapper。最新layout cleanup绑定machine/path，使换身份/卸载在commit阶段使旧读无效。composition起止均废弃旧读，不保存待回放内容。回归从真实源码提取again、setEdit和onText控制回包，能覆盖读后编辑/撤销/乱序的接纳规则；DOM事件与真实挂载/卸载、真人IME尚未证明。compositionend到CodeMirror最终事务间隙、任意外部text替换与所有输入入口仍是E后续，不冒整包修复。作者正跑最终既定检查，等固定提交后核对。

本审查只读日志/源码，未亲自重跑测试或启动App/native/Cargo；本卡外无写入。

## 固定版本结论（2026-10-29）

E447419cf81a811a9bc7da446acb96b7eacdffd56与H8bb3c4596cdd9a7a0069ff002a2feed02a4230d1均无阻断，可作为首片段checkpoint顺序集成。两作者树核对干净。E最终8例与TypeScript通过日志已读；H8项std原语日志及源SHA已核。未发现需额外扩矩阵的新问题。root集成后仅跑受影响定向组合并更新精确HEAD。

这不关闭R2-027、R2-006或整包：E仍需任意外部text/组词最终事务、DOM/真实卸载和真人Mac IME；H仍需完整服务/协议/UI有界链路、wrapper crate检查、分页关联/原文展开/文件身份/取消和真实长历史体验。R2-028、路径输入/搜索退出、草稿退出亦未由本片段覆盖。
