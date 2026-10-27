# Q037 R2：未知Native input的原子取消与晚到fence

状态：working，源码与fixture实现已授node_test_review；无native槽。作者树 `/Volumes/disk0/Khor/worktrees/current/khor-quality-agent-protocol`，基线 `8292e6543d26b323e99e7bc99e65ad2524a9bf12` clean。root独审/统一集成；main与用户App不动。R1当前31回归不可弱化，HTTP无能力继续Unknown。

## 已批准最小接口

一个 `BoundAction::CancelInput { message, parent }` Native RPC，在现有Store同一store.lock写事务中exact查authenticated caller＋agent＋message。Absent先持久fence才返回Fenced；已接受且仍为Entry.pending_message时，同事务按原request_cancel语义落取消intent。已换后续message返回Superseded，不改新turn。不能先只读lookup再普通Cancel，不List、不重发Send/Answer。

沿Ledger.messages，Dedup.task使用Some为原task、None为显式fence；不能用空字串表达状态。原64MiB预算、事务写失败明确失败，无新增store/TTL/GC。accept同写锁拒绝已fence的同input，不影响不同message。

小typed结果带home/caller/agent/message/parent绑定及必要task state/revision，不回整Record/history。parent按原firstSend归属核，不能错误要求已知Answer属于新parent。原生找回task先将同link写回，再更新receipt，保证重启能恢复；InputFenced/InputSuperseded不冒充ConfirmedCanceled/Completed。Snapshot全局字段保持按实际证据保守，不宣称HTTP或所有外部代理已停止。

## 写权与验证

允许a2a/src lib/store及既有测试，node/src delegation/remote_cancel/remote_cancel_tests/mcp/mcp_worker/link/proto与必要private测试模块/lib登记；a2a_http仅新增variant显式unsupported适配，不改HTTP恢复。Q054/GUI/shutdown不混入。先交源与exact测试入口，root独审后统一真实构建/红绿；没有源审代替运行。

真实验证至少：已accept但lostreply的原task/parent精确取消、Send仅一次无List/Answer；Absent fence持久后重启再释放晚Send无dispatch且新message可接受；同task新message superseded不误取消及错误caller/home/parent隔离；丢fence reply重试幂等、HTTP继续Unknown。

## source01首轮独审与修订

source01 11路径/2354输入候选（20dual历史快照）尚未native。root先graph minimal/detect（图仍c81，不能作为当前覆盖），再实际读a2a Store、Bound reply/schema、MCP发送前track、step/collect与fixture。小DTO、同Store写锁fence/lookup、绑定和新turn隔离暂未发现阻断；已确认两项须在source02收口：完成scan后collect未核source inventory，缺/丢input的receipt可能降回task-wide Cancel或漏计；original_parent首次发布read-missing→rename无锁，尾留tool与不同run同input可覆盖首parent，既有JSON也缺caller/opaque校验。owner已接修，要求有界source/receipt一致性、坏/丢状态保守未解及负面恢复父；短gate＋首次发布sync，不新store。两项未修齐、未实际回归前不授权native、不标R2 done。

测试分层保持明确：实际Store状态/CLI loopback IPC receipt、真实paired认证wire分别记录；setup直接accept且故意不提供task link不是实际观测到Send回包丢失，不能把组件证明写成全MCP/ACP端到端故障注入。旧31 body/旧paired parent主体不弱化。
