# Q037 GUI close fixture02 独立源码审

Reviewer: remote_cancel_owner，2026-10-27。被审 GUI fixture 由 root 编写；本审不把自己的 Node close 实现算独审。仅读当前 source/fixture01原件/S22 delta，无源码修改、Cargo/native执行。CRG minimal/detect_changes先行，但图built f010未覆盖当前fixture，转实际source。

**结论：fixture02 在所审边界无新增阻断，原 GUI body/oracles 未弱化。真实cleanup绿尚待跑；旧四父业务过但14文件残留的cleanup红保留，不回填。**

## 确切源和双盘证据

当前 apps/gui/core/tests/khor_agent.rs SHA ef793938c9c62c47b95393a2e4163db685c7f7a6da5c5c012443d6881237e489 /17157B。前版fixture01 SHA f31d04469613bbb86843d45d00277fe3a93d83d273f2957d58fdd73c2128f822 /17129B原件未动。

新证据 root-gui-close-fixture02 含current source、delta-from-fixture01、delta-from-s22、review.json及本卡；外Khor/internal recovery逐文件SHA一致。01原三文件也独立核两盘，不覆盖旧说明。root将负责当前完整source aggregate；此审不拿旧03frame当当前。

## 关闭/进展边界

- Fixture::finish 调真实 Node::close_agent_runtime，只有 typed Closed 才进入 stop_server；Busy通过真实 node.reconcile_agent 推动已有退休意图/metadata，没有固定sleep冒成功。5ms只是等待下一次可观察状态。Err/超时panic均在removeRoot之前，所以保留原Root。
- fixture01 的8s仅Busy检查窗口，单次最后await可能越过8s；此注记已交root并修。fixture02唯一窄delta是 tokio::time::timeout(Duration8, async loop{real close; Busy real reconcile; sleep5ms})。timeout失败expect“root retained”，不会继续删Root。
- 不将 Tokio timeout说成能强行中断同步filesystem IO，也不说它取消已开始的spawn_blocking owner。掉close waiter后API原finite owner仍持claim/fence直到join；Root保留，Runtime drop可能继续等待其有限回收。scope是agent producers/cancellation owners，不是所有Node配置/文档IO shutdown。
- 成功后 stop_server 设置真实stop flag，轮询已持有HTTP JoinHandle::is_finished，最多3s；确认finished再join并检查panic。server既有recv_timeout100ms、慢分支2s未改。只有join成功再remove_dir_all/assert absent。
- HTTP timeout会消耗并drop该handle、返回Err并使父非绿，Root仍保留；这条失败路径不宣称已join，不将detached thread说成已回收。实际运行仍须外层PID/TMP gates。

## Drop与原oracle

- Drop始终尝试stop/join，但从不removeRoot。正常路径若没有显式删Root会断言失败；unwind中仅记录HTTP清理错误，不再panic，也跳过Root-absent断言，未见新增双panic链。finish里先close失败会由Drop止HTTP并保Root，不会走旧的先删Root/迟写重建。
- 对network/restart函数做机械逆适配（fixture mut/新增serving字段、结尾finish→原drop/join）后，两body与S22逐字相同。真实devices completed、原32s权限/context、原Stop/status/retry、restart interrupted/noPi等断言均保留；不是放宽oracle换绿。custom main/native worker entry与selector未改。

## 验证与交接

仅root原parser/diffcheck及本审独立source比较，无编译/native父/App结果。原14文件cleanup红与fixture01 retry-window说明都保留。建议root批准下一阶段：先必要current CLI/Node-libtest/GUI builds→六ignored close父逐exact独立进程→GUI两父，严格ownedgroups/privateexe/TMP gates；通过后才最小旧R0/16/stream/paired current-source回归。旧paired binary属于旧production，若验证当前serve acquire则应必要重编paired；stub源不变可按原SHA复用。具体scope/Cargo槽由root另授，reviewer不自启。

本卡交接root后reviewer不再修改；R1/R2/Q037及实际App/Exit仍open。
