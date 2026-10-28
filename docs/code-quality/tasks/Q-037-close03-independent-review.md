# Q037 close-interface03 非作者独立源码审

Reviewer：node_transfer_owner，2026-10-27。角色是R1 close03非作者审查；未自审N1。只读源码、冻结payload和03相对02增量，未Cargo/native/test/改生产源/commit。本卡写权交回root。

**结论：本片未确认高/中/低级新增源码阻断。02已登记的容量预留缺口、completed-token独立oracle和六父默认隔离缺口，在03指定增量中均已源码闭合。可推进独立构建与实际逐父验证；这不是runtime绿、完整R1完成或整树合入结论。**

## 审查边界与证据

先读STATE/PLAN、既有[01/02独审卡](Q-037-runtime-close-review.md)。应用review-changes技能，minimal-context→detect_changes→affected_flows→tests_for→impact；CRG仍built f0104d73而HEAD8242b5bd，新can_admit_root无节点、affected_flows为0，不将旧图0结果当覆盖，转actual源。impact仅提醒agent路径风险，不据旧图扩大本片。

冻结外盘：`Q037/remote-r1/close-interface03-20261027T004746Z`；内盘：`/Users/zgh/Library/Caches/khor-code-quality/recovery/inflight-source/Q037-REMOTE-R1/close-interface03-20261027T004746Z`。**31files逐SHA外内一致，24source payload逐一等source-freeze.json，旧完整inputs2351保留。** author-from-close02.patch SHA **1d78eb90c09a5e30469a1ae5f6695ebd11f3284c53d19282f966d803a044d2ad**；它恰等冻结02→03三个指定文件的实际diff。

当前agent-protocol树HEAD8242b5bda383b456a0122fc2613ca4e274620187。本审3个关键actual源均逐SHA等冻结03：

| 文件 | SHA256 |
| --- | --- |
| crates/node/src/remote_cancel.rs | 36b9a102cca7fd12c451e2f719a733e74de2653d13a9bbf7a996709a645ea56c |
| crates/node/src/khor_agent.rs | 84af6fab7963dc74762826d4b352a49ac2c6a097c3b249aba169baf0bf41abfa |
| crates/node/src/khor_agent_runtime_close_tests.rs | 21ec8e3e70ca0f19c3285bb71135abe1f3062a755d78af0fae0c4a26ec00202a |

当前树另有root的scripts/run-node-acceptance.py及quality.yml CI WIP，**不宣称whole03=current**。冻结03包含rootcaller01的link/Exit payload；caller01及sourceentry独审由R1作者另办，本卡不审它们、不回填03旧frame，也不以新WIP覆盖旧原件。01/02已审冷Root NoCreate/closedfence/config-admission/Arcclaim/join锁外五项不重复扩scope。

## 03容量并集：确认源码闭合

remote_cancel.rs:1092的root_count对homes.keys/closed/closing取HashSet并集，按Path值计数，同Root在转换时重叠只算一份。can_admit_root允许已在任一集合的同Root复用，否则并集必须<256。实际生产新Home入口pump(:1337)、acquire(:1358)、request(:1614)，以及begin_close(:1414)都在同一scheduler mutex内使用该规则；没有找到遗留homes.len+closed.len的新Home入口。close.finish在该锁下移Home并插Closed，closing的RAII移除也在锁下；255Home+1冷Closing不会再让新Root进第257槽。

request对Status同样在未获Home时返回Unavailable/ResourceLimit（persist仍按原NotPersisted类型），不再走start_metadata生成未admitted Root的真实worker。state.pump(false)仍可回收/推进其它已有Root，未把不同Root正常进展改成全局禁止。原metadata/step closing配额与真实completion/join路径未被本片改写。

## 冷Root回归：确实经过public close预留窗口

cold_close_is_no_create_and_closed_roots_have_a_hard_quota先用真实Node::acquire_agent_runtime建立255个Home，test_defer_scan只把各真实Home的普通扫描时间延后，不清空/替换活Scheduler。第256个新冷Root通过真实public Node::close_agent_runtime，在begin_close已插Closing后、探实际claim前的cfg(test) producer_point暂停。

该窗口内并发执行另一新Root的实际pump、Status request和显式acquire，之后释放原close；要求预留时/竞争后/close完成三个实际union count均256，acquire与Status均准确ResourceLimit，所有冷Root不落目录。最后逐一真close255个Home，保原满额拒绝与已有Closed Root显式复用断言。此回归能够阻止02的cold reservation绕过；没有用假RPC结果、伪造计数或重置全局状态证明容量。gate4s/until3s过期会明确断言失败，未运行不能声称其时序已达。

## completed-token oracle：不再被旧executor claim遮蔽

running-token原幂等段保留，之后先等待该真实executor ordinary退出并close成功，再读取其真实Saved并确认ended_at_ms存在。completed retry复用同token/text/Saved，不合成terminal记录或RPC结果。

第二段在retry-record的独立新gate暂停真正record_command写者，前后分别调用实际executor close_claim并要求Held（matches表达式的临时File会在该语句结束drop），因此没有把probe本身持锁到close。旧executor已退，且retry gate期间executor claim仍可获得；这时public close必须Busy，才把屏障归于config admission而非运行中executor。释放gate后同run幂等结果和最终Closed均检查，两个retry gate及原executor gate都检查未超时。该新oracle源码能区分02曾被遮住的completed-token快路径；不是声称底层guard生产代码又有新问题。

## 六父隔离与仍待实际验收

六个新native父均显式ignored，且module为test+unix；中间四个父函数体逐字等02，只有ignore属性，其余原16/18不是本片改动。默认模块串跑/并发不再会执行cold256填表父污染其它测试；没有清活全局来绕过隔离。

root统一版本化专用入口仍须独立审并真正使用actual exe逐个新进程`--ignored --exact NAME`，六次各**1passed/0failed/0ignored**才计绿。list/默认ignore/0tests不计通过；无Unix支持必须诚实标未验证，不能冒native通过。本审不重复审root当前entry/CI WIP，不把只加ignore当入口已运行。

03尚无编译或六父runtime结果，FIFO/native owner/目录晚写/完成锁时序最终都待实际验证。原GUI late-write失败不能因本次源码审被抹掉；N1、N2、R2及完整Q037也不在本结论中闭合。本片没有发现需要扩大生产改造的确认问题，建议按冻结03指定边界推进后续真实验证。
