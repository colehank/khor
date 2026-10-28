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

## 后续片段进行中

root已将首片段集成到ef5f1143并跑8例/types；审查继续，不代表整轮结束。

E第二片段未提交diff已读：use-field-dismiss记录同一次主要pointer起点，保护内起拖外放；pointercancel、下一pointer、键盘detail0都清归属；外点动作后退出。ContentPicker搜索新增独立dismiss，Escape/外点清query；open/directory/source/close在动作前stopSearch，修正结果选择导致卸载时延后dismiss取消留下隐藏filter的风险。作者13例为hook/实际回调验证，真实拖选及PageField blur仍未验。暂无阻断，待精确提交和最终证据。

H后续v0契约审查：tail/older/newer＋含record内frame偏移cursor；256KiB读/64KiB记录/128帧/512KiB返回；GUI8页/512帧/2MiB先到上限淘汰并保重取游标。已提示小边界record只能在总读预算内受限重叠，超长行必须前进；跨页工具配对不能触发无界全文件扫描；refs亦计入缓存。append不因mtime reset，替换/截断身份独立处理。当前是接口方向审查，不冒实际纵向链已实现或通过。

E第二固定版577f888d80d0a5813f9d9417faefb9f740120150已核：现场干净，三文件SHA与field-dismiss-receipt.json一致。13例纯行为、既有field-interaction、8例刷新和types通过收据已读；初始harness失败单存，未冒业务失败。无阻断；root已集成79789276并验13例/types。R11仍缺PageField blur与实际拖选，不计整项done。

H首片段Node --lib cargo check已由root协调完成，cargo-check-result.json及retry日志核对exit0；先前“wrapper crate未编译”已被这次实际check覆盖，保留历史证据。不代表后续history_page未提交代码已编译。首轮setup错误保留，无新native操作。

## L03与history_page后续审查

E L03未提交diff初审无阻断：useDraftCount稳定Symbol owner，count更新和卸载cleanup分开，string Workspace reporter仍可共存；SessionContent/MachineFiles/SideWorkspace直接派生既有集合，FileBody仅无上游onDraft时报本地dirty，未复制正文。作者9例检查实际hook及生产调用表达式，不冒真实native退出/最后一键→即时Quit已验；等待固定提交核收据。

H transcript_page与core history_page/IPC/Tauri/gen根未提交diff初审：record内item游标能承接单record超128事件；core反向挑近期帧，预算break前最老已承接位置用作older，不静默丢掉后续前页。记录refs保留，来源64KiB约束作为serde to_value前分配上界；已提示core需单独覆盖序列化预算截断/续接，不以Node128测试替代。Node目前有界tail/older，newer、GUI缓存/渲染、refs按需等尚未交付；Node::open和vendor路径定位需纳入最终端到端首屏预算。当前独审未见阻断，不等于未编译新片段已通过。

L03固定版5c75f50dfc2fe8046c663b320c811a361eaa3f70独审无阻断。干净作者树、7源SHA与draft-count-receipt.json一致；9项hook/生产owner表达式及types通过收据。可集成派生计数接线checkpoint；没有实际React挂载/native Quit/最后一键即时Quit证据，上游onDraft被动回报间隙待真实App验证，不关闭L03。

H服务固定版2950ddb2d581860c4cef367e373b7cb514c49191独审无阻断。真实Node5项最终重跑含set_len、core3项wire预算/older续接/refs日志exit0；两测试记录diff SHA相同93c0dd30a4521a079abb5b3dcf130e527eec0d93ac4bd642e0594c6c41a59926。gen首次inventory extras拒绝保留，可逆备份后相同producer retry exit0；GUI tsc exit0。服务只tail/older，未完成newer/chunk/取消/ChatView实际消费；Tauri完整App未编译。

集成注意：HistoryPage/HistoryEntry/HistoryRecordRef由真实producer生成且属ignored产物，不在9文件提交stat；集成树需重跑producer或按精确哈希转移本次产物与inventory，不能手抄类型或用旧绑定冒types通过。root已收到这一提示。服务基础checkpoint不等于S1关闭。

证据补充：上述测试diff SHA只覆盖tracked diff，两个source.patch未包含当时untracked transcript_page.rs，不能将其称完整源码指纹。已通知作者在继续前保存固定2950ddb2新增文件快照/SHA及测试binarySHA，注明事后核定；root若在集成固定版本定向重跑，用新收据补齐来源，不回填原diff，也不双方重复测。代码无阻断结论不变，测试范围/源码来源限制须保留。

## 2950来源补充复核与纠正

posthoc-2950-source.json和两份全文已核对git show：transcript_page.rs SHA9983493db537492f2ca0bbebd42d19d9137a6f40921d6a5916f3953ba0365e64；core chat.rs SHAd291a565cc46b84daa1da70027287cfdb3562728f9a46a7af0876e8f85713293。binary明确仅事后观测，不冒原日志producer身份。

纠正前一节“final-source.patch未包含”判断：只查diff header的推断不准确。实际final capture在tracked diff之后raw追加新增模块全文，字节包含检查为true；缺的是unified路径标头，不是源内容。早期page-test-source确实不是补set_len后的字节，保持历史。已告root这一纠正，原日志不改，作者无需重复测试；root已安排的集成固定版定向验证继续。
