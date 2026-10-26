# Q039 pending导入完整性：主智能体独立审查

状态：源审无阻断；ed460746→9543b233进入集成，最终整合精确运行待。root非作者。

图谱旧built8d87/HEAD2211无新增符号，0 flow不是证明。实际逐读glue/store/wire、13Doc委托、完整6个业务测试和归并unit，并核pending-green-01/result及源SHA。真实红为warm cache后edit丢失、compact直接删唯一edit block；同次cold edit→dependency控制绿。

候选保留真实Loro ImportStatus success/pending的half-open范围，不当完整VV/op数。Doc必实现merge_report，普通merge允许pending；read_once注册pending并按最终VV检查，未解时不写新KEPT（原安全base保留），compaction明确Err且不写snapshot/删原块。wire answer/absorb的两处live真实导入登记report，原took_in/flush顺序保留。

root发现长寿命pending Vec逐消息extend可能重复增长，作者改为store内部每writer最高end的BTreeMap，公开Report保留真实范围。真实4096重复report unit证实只1writer并在依赖到达后compaction可恢复。所有13Doc只改委托，无假empty report实现；冷恢复仍接受合法pending，不标broken。

固定store6c906fe1、glue5f7fb8df、wire110ae0c7、integration097bb693；6窄+完整147lib+12transfer通过（147包含新1unit，不重复计），51.42s、46源稳定、最低外6,257,414,144B/内10,952,970,240B、3 own PGID已结束/scratch空。各原红相同内容断言转绿，cold最终resolved仍能cache。最终集成还有root当前inventory候选，作者绿不代替该组合绿。

限制：普通Doc.merge只丢报告，外部直接merge后compact仍需正确登记，后续strict transfer会集中在Loaded入口；旧wire pending还只驻RAM/flush已应用部分，未声明其已经持久或Applied。本批修的是读缓存/压缩对已有合法pending block的丢失及已登记live compaction，不关闭完整Q039。Node/Pair bounded transfer与真实25MB收敛、receipt/directory durability仍待。
