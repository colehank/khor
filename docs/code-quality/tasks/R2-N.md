# N 状态查询与文档同步并发

状态 queued，无实现作者/写权。R2-004既定范围：集群发布后hinton首次--all、turing settled --all各一次document transfer state busy；后续成功不抹原失败，不证明已修。首空闲作者槽再实施，root不加第三实现线。

## 已完成一次只读定位（2026-10-29）

main图谱对完整错误文本/这条调用链无有效覆盖，fallback定点查源码。仅证实同一字串来自doc_transfer_state::Error::busy，不能据字串判断实际争用者：

- node/doc_transfer_state/io.rs::gate 的进程内LOCAL.try_lock或实际gate文件try_lock会Busy；RootPermit/RootGate还有同线程锁序拒绝、admission等待/配额等独立原因。
- node/link_doc_transfer.rs::on_worker已有单重型WORK队列＋最多8个WAITING，WAITING满也Busy；State只在blocking step内存活，不能直接改成跨await持锁。
- doc_transfer_state::Context::apply devices遇needs_recovery也Busy。
- Node::open/open_async/open_hosting进入RootPermit，trust::recover_pairing_before_bootstrap在确需恢复时另开State；普通查询是否在此失败还是其同步RPC阶段，尚未用运行证据区分。
- 既有metadata_contention测试证明真实锁可产生typed Busy且释放可重开，仅此不证明原--all场景或修复。

证据对应集成d4232d11源；本次没有运行真实用户home/机器、没有并发fixture或修改代码。

## 派发时的最小行动

先在私有home和明确进程/锁归属的夹具，对一次只读查询＋后台同步取运行证据，保原Busy与当前阶段/锁持有身份和时间线。入口先区分constructor、metadata step、root admission、worker quota，阶段诊断保typed Busy/Storage差别且不输出敏感配置。调查预算为一个固定并发夹具和一轮对照，未复现就记录缺失证据/退出点，不无限放大到全同步栈。

证据证明根因后才选择最小修复：普通只读快照或已存在的有界队列/锁序契约；不盲目sleep重试、不吞Busy/把错误当空列表、不削弱文档事务/配对信任门。修复用原夹具验证并发成功、超载/取消仍如实失败、锁不跨await，以及状态查询原IDs不丢。root管理native槽；不触用户服务/kill真实agent。当前未能标完成。
