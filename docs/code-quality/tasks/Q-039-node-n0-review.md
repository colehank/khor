# Q039 N0 / N0b 独立测试源审查

日期：2026-10-27。审查者：`node_test_review`，非测试作者。仅写本卡；未改 Rust、未 Cargo/typecheck/运行、未提交或合并。先读中央 STATE、PLAN、Q-039-node-transfer。

## 最新 Pair04：有限证据独审确认真实产品红

本节覆盖下文Pair02仍仅setup失败的历史进度；02/03原失败均保留。审查者未编译/测试/改源/清目录，仅核已执行的 `Q039/pair-bootstrap-first-red-04`。原始raw（含Cargo JSON与实际libtest输出）、runner、compiler artifacts、source/binary/guard/cleanup与final audit已完整读取/逐项解析；不只照抄assessment分类。

固定 recovery `fix/quality-node-doc-transfer@67927fcfacb84fe260553e31614d8c1e2fd1cd5c`，Pair04保存/live/candidate源SHA均为 **`e7dbda3c9c74c757f31663ea82d6d4f16dd1ae3399f29efd4fb0c24f1a512e4f`**；N0仍 **`76495a0b1a47c92abbd58996a94f3710836a961b1c4ba4d9b080c3f88ab0d4fa`**。独立逐diff核03→04仅合法既有issuer id地址历史 loop **4096→12288** 与一行说明；仍64项/轮、末尾恢复真实hints、不加假设备。没有改 `snapshot >24MiB`、`>MAX_FRAME`、`<=64MiB`、epoch/target/content/root、same-caller retry或third身份exact bad-invite门。此数据delta已由root批准，不是降低oracle。03实际snapshot日志为9,742,899B，故原24MiB准备失败有明确Snapshot/Updates差异，未被改写成产品红。

**04 raw直接支持1个普通父产品红**：

- 真host/relay/public Pair fixture准备成功：`snapshot_bytes=29224082 epoch=1 ticket_present=true`，实际29,224,082B超过24MiB/6MiB且在共享64MiB内。
- 首次public Pair返回 `msg.no-answer`，保留redacted具体原因 **`stream too long`**，同时 `issuer_has_caller=true ticket_consumed=true`。这次有原始具体stream长度拒绝原因，结合已审当前Paired全snapshot/base64→client read_to_end(6MiB)路径，足以归因为巨大应答被有界reader拒绝后的半入网；不是仅凭row/burn猜frame cap。
- 同caller同票重试：`retry_ok=false retry_error_key=Some("msg.bad-invite")`。另fresh identity同票明确 `other_error_key=Some("msg.bad-invite")`，不是dial/任意Err假拒绝；其root/row与issuer仍2rows等前置断言均已越过。
- 最终重新load/reopen的诊断：`target_covered=false content_matches=false epoch=1/1 rows=1 same_root=false holds=true`，caller仍持自己的lone root（standing trusted只是该lone root下的本地状态，不表示已加入issuer）。worker停止resident后在`:330` samecaller retry必须恢复的业务断言失败。**后面的target/content/root assert因前面panic未执行**；它们的未收敛由已计算raw诊断支持，不假称每条assert分别运行失败。
- 普通父因真实worker exit101失败；raw两条FAILED summary分别为父和它spawn的ignored worker，合计只算**一个ordinary产品红**。这是Pair巨大bootstrap+spent-ticket retry缺陷首红，没有lostreply注入/kill窗口、claim恢复或新协议收敛证明。

source inventory核：**1075**个root/phase before=after=current全同；与真正执行过的03 source-after相比，只有该Pair测试一源变化，其他1074输入相同。03 raw明确曾从当前recovery路径编译Node/test；04不再touch依赖mtime。04原Cargo JSON共700 compiler-artifact记录，current-tree `khor_node` manifest/src路径正确、features=[]、**fresh=true**（复用同树同生产输入库），Pair test路径正确、features=[]、**fresh=false**（新编测试）。不能将fresh=true说成04重编整个生产库，也不拿别树/main图覆盖当当前来源。此1075选择仍不是所有ignored frontend/generated输入的穷尽证明。

实际test binary来自同raw Cargo artifact与Running行：`pair_bootstrap_large-70bb126099e049fc`，191,761,640B，mtime_ns1793039865224841749，SHA **`541fdf8539728425bffd3a4e7bb873567496c60db97f4e1c2267258404af6e0c`**。与run前03 binary SHA c856cd…不同；actual=after，独审时current SHA/size/mtime也三项一致。后续共享target合法覆盖仍不能替换此历史收据。

实际runner AST与launcher/preflight/final一致：**STEP300s/TOTAL420s/MIN5,368,709,120B**，loop分别使用，locked/offline/jobs1、incremental0、quality-target与短owned TMP明确；源/HEAD/相对03唯一delta prechecks均在launch前assert。仅跑exact Pair04，没有重跑N0/normal check或不相关全库。phase25.1067s、总25.627044s，worker/父13.20/13.23s；无guard触发。采样minimum free外盘 **6,131,322,880B**、内盘 **14,772,289,536B**，均>5GiB。

Cargo PGID **89713**当前独立signal0为ESRCH；scratch `/Volumes/disk0/Khor/caches/tmp/pred-04`当前不存在。phase cleanup receipt为零files/3空目录，root为零files/1空目录；这是原receipt+当前absence，不宣称审查者亲见清理前列表或亲自回收。外盘run与 internal `/Users/zgh/Library/Caches/khor-code-quality/recovery/pair-bootstrap-first-red-04` relative sets及全部 **24 files** SHA/bytes逐项相同，finalaudit已齐；`execution_complete=true`仅指该单父首红实验完成，不是业务绿。

结论：Pair04首红源与有限证据**无阻断**，可作为后续修复的独立Pair基线；连同之前N0，是两不同普通父在不同run各自取得产品红，绝不把任何父/ignored child双计。Node生产N1/Pair实现授权另由root派发，本审查没有新设计或实施授权扩张。

## 最新首跑证据独审：N0产品红，N0b仅准备失败

本节覆盖下文“尚未运行”的历史状态。审查范围只为已执行的 `Q039/node-transfer-first-red-01`；未启动任何编译/测试/gen，未改源码或清目录。完整读取normal/N0/N0b raw logs、实际runner、preflight/launcher AST、全部phase source/binary/results/cleanup与final audit，不只接受runner分类字符串。

固定运行树为 recovery `fix/quality-node-doc-transfer@67927fcfacb84fe260553e31614d8c1e2fd1cd5c`。保存的实际测试源 N0 SHA **`76495a0b1a47c92abbd58996a94f3710836a961b1c4ba4d9b080c3f88ab0d4fa`**，Pair N0b02 SHA **`0f21a85006a32a22fe2a8b4473c2f47828c2072ace1de7a234472a2f82616a84`**，均等于先前独审冻结。现场HEAD仍67927fcf，当前1075输入中唯Pair测试已合法进入后续03日志前移修订；不把当前Pair bytes或新诊断运行替代此02记录。

| 实际phase | 直接证据与结论 |
| --- | --- |
| normal-lib | 标准默认 `cargo check --locked --offline -j1 -p khor-node --lib` exit0；raw明确从当前recovery树检查Node/依赖。是正常库编译，不计ordinary test。 |
| N0 exact ordinary parent | exit101，raw先编译实际测试、再spawn真正ignored worker；准备完成 `raw_gap=25447954 old_cap=4194304 epoch=1/1`，baseline Seen0/Pinfalse。四次真实public Node sync visit均明确旧cap Err，且每次 `seen=123 pinned=true target_covered=false content_matches=false cap_rejections=1/2/3/4`。最终worker在`:679`的固定target/receive-dir必须收敛业务断言失败；父再因worker exit101失败。**确认1个普通父的真实收敛产品红**，没有relay/setup/无关transport原因替代。 |
| N0b02 exact ordinary parent | exit101，raw只有started，worker在`:218` `retain the full 25.4 MB history` 的snapshot大小准备断言失败。代码顺序与保存源证明此时尚未执行第一次public pair；raw也没有prepared/first_ok/retry日志。**fixture/setup失败，Pair产品红尚未取得**；不能推论burn、巨大Paired frame cap、same-caller retry或third-identity拒绝已运行。 |

N0的真实relay/nonhosting/public pair/sync及fresh persisted-read依据前面的精确源审，实际raw已越过配对/size/baseline门，恰完成四visit再落最终目标断言。Seen/Pin首visit已先推进，不能把它们的成功称devices已同步；name/addrs诊断不参与最终等值。N0两条失败summary分别是同一ordinary父与它实际spawn的ignored worker，**不计两条产品红**。N0b同样只是一个ordinary父的setup失败；没有两组产品红，也没有任何收敛绿。

所有1075个明示related输入的whole-run before=after，normal/N0/N0b各自before/after也逐项完全相同；保存源hash与当时manifest一致。独审重算当前1075仅Pair源不同，说明后续源演进而非首跑中漂移。365个first-party source/manifest mtime刷新有记录，原raw实际编译路径正确；这1075选择不声称覆盖所有ignored frontend/generated构建输入。

两份原实际binary记录均与对应binary-after的SHA/size/mtime相同：

- N0 `sync_doc_isolation-862f65292cd2f541`，191,836,544B，SHA `c22d409e25ad1bb61f52d3f89a9fd1dd0599e030e1f1d08b0c3d3764c4e26b41`；与run前旧binary SHA不同，原raw Cargo Running指向此精确路径。
- N0b `pair_bootstrap_large-70bb126099e049fc`，191,761,640B，SHA `9fa42435cf508c3eca345acfe75204c678419b2deeab918742c628f8d624e438`；run前无此binary，raw Running一致。

本次独审时两binary当前仍三项匹配；后续共享target合法覆盖不回填首跑历史收据。仅二binary/test源码编译不能扩称全部Node测试通过。

独立解析实际run.py AST，与launcher/preflight/final一致：**STEP300s、TOTAL720s、MIN5,368,709,120B**；实际loop分开引用，不借计划中240/420等其它阈值作护栏信用。launcher记录parsed_before_launch，但没有该launcher中的script SHA字段；不额外宣称拥有不存在的launch-hash收据。precheck为assert，源码/路径错误会阻止launch；locked/offline/jobs1、incremental0、quality-target及每phase短owned TMP明确。连续3次IO/资源/deadline只停自有新session PGID。本run三phase无guard触发，总129.282751875s，N0 worker/父实际9.45/9.49s，Pair worker/父1.68/1.73s；余时间主要编译。

记录minimum free外盘 **6,257,033,216B**、内盘 **12,829,810,688B**，均超5GiB。这是runner采样记录。三个Cargo PGID **74525/74769/75710** 当前独立signal0均ESRCH，父等待worker exit有raw/source证据；没有把Rust thread id冒充OS child PID。各phase cleanup为零remaining files、只删1/3/3空目录，最终owned scratch `/Volumes/disk0/Khor/caches/tmp/nt-red-01` 当前不存在。审查者未亲见清理前目录，准确归属为原cleanup receipts + 当前absence，未自行删任何文件。

外盘证据与 `/Users/zgh/Library/Caches/khor-code-quality/recovery/node-transfer-first-red-01` 的relative sets及全部 **35 files** bytes/SHA逐一相同。final `all_planned_steps_executed=true` 而 `execution_complete=false` 与Pair setup失败一致，不把总runner非零误说N0无法归因。N0首红证据无阻断，可作为后续生产修复前基线；Pair必须保原失败，分析真实Snapshot/Updates体积后另冻合法新fixture及新run，不能降低断言凑Pair红。

## 最新 delta 结论：N0b candidate02

本节覆盖下文 candidate01 的阻断状态；保留下文为历史独审记录。作者仅补 B1/B2 与 token 借用，当前 N0b SHA **`0f21a85006a32a22fe2a8b4473c2f47828c2072ace1de7a234472a2f82616a84`**。独立逐 delta 核：第三 caller 必须 `expect_err` 且 catalog key 精确为 `msg.bad-invite`，真实 transport/IO Err 不再假绿；首次完整 why 先替换完整 ticket、再替换 raw token 为 `[redacted]` 才打印，保 catalog key。原 row/root/固定 target/epoch/content 断言未弱化。

外盘 `Q039/pair-bootstrap-n0b-candidate-02` 与 internal recovery 同名目录 6 files 全部逐字匹配；source-before 等于 candidate01 原源、current 等于 candidate02 snapshot、445 selected Rust/Cargo inputs 全部与现场匹配。N0 SHA `76495a0b…` 未变。两普通父当前均**无源码阻断，可分别授 exact 首跑**；本审查仍未编译、未运行，不算首红。B1/B2 源码阻断已关闭。

首红归因与协议边界仍保留：first row+burn 仅 reachability，具体巨大 Paired/frame cap 必须由实际 why/运行证据支持；不足时只称 large-bootstrap + retry 失败。首次 burn 前置用于当前旧路径定位；未来合法 preburn Pending 若需要等价迁移 oracle，先由 root 明确契约，不为 fixture 提前烧票。第三 caller 的新协议明确 owner/spent refusal 类别如更名也需显式迁移，不能退回 any Err。未改生产，未动 Cargo 槽。

## 精确范围与结论

实现树 `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-overwrite-recovery`，`fix/quality-node-doc-transfer`，HEAD `67927fcfacb84fe260553e31614d8c1e2fd1cd5c`，现场恰 `sync_doc_isolation.rs` tracked dirty 与 `pair_bootstrap_large.rs` untracked。完整阅读两个测试及 `tests/util/mod.rs`，并跟到当前真实 `Node::pair/pair_with/sync_now/via_serve`、dispatch Pair/Sync、exchange/MAX_FRAME、wire export、trust settle/root_state、store load/cache、DeviceDoc history/export。

| 普通父 | 冻结源 SHA256 | 源审结论 |
| --- | --- | --- |
| `a_large_devices_gap_converges_through_real_node_transfer` | `76495a0b1a47c92abbd58996a94f3710836a961b1c4ba4d9b080c3f88ab0d4fa` | 无源码阻断，可单独 exact 首跑；尚未编译、红或绿。 |
| `a_large_pair_bootstrap_converges_and_the_same_caller_can_retry` | `14a6f2e6c144e4f4e02ea0864b07030db1f430916261081c5a7a9abea34cd931` | 有中风险测试归因阻断：不同 caller 的认证拒绝只检查 any Err；首次巨大应答错误只记录 catalog key，未保实际 cap 原因。建议最小测试修订、重冻再 exact 首跑，不改生产。 |

本卡不作生产合并建议。两个父必须独立执行并各自判定：编译、relay/setup、无关 transport、cleanup 失败均不算目标产品首红。

## Graph 证据边界

依 AGENTS 先调用 code-review-graph。`list_repos` 为 0；此树无 `.codegraph`，因此没有调用 CodeGraph。review-changes 技能要求的 minimal-context/detect-changes 返回图仅 51 files，build branch `fix/quality-transfer-initialization`、SHA `92742250…`，明确 `head_matches_build=false`；detect 的 8 changed files、0 functions/flows/test gaps 与现场两文件不一致，不能作为无风险/覆盖证明。minimal-context 与 affected-flows 的 `detail_level=minimal` 参数被服务拒绝；去参数后 minimal-context 可读但仍旧图。随后直接审真实源，没有拿 graph 的 0 代替覆盖。

## N0：无阻断

- 真 Node nonhosting resident + 私有 loopback relay；`relays_n0=false/map.len=1`，util 先禁 mDNS，sender 没有 resident。`pair`/`sync_now` 会走真实公有 Node 驱动；sender 无 endpoint 文件时走 direct authenticated iroh，receiver 用真实 serve/dispatch。没有 fixture transport/import history，没有伪 RPC peer。
- 先真实配对并核两端 2 rows，再对 sender 既有真实 id 写完整 4096×64 地址历史，恢复真实 hints，设置 owned receive-dir 并真实 flush。raw exported gap 必须大于 24MiB、旧 4MiB cap，且不超过共享 64MiB body 上限；固定 target VV/epoch，receiver 初始不覆盖 target。没有添加第三个假机器或以随机无效 body 代替历史。
- 每次至多一个真实 paired receiver；首 visit 后及后续每次 Seen=123/Pin=true 必须重新 load 得到。最多四次 visit，最终同时要求 fresh devices epoch、2 rows、固定 target VV 覆盖、receive-dir 值一致。最终 name/addrs 只诊断，允许握手合法更新。只把 `msg.outbound-too-big` 分类为当前预期旧 cap；其他 Err 立即单独失败，不能算目标红。
- 每次 load 是新的 Loro copy/lease；store 的 kept snapshot 仅来自真实磁盘 fingerprint 匹配的已读块，不是直接复用 sender/receiver 的 live Doc alias。因此这是 fresh persisted-read oracle，但不是独立进程重启/断电证明。
- `Serving::stop` 在最终 intended convergence assert 前 abort/join resident；早期 panic 靠 Serving Drop 与 owned worker process/runtime 回收。父保 started/passed marker、真实 exit status、防零测试、90s operation/180s worker/210s process fuse。父只 kill/wait 自己 child、清自己 unique scratch；HOME 私有且已移除 provider/config/relay overrides。
- 独立执行精确 inverse edit 后，baseline 文件 SHA `effbb7aa0a86dd2e2b82b6ceab888c804774bfe1926a89818ce65a5ffa022574` 逐字相同：原四普通父、原 exercise/exercise_inventory、shared fixture/helpers/oracles 未弱化。原 oversize cap oracle 以后改接新生产时仍须按 Q039 计划显式替换并解释，不能把它与新增 convergence 混称同一未变测试。

## N0b：中风险阻断与协议边界

**B1 — 不同身份的 any Err 不能证明认证拒绝。** `pair_bootstrap_large.rs:287–293` 仅 `other_result.is_err()`；如果此时 dial、stream、内部超时/IO 失败，也会继续通过 other 的 1 row/root 不变及 issuer 的 2 rows 检查，并打印 `other_refused=true`。这些状态同样符合请求根本没到 issuer。这是成功路径上的认证 oracle 假绿缺口。应要求真实、明确的已消费票/claim-owner 不匹配拒绝：当前基线可核 catalog `msg.bad-invite`；新协议可用其明确拒绝类别。仍须保超时和双方状态检查，不应改成任意 Err 或靠不同 caller 恰无网络证明权限门。

**B2 — 首次失败具体原因丢失，不能仅凭最后 retry assert 叫巨大 Paired 首红。** `:242–258` 的 row+ticket-consumed 是有价值的 reachability：排除了请求前没能到达 issuer。但 `first_error_key` 丢掉 `msg.no-answer` 的参数；burn/enroll 之后的无关网络中断、server IO/应答错误也能留下相同 row/票状态，随后同票 retry 必然 bad-invite，最终 `retry_success` 红。建议保不含 ticket/token 的实际失败详情或明确 frame-cap 诊断，并在首跑归因中核该具体原因；固定旧错误文字只适合当前首红分类，不应成为新协议永久成功 oracle。源审预期当前路径为：dispatch burn/enroll/flush → `wire::table_b64` 不限长 snapshot → msgpack Paired → caller `read_to_end(MAX_FRAME=6MiB)` 拒绝，第二次为 spent ticket；这只是代码推导，尚无运行证据。

**首次 burn 的前置不是通用恢复协议契约。** `:259–266` 强制第一次 public pair 返回时 issuer 已有 row 且原 invite 文件消失。对计划中的无故障 public pair 在一次调用内完整驱动所有阶段、或已 `AdmissionFlushed` 后失败/完成的实现，这个要求不妨碍合法新协议；合法完成后同 caller 的 completed local intent 幂等返回也可通过。它并未强制第一次必须失败。可是若新协议允许第一调用在 `ClaimReceiving`/roster 验证完成前合法返回 Pending/ResourceLimit，保票、下一次同 caller 恢复，当前两个前置 assert 会抢先失败，根本到不了 retry oracle。不得为过此 fixture 提前烧票或弱化 pure root/签名检查；应届时按明确 public pair 契约分开 current-red reachability 与 acceptance，记录 oracle 变更。此处没有注入丢 reply/kill，不能说覆盖那些窗口。

其他已核范围：

- issuer invite 之后才在自身唯一 existing id 写完整相同 history/restore hints；pair 大小以真实 snapshot >24MiB、>6MiB、<=64MiB 核。只用 Ticket::decode 取 owned 私有 invite path；未打印 raw ticket/token/签名/key。没有造 roster 或绕过 crypto；真实 issuer/other 均初始 fresh lone root，实际 settle/verify/enroll 执行。
- second 同一个 public caller identity/同 ticket；成功既可真正恢复也可本地 completed intent 幂等完成。fresh caller store 必须 target VV covers、epoch/2 rows、issuer receive-dir 正确；另 Node reopen 核原 issuer verified public root、caller 不持 lone root、standing trusted。第三身份状态与 issuer row 检查保留，B1 只补拒绝类别，不替代这些检查。
- 私有 HOME/relay、worker started/passed、kill/wait/unique scratch、deadline 与 N0 同类；issuer stop 在最终 retry/content/root assertions 前，前置失败仍由 worker/process 清理。180s worker 总限独立于三次各90s operation，不会叠成270s无限等待；超过总限应单列 timeout，不能叫目标红。stop/join serve 不代表所有 detached task 即刻退出，最终 worker runtime/process 才是 relay/task 生命周期边界。
- 本组不证明同 caller 重启/nonce 持久恢复、最后 reply 丢失、claim journal 各 kill 窗口、leave/revoked/root changed 后旧 completed intent 不复活、巨大 signed roster、坏签名/Clash 保票；这些仍归 N3，不用本父填覆盖表。

## 冻结与复核

两个外盘 candidate 目录与 internal recovery 同名目录分别 5/4 文件，逐文件内容/hash 全相同；source/source-before/完整 patch 均在内。两个 selected-inputs 清单相同，445 个明示选择的 Rust/Cargo 输入与现场全部匹配；不是全 frontend/generated build 输入保证。两新文件中的 large_address_history 函数体逐字相同。独审执行 `git diff --check` 通过，没有运行 rustfmt/Cargo/测试；无自有运行 PID/端口/scratch。

证据目录：`/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q039/{node-transfer-n0-candidate-02,pair-bootstrap-n0b-candidate-01}`；内盘 `/Users/zgh/Library/Caches/khor-code-quality/recovery/` 下同名。首跑前 root 仍须独立冻结实际 build 输入、守唯一 Cargo 槽和资源/cleanup 规则，两个 exact 父分别判定。当前两组首红均未发生。
