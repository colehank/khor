# Q037 root product callers / acceptance entry 独立审查

Reviewer: remote_cancel_owner，2026-10-27。被审四源由 root 编写；本卡不把 reviewer 自己的 khor_agent/remote_cancel 实现算独审。仅静态源码、锁定依赖实际 API 与有限 fake-executable 控制自测，无 Cargo、native 业务测试、真实 App Exit 验收或源码修改。卡交接 root 后 reviewer 不再改写。

**结论：最终审读本中未发现本范围剩余阻断。旧入口的可观察 setsid 组漏门及 CI 缺 Pi dependencies 已有明确证据，root 修订后控制复验符合预期。产品 caller 仅静态通过，仍需真实构建/GUI与 Exit 验收。R1/R2/Q037 均不因此关闭。**

## 审读来源和确切字节

按 review-changes 技能先调用 CRG minimal/detect_changes；图建于 f010、current8242 不匹配，且未覆盖新 entry/Exit，转实际 source。无 .codegraph，不索引。

| 文件 | 最终 SHA256 | 范围 |
| --- | --- | --- |
| scripts/run-node-acceptance.py | 035a049b8c379f5ec5e54a16435bb7a6853902ab1f6c23a1731d94813e05ba7c | 独立 native parent 调度/控制 |
| .github/workflows/quality.yml | 7bc4573b7be775e55261106c155ecca8c5f5f09b210c8ad154c1ad143918a8bb | node-native CI job |
| crates/node/src/link.rs | ca6690c73874f60fe4fd065bb8b446e95c2a3e7a5ca72cb5af1595d2903588fe | root 的 serve_as 单处 acquire |
| apps/gui/src-tauri/src/lib.rs | 79be74f98a7871a67f136d22f88da8cd4dcb69508a8ef3f427c14ab06f3f20d7 | root 捕获与真实 Exit typed close |

最终四源副本存 `root-callers-entry-review02-20261027T010059Z/final-reviewed-source`。源/入口/CI 尚待 root 最终聚合完整 source frame；本审不复抄03历史2351 frame当当前状态。

## 产品 caller：静态通过，执行待验

- link.rs serve_as 在原 bind_ears 前调用 Self::acquire_agent_runtime(self.root())?；serve 与 serve_resumable 共同经过此处，没有把普通 Node::open、Read 或 ticker 变成显式 reopen。真实新 serve 的进入权限清楚；不假称整个 Node 关闭。
- Tauri run 捕获一次 Node::root_from_env，clone 给真实 resident thread，Exit 用同一 Root。RunEvent::Exit 直接调用 close_agent_runtime(root)，不为冷 Root 调 Node::open。Closed/Busy/Err 分支真实 match；Busy/error 仍保持原退出行为和已有持久状态，不宣称远端停止或 active producer 已完成。
- Cargo.lock 锁定 tauri2.11.5；实际本机 registry `tauri-2.11.5/src/async_runtime.rs:272` 确有 pub fn block_on<F: Future>(task:F)->F::Output，内部 runtime.block_on(task)。run 从普通 main / native event callback 进入，并未新加 async runtime 嵌套入口。这里只确认实际 API 与源码接线，不冒编译或真实 Quit 已过。

## 两个已修 findings

### 原高风险控制漏门：只查初始 PGID 可报告成功而留已可观察 setsid owner

旧 entry bc278ba516185c69e0b2967481167bac61c20db132b29adc409aa6d7a119e48a 的 run:50-92 只 wait/kill/check Popen 初始组。有限 fake 反例真实派生 parent4498/group4498 -> setsid child4499/group4499，并保持父0.4秒供观察。独立25ms sampler18条包含 child PPID4498 和 start Tue Oct27 08:56:41；父退后 child PPID1仍活。旧 entry 却 exit0、fake passes6、case1 forced_cleanup=false/group_absent=true。只表示控制器漏门，**不是六个 native 业务父通过**。

原 raw、case receipt、PID/start/PPID/PGID samples 存 `root-callers-entry-review-20261027T005617Z`。Reviewer 仅 TERM 自己带唯一 tag 的4499组，随后 ps absent；未操作用户或其它任务进程。

root 的 f695c4e59b3ec61588cffffe2141ffd4b73db9a71b66733dbad2a192eecce68f 版已加25ms ancestry、最多512 PID/start 的 ownership、观察过的 sessionleader 组全成员 drain、leader 身份不匹配不 signal、实际 private libtest/CLI footprint gate。复跑同反例 parent7256/group7256 -> setsid7258/group7258，start09:01:04，独立 sampler8条确认父活时从属。entry exit1/passes0，forced_cleanup=true，remaining_owners[]、private footprint[]、原组和7258均 absent。随后最终035a版唯一 delta 是提取并提前 check_space；控制逻辑未回退。

边界保留：采样不承诺发现采样间瞬时脱离的所有未知任意 exec；只 signal 可追溯 owned groups，不按共享路径宽杀。真实六父仍须各自原生 worker ESRCH oracle。若未知 private executable 尚活，只能非绿保留报告。

### 原 CI producer 缺项：只安装 GUI dependencies，真实 host 可能因缺 Pi SDK 早败

旧 node-native job 未安装 apps/agent dependencies，独立 agent job 的 node_modules 不会传到此 job。未设 KHOR_AGENT_BUNDLE/KHOR_AGENT_SIDECAR 的 debug sidecar 实际 fallback 是 `node apps/agent/src/host.ts`，其顶层导入 @earendil-works/pi-*；单独 npm ci apps/gui 不能满足 Node 从 apps/agent 的解析。这样的 failed terminal 不能冒真实 Pi 请求覆盖。

最终 CI 已在同 job 加 npm ci，working-directory apps/agent，且保 setup-node22.19、GUI build、实际CLI build、actual libtest artifact JSON。当前 fallback 消费 TS host，**不需要为本片自发 Bun build**；若未来消费编译sidecar，必须安装 Bun 并显式指定/记录实际产物，不能仅 build 不消费。实际 CI/host bootstrap 本审未跑。

## Entry 注册、精确执行与来源边界

- SUITES runtime-close 前缀 khor_agent::runtime_close_tests:: /6，匹配当前新六个明确 ignored 父；doc-transfer 前缀 link::documents::flow_tests:: /5，匹配 N1树 link_doc_transfer.rs 挂载的五个明确 ignored flow 父。后者尚未集成到当前 agent tree，当前 CI 只声明 runtime-close；不能据 registry 称 doc5 已运行。
- 两次 --list（all 与 --ignored）同 suite 名单必须完全相同且恰6/5，每个父独立 process 调 --ignored --exact；仅最后一次且唯一 summary 为1/0/0才收 passed，0 tests、ignored不执行、skipped、非0exit、超时、强制cleanup或组/footprint遗留均非绿。
- 实际主 libtest artifact 需唯一且 manifest/src_path 属当前树，exe 非 symlink；真实 exe digest 和 git source frame 前后检查。函数文档明确依赖构建来源由调用者证明：artifact 路径不证明 cache producer 字节正确。此入口的 git frame不含所有ignored生成产物，不能代替 root 完整235x/gen/CLI/source producer receipts；当前 CI 构建和治理 runner仍应保相应证明。
- 最终035a新增 pre-Popen check_space(root,binary.parent)，poll复用同helper。静态 AST line118 < Popen126；受控超大floor自测 Popen调用0、无log创建，正确拒绝。普通正对照也通过，入口并非一律拒绝。

## 独立控制自测结果（fake only，native业务父计数0）

- 旧 bc278：zero/skip/binary mutation 均 exit1；timeout 也非绿但当时EPERM，未据此宣称 signal cleanup 成功。setsid 漏门如上原样保留。
- f695：zero/skip/mutation 均 exit1且 passes0；短0.2s timeout 实际 fake PID7206 exit-15、timed_out/forced均true、remaining[]/footprint[]，无原 EPERM；同 setsid 反例正确拒绝并回收。TIMEOUT_SECONDS 的短值仅 control 内存覆盖，不改受审源码300秒。
- 最终035a：6项 runtime-close fake 正对照 exit0/fake passes6；5项 doc-transfer fake 正对照 exit0/fake passes5；缺 ignored项与缺父数均 exit1/passes0；最终 source SHA 前后稳定。它们只证明 entry 的 inventory/dispatch/summary协议，绝不计为 native 6/5父或 native 功能绿。
- 最终035a：纯控制超大磁盘floor10^30 +禁止Popen stub，RuntimeError Disk below acceptance floor、Popen0、无log。当前全部自有 fake path/tag process footprint 为0；其它 Q052 native 进程未触碰。

证据目录02保全部控制raw/两次sourceframe/result/ancestry/最后scope/资源门及四源快照；旧目录原始反例保留。二者分别双盘逐文件SHA核齐。没有新增 Cargo、npm/Bun生成、native业务执行、App验收或源修改。

## 交接

本审可交 root 继续最终 aggregate freeze与受控真实验证。无本范围新增阻断；真实CLI/Node/GUI/native inventory/Exit与主动退出Q043仍待对应槽验证，不能以 fake 6/5正对照代替。R2未知task/lateSend fence、Stop旧5.27秒原因与整体Q037仍open。
