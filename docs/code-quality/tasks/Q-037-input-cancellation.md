# Q037 R2：未知Native input的原子取消与晚到fence

状态：已提交并集成；作者当前源码59父已验证，集成组合/App仍待。作者树 `/Volumes/disk0/Khor/worktrees/current/khor-quality-agent-protocol`，基线 `8292e6543d26b323e99e7bc99e65ad2524a9bf12` clean。root独审/统一集成；main与用户App不动。R1当前31回归不可弱化，HTTP无能力继续Unknown。

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

## candidate02复核与实际验证授权

02冻结 `Q037/remote-r2/input-cancel-candidate02-20261027T103735Z`，11源/2354输入、21dual由root逐file核；manifest5f6d6b71…44c3d、patchd3b756ad…b5e3。仅delegation/remote_cancel/new私测三路径相对01改变。collect现合并有界source/receipt inventory并共用identity校验，scan_complete后缺receipt不再漏计；坏/丢scope不允许task-wide Cancel且保原字节。original_parent短gate、bounded existing read+caller/opaque校验、首次file/dir sync，gate释放后再links.lock。新增CLI负面恢复父和真实双thread/坏index普通父，root已读，无新功能阻断。

已授node_test_review：先等价移入ec404 receipt Peer blocking两行并删未用参数，冻结03保留01/02；再唯一槽做当前normalCLI/Node/A2A delegation libtest/acp-stub/delegation_wire五必要build及新10＋R1旧31＋A2A旧ordinary14=预期55 distinct。actual inventory核数，helper不充父；原多summary/条件保持，worker/stub仅test child env，来源/5GiB/300/720/owned/短TMP/dual沿既有控制。当前仅授权，尚无这轮实际结果；产品失败需停报，不因source审通过标done。


## R2 run05同源59父完成与提交集成（root）

当前实现提交 `828fbd3167732c27fc4bb1718474a41961882279`，root cherry-pick至 `aa9f80caab8128f648c7e99e8222de3a76494985`。12源payload已核：非proto 11文件字节相同，proto新增测试块相同且保留N1已有增加；作者/集成树均clean。不含deps/locks/生成物/文档。尚未把作者树结果冒集成App绿。

source01/02两审查缺口已修；最后P1 sticky input-scoped修复让首次Unknown在receipt前持久source标记，normal Send迟到补task仍保exact input，坏/丢receipt不降级task-wide，links.lock Busy有限Deferred留cursor。新增负面父实际允许错误task Cancel到达peer并检查新turn状态，覆盖正常出版、Retry、坏/丢记录与锁重试，不能仅靠mock拒绝坏请求制造绿。

run05五个必要产物正常build/require-provenance全过：CLI65.530s、Node66.810s、A2A0.549s、stub0.445s、wire45.602s（A2A/stub复用同key有实际来源证明）。59 distinct=既定58+已开始P1父，70阶段全0，311.918628s。helper成功summary不另计业务父，SSH环境内外summary合为1父。完整2354输入before=after=current、94双盘文件逐SHA，root已读59个真实raw及result并实核288 owned groups消失、TMP无、cleanup/guard/forced空。最小外12238622720B/内8644415488B。槽已归还。

外盘证据 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q037/remote-r2/input-cancel-validation-run05`；内盘 `recovery/inflight-source/Q037-REMOTE-R2/input-cancel-validation-run05`。原run01 wire私有fixture编译失败0父；run02新9绿但wire自重执行缺hook清理红；run03新10+旧26绿后fixture首entry误读links.lock失败；run04只受控中断编译0父。均保留，不回填。用户收口约定：已开始做到既定验收，后续独立问题只记下一轮，不扩当前矩阵。HTTP无能力继续Unknown，全远端停止与真正App退出均未由本片证明。
