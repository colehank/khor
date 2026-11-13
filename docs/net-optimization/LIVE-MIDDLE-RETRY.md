# 存活peer relay的需求恢复

当前隔离修复R `khor-net-live-middle-retry-v1` / experiment/net-live-middle-retry-v1，commit **9081b94dce827cbd3fb9b9e7bb13633d762e3cd9**，tree **cf9d172bb92778ccfd97899ddea27bdf0e6d2f6e**。它与实际已验证的dirty snapshot a3cad264bc6772d68289b5f954c0966dc87627cd的source tree逐字相同。基于产品I117c39，仅hop.rs是runtime差异；另外三个原physical fixture及两个unit测试文件。没有应用copy-wake消融，也未合入canonical/main或发布。

## 复现与修复

T3/sp7保留实际X→M Direct、M→Y真实Relay：暂停第二腿，等resident真正拒绝后做一次X目标失败读，并以五次X→M真实内容读维持首腿。恢复时先观察M实际目标内容，再在**同一总5秒**中做一次新的X目标读。same caller PID、raw hint保持、不预热X→Y、不重放写。原fixture三文件与修复前tree8ff930完全相同。

修复前064222Z-57c213de三个case前置全部成立，M在441358/337646/554797us恢复内容，X仍耗尽5秒，恢复后没有新的hop尝试。原失败退避从30秒起，按每candidate失败数加倍；hop_ahead与actual admission都用它过滤。此时X第一腿保持同一活连接、hints没有变化，失败缓存不会自动清除。

修复区分三种用途：背景Dial、周期Comparison、无目标路的有限Ask/Forced Recovery。只有Recovery且实际首腿weak handle仍可升级为开放连接时，允许在失败退避内重查；复用HOP_WITHIN=3秒作为actual ask的最短间隔。等待由请求持有，包含在原min(caller, now+3秒)预算内；到期/取消即停止。MISSED锁内按current epoch原子领取attempt，32并发需求只能领取一次。后台/比较和无活首腿的原指数退避保持；TRIES2、owner/提交门、暂停/信任/单跳/写重放边界不变。

这不是将3秒确认为最优参数：它是本次有界救通修复复用的原工作预算。活跃UI会归为Ask，真实1/3/5设备闲时/流量/手机能耗仍须按完整N3/N4验收；多middle排序也不由唯一middle绿覆盖。

## 原固定验证

- prep065900Z-8123d068：0/46.101216s，12files/source/hash/identity/owner及准确fmt patch核并应用。
- compile070105Z-8e82e611：0/281.033342s，17files/source/hash/identity/owner，whole hop模块**92/92、0failed/ignored**（113.32s），含真实NO_ROUTE→有限需求恢复、finite Pump不释放退避、25ms caller到期及32并发名额；旧取消/epoch/owner/保护流等原测试均过。原native verifier20unit/同producer test与CLI准入。
- runtime070832Z-6b87e77c：0/98.267049s，原sp7固定3fresh、替代0，前置3/3、恢复**3/3**，总恢复**1013894/784788/878995us**；实际两腿/目标marker、hint held、same PID、真实drop counters、原5s负向和所有owner清理全核。18files/source/control/hash/identity/原group空，ledger已闭。

修复前3红不删除/替换。行为比较有同一fixture、同预算、单一runtime文件改动；Debug数值只判此场景恢复，不是Release吞吐/长尾benchmark。实际CLI SHA856ecd02bf638c2b08908a625e5579aac732d4369866df36eca5a6da0d101121，test72a8b6536c60eefd4adede0aa48aa549b07b93bcde6d64ecd64ec8d4b8b5afd3。旧CLI/test只读保留在Turing closed-sole-cache-resume-v3。

原始协议E/live-middle-retry-plan-v1.json、physical-plan-v1.json；专属proof E/turing-20261113T070832Z-6b87e77c/live-middle-retry-physical-verification-v1.json。不是完整NET-16、N3性能、手机或能耗接受；I原Release high退化仍未解决。下一步回到copy-wake独立消融的原健康spike/短停顿/黑洞控制，再收敛Release共存退化；完整N0–N5范围不缩减。


## 同源Mac whole-hop与后续集成

2026-11-13，Mac43432815139在准确R9081b94d/treecf9d172bb92778ccfd97899ddea27bdf0e6d2f6e执行原Linux whole-hop相同92个名字，92通过/0失败/0忽略，105.52秒。普通Mac原生回归，不是性能对照。原50/40GiB guard与jobs2、源码Git bundle/control、14份artifact CRC/hash及逐条raw已核，E/public-mac-live-middle-whole-hop-v1/verification.json。原源refs仍属于ceeb请求，复用请求未删除。加上Linux92与原3次物理恢复，接受仍仅限本窄修复，不代表全部NET16、换网/能耗或最终N5。

为后续交付整理integrate/net-product-v2 / 3093f866，runtime与R逐字一致，另仅合入两份已远端46回归验证的Cargo dev边界工具修正；GUI/workflows保持冻结main c3b098。新候选仍有N3已知红且无最终整源检查，不晋升；I/R原树与所有旧失败保留。
