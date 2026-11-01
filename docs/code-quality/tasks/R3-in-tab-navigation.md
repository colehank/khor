# Session 标签内导航契约

用户最终确认：标签承载一段浏览过程，不与某个文件或提交永久绑定。在已有session内容标签里，提交、文件、网页、地址栏及网页弹出链接都在原标签跳转；只有显式New增加另一枚标签。首次进入、尚无标签时初始化一枚。独立工作区的多文件/提交预览模型不变。

实现：

- 统一ContentDestination/navigate，移除按内容类型决定是否新开标签的openFromPicker/openWorkspaceDocument两套入口。当前handle沿用，不按目标地址跨标签去重/切换。底层显式创建与跳转分开；正式UI的创建仅在New及首次初始化路径。
- 原point的乐观目标和串行写入继续使用；关闭后的旧回调返回null，不补建标签，导航层忽略空目标。Session入口链接和网页弹出目标也走同一navigate。
- 文件/提交返回仅浏览原标签的工作区导航，不把标签重置成新建页；跨类型切换不增标签。文件草稿继续按handle+device+target隔离；提交收起/阅读状态按目的地留到所属标签关闭。旧文件的行跳转提示不带到下一目标。
- 恢复/失效导航位置优先复用现有标签，不因旧位置多创建一枚。相同地址可以位于两个由用户明确新建的标签中，草稿独立。

证据 `/Volumes/disk0/Khor/evidence/round3/session-in-tab`：red记录原接口无法兑现统一导航；model-final覆盖快速跨类型跳转、New以外不增加标签、同地址独立标签、关闭后回调、行提示、重启/丢ack。原传输创建/关闭/重排、旧读取位置等回归保留；显式创建测试夹具使用命名清晰的createDocumentTab，不再把创建行为称为导航。unit/types通过。真实组件UI已通过六组，含commit→文件→browser→文件、草稿恢复、只New创建第二枚标签、同地址草稿隔离，以及独立工作区原单击预览/双击保留。最终verified/report.json六组全部通过（源hash前后一致），包括实际文件Back按钮保留标签身份。types-verified实际0；原model-final/unit亦实际0。

无Rust/原生构建；main/运行app未动。Root执行并自审，不使用子智能体。原Lab8946/port23836可刷新验收，暂无合并或ship授权。
