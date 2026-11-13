# 当前候选的重试与0-RTT边界审计

静态准确源I3 6f4c8c390ea44f068886dd95716dacd02c1eaa45/treec651b2fbfa2927f00cc1d2b679455d1c77dad9a6。E/request-replay-static-inventory-I3-v2.json包含完整74种Request变体、两段原函数与源SHA；额外逐行匹配验证没有漏掉tuple形状（该枚举当前无tuple变体）。这是静态审计，native_tests=0，不单凭此关闭完整N2。

`Request::is_reading`实际是“可安全重问”许可列表，共51种变体，其中ContentRead仅Cancel、Subscriptions仅Discover、Recognition仅Read等按action限制，不能把整个变体误当允许。`rides_early`单独许可28种，均在可重问集合内；默认拒绝新变体。

以下23种变体都不获自动重问或0-RTT许可：RecoverSession、NewShellRun、Pair、Act、Open、SatRemove、SatMkdir、SatTouch、SatRename、SatLaunch、SatWrite、OpenAgent、Remove、Mkdir、RenamePath、Launch、WritePath、CodeServe、CodeStop、Watch、Vouch、KhorAgent、Delegation。带action的其余变体还受上面的条件约束；这不是“只有23种操作会改变状态”的说法。

调用链是ask_on→ask_on_within：仅第一次exchange的Miss::Unreached且is_reading=true、原总期限未过时，最多再line_to+exchange一次，两次用同一个by；Miss::Late、Garbled与CallerExpired不走该分支。新Relay健康提示在exchange_inner内，没有增加应用层exchange或写重放。客户端ask_early先核rides_early及暂停/休眠/已有wire；服务端收到非许可early request时等待真实handshake成功，否则不dispatch。

OpenOnce/OpenAgentOnce/OpenShellOnce、DropFile/DropFileInto、DropStop、AddOverSsh及Offer等不属于普通只读，许可理由是去重token、取消幂等或重复记录合并；均未获0-RTT许可。其持久性、并发、崩溃窗口、业务副作用仍应逐family核对，不能由许可列表推定已证明。DocTransfer为CRDT/claim合同，Fetch会改变offer进度，亦不能混入普通读的“无副作用”结论。

历史已有Sessions严格两次/Mkdir严格一次、真实early Mkdir未完成握手不执行/完成后执行、Noq2删除非法ObservedAddr的红绿证据；最终准确源完整Quality仍在途，全部写family和包级、跨进程、取消/资源的必要范围见REQUIREMENTS。未新增分类镜像单测，也未放宽任何原caller、重试、信任或重放边界。
