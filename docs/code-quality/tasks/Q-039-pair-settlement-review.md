# Q039 N2 settlement 第一小步

状态：source review，未编译/运行/提交/集成。实现 node_transfer_owner；root 已源码独审，无确认行为阻断。实现树 `fix/quality-node-doc-transfer@8f2f1d1b`，仅 `crates/node/src/trust.rs` dirty；candidate15 位于外盘 `Q039/node-pair-settlement-candidate-15` 及 recovery 同名内盘备份，8dual/1085 inputs，其他1084个输入逐字不变。

先前 Pair04 已证实29,224,082B回包失败、票据已消耗、同 caller 重试 badinvite，尚未修复。issuer 的 `settle_with` 在 burn 前就可能改 anchor/旧 key/roster，因此未来可恢复 intent 必须先于 settlement mutation；不能只在 burn 后补 claim。

第一步只抽取现有 verified roster→纯 SettlementPlan→即时 apply，旧 `settle_with` 实际调用。plan 持有已验证 roster 的确切 signed bytes/seq 和 public anchor 快照，Keep/Adopt 决策沿用 nt::settle；verify、anchor/mine/my-id读取和原 give_way_to/store_roster 的顺序均保留，无新锁或公共调用变化。新增3个回归源覆盖 owned snapshot、真实身份/拨号方向及坏签名/两个非lone网络拒绝。root 逐diff核过，但类型和实际行为仍待当前 Node 构建及原 trust 回归。

这不是 durable intent：没有 mine roster 预期指纹、共享 root fence、stale-plan 重验、幂等 replay 或重启恢复。apply 只允许原调用在同一 Loaded 下立即消费；源码注释已明确范围。大 Pair/Vouch wire 分块、可恢复 nonce/tokenHash/caller claim、bootstrap 前 reconcile 等仍待后续接口审查和实现，不得把这一步标 N2 done。未知 caller/被 leave/revoked 的权限不得因完成 claim replay 复活；第一请求仍须验证真实 token，hash 不是新凭据。

当前保持候选冻结；N1 owner 暂做 R1 driver 非作者审查，R1 之后再授 N2 必要类型与真实回归验证。无 main/ship 授权。
