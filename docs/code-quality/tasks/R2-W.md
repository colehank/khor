# W 工作区正式接入

状态：queued，尚无实现作者/写权；接 G 作者槽后派发。2026-10-29。范围 R2-019/020/021/023，R022 材质由 G 单一实现。main/ship 未授权。

## 进入与交付顺序

1. 接手先核 STATE、实际集成 HEAD、A/G 共享文件和 native 槽。设计决定读 [WORKSPACE-LAYOUT](../WORKSPACE-LAYOUT.md) 第2、10–13节；最终参考 c501029490bf5997a271e252941582911d421a8f，只提取已验能力，不整支 cherry-pick Lab。设计系统与本轮 Clean Code/独审约束继承 [ROUND-2-EXECUTION](../ROUND-2-EXECUTION.md)。
2. 首片段先统一当前选中工作区与内容展示：取消选择右侧留空，保留标签/草稿/位置，重选恢复；已选无标签显示最近。完成真实宽窄四态、返回/重选与脏文件保留后固定提交。
3. 文件与 commit 使用同一预览槽：单击预览、双击固定、编辑自动固定，提交身份为完整 hash。左栏树/日志就地切换、正式列表布局和操作入口以参考为准。先记录状态归属，再改 UI；保留正式搜索/新建/上下文菜单。
4. 当前分支完整可达 Git 历史采用真实有界分页。先确定稳定游标、源变化/断线错误、批次/响应/缓存上限，再实现；覆盖合并历史、超过200条、分页并发与HEAD改变。Git 历史与聊天历史各自实现，不制造通用分页框架。
5. 实时更新接 E 的读取/组词/草稿接纳契约，覆盖外部保存/目录增删/branch/HEAD/重连。刷新入口取消以自动更新真实可用为前提。菜单只留一个滚动 owner；G 的 band/sticky 属性直接消费。

## 共享边界

派发时 root 指定唯一文件 owner；预计入口 WorkspacePane、App 工作区选择接线、FileTree、FileHistory/CommitPage、工作区 UI 模型、GUI/Node Git API。此卡不预授权修改全部这些路径。A2 当前持有 Rust/GUI API/生成器与多处 Session UI；native、生成器只由唯一槽串行。G 当前持有 app.css/scroll-edge/CommitPage 等，转交需固定提交/实际diff清点。

E已集成的 file-refresh 以编辑代次/请求序列/composition 拒迟到回包；merge 接纳保护、草稿 ledger owner 计数保留。FileView/CodeEditor先读 R2-E 对应契约；观察频率增加不等于可覆盖编辑缓冲。WorkspacePane当前草稿owner延续，不因隐藏/未选而卸载或清计数。真实Mac IME/退出验收仍由整合实例完成。

## 证据与收口

正式视觉验收按最终参考逐项核：无重复顶部标题；右上[会话 VS Code][更多]双玻璃、标签与文件标题无分割/间隔；commit标题正文、默认全部展开及手动状态按标签保留；SideList正式槽位、树/日志文字同起点、branch与摘要合行、头像1em与两段等间距；下载仅右键。文件编辑参考Lab使用textarea/内存API，正式CodeEditor阅读位置、composition/草稿接纳及编辑自动固定必须重新验真实组件。

evidence/round2/workspace 按片段保存源SHA、实际预算、原失败、定向检查、非作者审查及浏览器宽窄结果；每片段集成后只验受影响组合。独立commit标签阅读位置、收起状态、长文件跳转/唯一菜单滚动、透明主题、脏文件在切工作区/自动更新后不丢需有真实组件证据。原生/手机单列，不用Lab当正式交付。

新增独立问题入后续轮次，当前四条缺口继续修。每次交接/compact前更新本卡固定ref/diff、进程/端口、未完成和下一明确动作，源码/新增文件同步内盘恢复集。
