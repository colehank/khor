# Q039：真实Node inventory加载失败隔离

状态：two-real-inventory-failures-red-awaiting-root-fix。两普通父/两专用worker实际执行，local饥饿与remote吞错均真红；测试effbb冻结，生产link/proto/catalog由root独占，作者未改生产。

## 边界与来源

- 用户最新要求统一当前版本，无旧端兼容。本测试不恢复legacy fixture，不造替身Doc/Node策略。
- worktree `/Volumes/disk0/Khor/worktrees/current/khor-quality-native-io`，branch fix/quality-sync-chunked，HEAD **ed46074626a94cfe272a50decfca5e601a1ad6a5**；接审tracked clean，仅修改获准`crates/node/tests/sync_doc_isolation.rs`。
- 中央STATE/PLAN、[当前协议卡](Q-039-current-protocol.md)与root最新归因纠正为依据。graph-first minimal context显示built2211/HEADed460不匹配，随后核实际opening/answer_versions/with_doc宏、accounts_loaded、AccountDoc/store error路径，不借旧图说明代替实际代码。
- test SHA **effbb7aa0a86dd2e2b82b6ceab888c804774bfe1926a89818ce65a5ffa022574**；Node link仍 **62ebbe6b46fb29dc52a832d406553a0c840c97c9758cfb4075525d9ac378ea1e**，proto b6fd92ec、sync store6c906fe1/glue5f7fb8df（ed460），锁4436a386。完整13源SHA、test副本/diff在`Q039/inventory-prepared`。
- 原oversize/epoch exercise完整函数、合法IPv6 history helper均与原dbfd字节一致，原两普通父及断言不改；仅新增两个父、独立inventory exercise、worker分派/说明。

## 两个真实fixture

普通父分别为`a_local_inventory_load_failure_reports_error_but_seen_and_pins_still_sync`与`a_remote_inventory_load_failure_reports_error_but_seen_and_pins_still_sync`，启动原private worker case inventory-local/remote。顶层不使用ignored；父须观察dedicated worker started/passed和真实退出状态，防0测试计绿。

1. A真实Node resident+B one-shot Node先通过真实pair完成认证/roster，两端设备数仅2。util::hermetic只有自有loopback relay、only_mine、禁mDNS，open_hosting(false)，继承原worker private HOME/vendor loopback拒绝URL与配置/relay env清理。不注册任何provider或凭据，不访问用户store。
2. pair完成后，在故障一端使用公开AccountDoc/store API创建真正空pile并成功load+compact成实际snapshot；断言文档无accounts、无broken。之后把**本例自己的**持久目录rename保存在root下，原`.khor/accounts`路径写普通文件。不会在Node::open准备期引入故障，也不删除任何保留store。
3. 真实`fs::read_dir`必须NotADirectory；`store::load_pile<AccountDoc>`必须返回精确`khor_catalog::msg::cant_list_dir(完整accounts路径, 实际I/O错误)`。此原因作为具体oracle，不能把dial/网络错误冒充inventory失败。
4. local case破坏B本机accounts，针对其opening；remote case破坏A的accounts，针对真实answer_versions。B写Seen123/Pintrue并flush，A先确认为Seen0/Pinfalse。B无resident，只有**一次真正B.sync_now**可推进文档。
5. 保存实际per-peer Err/Ok、Seen/Pin。若Err，必须包含完整具体accounts原因；业务最终要求Seen123/Pintrue且整轮Err，故无论“提前退出饥饿”还是“错误吞掉假成功”都不能通过。保存的坏file/有效snapshot目录不得被sync抹掉。

归因修正：with_doc已知accounts arm的load `?`会向外退出answer_versions，而不是落到elsecontinue；仅unknown doc宏最终Err才可能被elsecontinue忽略，正常ROUND_DOCS不包含unknown。**不先验remote必为Ok或必为Err+Seen0**：当前client可能还有Refused后的回退，真实日志/落盘结果裁定；本测试只定义当前协议应保留具体加载失败并继续独立doc的目标。

## 所有权、截止与运行计划

- 复用原父Worker RAII，只kill/wait自己确切child并回收自己的root；stdout/stderr私有文件，失败也由父完整转印日志。Serving guard失败abort，成功主动abort+await。relay runtime由自己的worker退出回收。
- operation90秒、worker180秒、父外部墙钟210秒保持；runner另有含编译总guard与双盘5GiB检查，仅终止自己PGID。不把Tokio timeout称为可取消同步CRDT栈。
- 源编译期间冻结，不改生产。first-party mtime只在本授权树触碰，前后记录字节SHA与实际重编；同quality-target/jobs2/locked/offline，不另建target。
- 首先执行两个新ordinary selectors；原两现代case不因未触业务而无故重跑。可用`--test sync_doc_isolation inventory_load_failure -- --nocapture --test-threads=1`（两个名字共同substring），顶层不加ignored。保存Cargo完整log、13源及实际编译输入before/after、容量、PGID、pair成功/实际accounts preflight/两个worker计数和完整outcomes。编译或pair/前置错误单列，不能称业务红。

当前仅rustfmt/--check和diff-check通过；本卡不预借此前modern2绿或inventory静态分析为新运行证明。root等两个实际结果才改link/proto，作者不改生产。STATE由root维护。

## 实际两红（2026-10-26）

root授唯一Cargo/native槽后执行`Q039/inventory-run-01/run.py`。运行前343项本树第一方.rs/Cargo.toml仅touch mtime，前后全部字节SHA相同，完整manifest保存；Cargo日志实际从本树重编catalog/detect/core/node/a2a/sync/acp/net，不借其他worktree较新artifact。

- runner46846/Cargo自有PGID46867，jobs2/shared quality-target/locked/offline；Node仍62eb、sync为ed460、test仍effbb。13项专项冻结源全部匹配，347项编译来源/锁/catalog前后**source_stable=true**。
- **编译55.10秒成功**；两个普通父实际运行、各自真正spawn专用worker，测试组3.43秒，总59.053秒，exit101，**2个目标业务红**。没有编译/pair/fixture/setup/0测试/timeout失败。
- 两例均实际pair成功，真实空AccountDoc snapshot的保存、普通file替换与store精确cant_list_dir preflight通过，A baseline均Seen0/Pinfalse。故障路径分别是本例sender/.khor/accounts与receiver/.khor/accounts，确切I/O原因均Not a directory (os error 20)。

| 实际case | 真实outcome与目标失败 |
| --- | --- |
| inventory-local | 唯一peer **Err(完整accounts cant_list_dir/NotADirectory原因)**，Seen **0** / Pin **false**；具体错误匹配已通过，最终在Seen expected123/actual0处真红。 |
| inventory-remote | 唯一peer **Ok("传输了 2160 字节")**，Seen **123** / Pin **true**；独立文档推进通过，但最终“selected accounts具体加载错误必须保留Err”断言真红。 |

remote实际输出不能由宏展开一句话替代：known accounts load会令answer_versions整体Refused，但当前client尚有Refused→Vouch/全量轮回退；它可继续Seen/Pins，同时把optional accounts exchange拒绝当best effort而最终报Ok。当前日志证明的是**最终吞错假成功**，不是将正常accounts load错误归到unknown doc的elsecontinue，也不是预先假设Seen必为0。新当前协议的目标仍为明确Err且Seen/Pins继续。

容量/所有权：最低external**5,580,218,368B**、internal**9,255,616,512B**；未触5GiB/总420秒guard。结束external5,580,468,224B，接近停止线，已提醒root下轮编译前fresh preflight；没有继续起新批。PGID46867结束后signal0确认不存在，private scratch为空，**唯一槽已立即归还**。

证据目录保留完整cargo.log、run.py、process/result、13准备源与347项source-before/after、343输入mtime前后。已编test binary SHA **b250843b8efb501b384aefe668b558754b2e699dda522a2a39c7717586e5035f**，size192580144，path/mtime记录在binary-after.json；共享target的同路径未来可重编，不能由该路径本身推定同源。

内盘准备快照已保存`/Users/zgh/Library/Caches/khor-code-quality/recovery/inflight-source/Q039-inventory-prepared`（完整effbb test/diff/13manifest/卡），原外盘证据不删除。作者仅写本新卡和获准测试，`Q-039-current-protocol.md`历史交付已正式交还root，不再混写。当前测试未提交/未改断言，root将基于这两实际红独占修复link/proto/catalog；绿/精确集成仍待，不能先标本任务完成。

## 测试提交与最终交还

root接收两实际红后仅授权保存本测试，已提交 **78e978a81261c85053e0d5ba27e842425fc85b57**（`test(node): expose inventory load failure isolation gaps`），父 **ed46074626a94cfe272a50decfca5e601a1ad6a5**，恰1文件，158新增/5替换。提交前后test仍effbb，原exercise/history主体字节复核不变，Node link仍62eb；fix/quality-sync-chunked **tracked clean**，receipt在inventory-run-01/commit-receipt.json。

测试源码正式交还root冻结/集成，作者不再改该测试、不追加编译。两红测试提交记录的是旧Node故障，不能由commit本身称运行绿；后续root会在wave1接入逐doc inventory错误并去除当前协议的旧fallback，再精确重跑。卡/源/结果内盘副本一并交还，STATE仍由root维护。没有本任务活动进程/槽占用；strict Doc apply不在本批。
