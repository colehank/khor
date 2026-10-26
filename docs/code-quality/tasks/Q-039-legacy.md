# Q039：真正Node客户端的legacy协议回归

状态：**superseded-by-user**。用户最新决定无需旧端兼容、按尚未上线的纯净开发收敛；本支线已停止。下方准备说明保留为历史，实际执行与撤回以文末记录为准；新增legacy测试不提交、不集成。
负责人native_helpers；生产/集成/唯一构建槽由root协调。

## 契约与现场

- 用户完整治理授权下，root派发本兼容测试批。复用`/Volumes/disk0/Khor/worktrees/current/khor-quality-native-io`，`fix/quality-sync-chunked`，起始HEAD **8d87af99fb97da3d3d61435df82dcd7bae3c5eef**，接审时tracked clean；未新建tree/target或切分支。
- 唯一源写入为扩展`crates/node/tests/sync_doc_isolation.rs`，不改link/util/public API/Cargo/锁。选择复用现有独立worker、父截止、真实pair准备和合法IPv6历史helper，避免复制第二套进程隔离。
- 读中央STATE/PLAN和[Q-039-node-isolation.md](Q-039-node-isolation.md)的研究/真实现代证据后按graph-first查询；图谱built2a11、当前HEAD8d87不匹配且semantic未覆盖endpoint/Versions/wire，转现场link/proto/identity/endpoint/wire/store与util。没有用0结果或旧图替代实际源。
- 当前root明确仅源码准备；fanout_owner/之后root拥有Cargo/native槽。作者未Cargo、未触真实外网/provider/用户凭据/用户store/真实HOME。

## 确切fixture路径

新增3个**普通父测试**，分别启动当前binary唯一ignored worker，case区分legacy-oversize/legacy-epoch/legacy-optional；顶层不使用`--ignored`。沿用原父started/passed标记与status检查，不能将0测试计绿。

每例：

1. 两个独占scratch root先`util::hermetic`，仅loopback自有relay、only_mine、禁mDNS；现有`Node::open_hosting(..., false)`禁止无关provider/process发现。
2. A真实resident serve，B调用真正`Node::pair`完成认证/roster并验证两端仅2设备；没有手工伪造pair成功。
3. 用既有公开`link::deafen`关闭A的旧endpoint并等待ack，再abort+await原serve。仅从**本例A自己的**`.khor/identity.key`重开同key受控端点，断言endpoint id仍是配对A。更新两端公开DeviceDoc中的A真实新addrs，避免同时同key端点或拨旧端点。
4. 受控端点只放行已pair的B `remote_id`与Main ALPN；真实QUIC认证后，明确对Versions返回`Response::Refused`，对Vouch亦明确拒绝。客户端保持当前生产`Node::sync_now`，由它实际选择`sync_everything`，fixture没有抄写Node fallback或聚合策略。
5. 对devices/Seen/Pins和mandatory `wide/all`，加载真实公开Doc/BlockStore并调用真正`wire::answer`；对8个optional global docs及6种readings明确Refused。statistics只含测试id、wire name、pull-only布尔、计数/错误，不保存正文、归档、tokens或身份key。
6. B没有resident/inbound loop，写独立Seen123/Pintrue并flush后，只调用**一次真正B.sync_now**。没有直接wire修复Seen/Pins，A端只被动回答。

这是受控“旧协议能力端”的互通验证，**不是完整历史旧版Node binary实跑**。fixture解码使用当前proto，但行为显式拒绝不支持的op；最终只有真正Node客户端的fallback路径接受验证。

## 三个判据

| 普通父 | 要求 |
| --- | --- |
| `legacy_oversized_devices_reports_failure_but_seen_and_pins_still_sync` | 仍用原4096×64合法IPv6更新既有B自己id，恢复真实name/addrs，rawgap>24MiB且>4MiB；devices超cap使整轮Err，完整catalog key为outbound-too-big、参数上限4096KiB、实际大小>=24MiB，按参数重新渲染必须逐字等于原原因；A Seen123/Pintrue，同轮继续mandatory broadcast/optional/readings。 |
| `legacy_other_devices_epoch_reports_failure_but_seen_and_pins_still_sync` | 真实公开devices落盘epoch=B EPOCH+1，与A隔离；旧legacy契约仍**Err**而非现代的Ok skip；后续Seen123/Pintrue、broadcast和readings仍推进/被访问。 |
| `legacy_optional_refusals_do_not_fail_an_otherwise_successful_round` | 无devices人为故障，8个optional拒绝仍**Ok**，mandatory devices/Seen/Pins/broadcast均真实answer且Seen/Pin落盘；6读数拒绝保持best effort。 |

共有判据：认证连接>0、无陌生身份、无fixture transport异常、Versions明确拒绝>0、Vouch拒绝1次、首个文档为devices且首请求pull-only（区别现代known-version直接push）、mandatory Seen/Pins/broadcast均被访问，optional拒绝8、readings拒绝6。

两失败例仍用公开DeviceDoc写未解析receive_dir，不调用会立刻follow的Node setter；须目录不存在、B receive_for不变、A epoch/receive_dir不混入、A VV不包含被拒source历史。optional成功例不制造pending receive，保留成功devices可follow的既有行为。

oversize原因不把先验gap KiB作为硬编码：sync_now允许追加少量真实路由op，故要求**完整catalogued超限原因+解析出的真实大小/上限再渲染一致**，不以误匹配数字、pair/dial失败或阈值放宽冒充目标通过。原现代两case原来的完整cap_reason断言保持原字节，没有弱化。

## 所有权与截止

- 沿用已有每worker private HOME，父spawn时移除CODEX_HOME/CLAUDE_CONFIG_DIR/XDG_CONFIG_HOME/KHOR_HOME/KHOR_NAME及外部relay env，并将provider URL限制到loopback拒绝端口。父RAII只kill/wait本例child，自己的scratch收口后删除；不动别人的进程或store。
- 原operation90s/worker180s/父外部墙钟210s不变。同步CRDT调用不可被Tokio软timeout保证打断，故真实外部父截止仍必要。
- 原Node resident的Serving guard在失败时abort，成功替换前deafen+await；legacy server guard拥有endpoint和accept任务，accept任务JoinSet拥有连接，连接JoinSet拥有streams；每握手/stream90s。Drop取消本fixture任务树，正常stop abort+await并close endpoint，worker退出回收util自有relay runtime。
- 没有新增产品测试开关。既有deafen仅用于原resident退役，不改变fallback策略或伪造同步结果。

## 准备源与轻量检查

- 新test SHA256 **7fd273d3db15a6eb27765927a139613b3170e40fd119ce45b68c662ca557ed09**；原已提交测试 **dbfd51271ab79ad2eb1fb1fe223cacca2c152bcf1b58551da0b1e0a4afaa5c74**。新增489行/替换5行主要为imports、worker分发和ignore说明；原`exercise`函数整体、原合法历史helper和原两普通父通过逐字substring核验不变。
- 生产link仍 **62ebbe6b46fb29dc52a832d406553a0c840c97c9758cfb4075525d9ac378ea1e**，未改。
- `Q039/legacy-prepared/source-prepared.json`保存18项实际来源SHA，`prepared.diff`保存测试差异。此时未编译/运行，不能用prepared名称冒充运行证据。
- 仅新test执行`RUSTUP_AUTO_INSTALL=0 rustfmt --edition 2024 --config skip_children=true`与`--check`，并`git diff --check`，均exit0。它们仅证明解析/格式，不证明类型/运行。

## 排队命令与证据边界

root授槽后，先核两盘>=5GiB、全第一方源mtime与精确字节SHA；不得运行中改源。复用既有quality-target，不切Q037新源/不新建target。先小optional-only前置有利于发现认证/端点换代错误，再oversize/epoch；运行次序由root安排。

```
KHOR_NODE_ISOLATION_SCRATCH=/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q039/legacy-run/scratch \
CARGO_TARGET_DIR=/Volumes/disk0/Khor/caches/rust/khor-quality-target \
cargo test --locked --offline -j 2 -p khor-node --test sync_doc_isolation legacy_ \
  -- --nocapture --test-threads=1
```

此selector应实际运行3普通父/3专用worker，不使用顶层ignored。必须保留Cargo完整log、真实计数、protocol stats、rawgap/epoch/typed Err或Ok、Seen/Pin/follow、source-before/after、容量及自有PGID/墙钟护栏。编译/pair/dial/超时/fixture错误算前置失败，不当产品红；未运行不计绿。

既有独立现代真红（Seen0/Pinfalse）及同源epoch绿证据仍属于原dbfd+旧link；root现代修复2绿属于62eb+dbfd。本批不重复/改写原始证据，不预借现代2绿证明legacy，也不关闭25MB gap本身不收敛的bulk/chunk问题。测试提交等精确运行后收口，当前源冻结待只读审/排槽，STATE由root维护。

## 实际执行历史与用户决定后的停止（2026-10-26）

root源审后曾授唯一槽。仅本树340个`crates/**/*.rs`和Cargo.toml touch mtime，SHA前后相同；没有touch vendor/main/发布target。完整manifest在`legacy-optional-01/first-party-{before,after}-touch.json`，Cargo log实际显示第一方本地catalog/detect/core/node/a2a/sync/acp/net重编。所有运行保存341项源前后（含全部第一方Rust/Cargo与锁），jobs2/common quality-target/offline、5GiB和自有PGID墙钟护栏；运行中不改源。

| 历史轮次 | 真实结果与边界 |
| --- | --- |
| `Q039/legacy-optional-01` | runner29551/Cargo PGID29562；编译45.59秒成功，父1/worker1执行后在deafen返回false处前置失败，未启动受控端点/未sync_now。总47.984秒/exit101/source稳定，最低外8,444,776,448B/内11,739,553,792B。**不是产品红**。 |
| 前置修正 | 实际source核明deafen只登记resumable serve；仅legacy准备`serve()`改为现有`serve_resumable()`。deafen在真实ep.close().await后ACK，再abort+await旧resident。原两modern函数/历史helper/父逐字保留。最终legacy候选test SHA **b8f11f81a7cf732786541e40d0f1fac07a2bab68f99a1ca6dd4e4c0c790c1e0c**，link62eb未改。原7fd/prepared/失败证据全部保留。 |
| `Q039/legacy-optional-02` | runner31166/PGID31177，optional exact父1/worker1通过，含编译9.595秒/source稳定。samekey认证1、Versions拒绝1、Vouch拒绝1、devices首pull-only、8optional/6reading拒绝、errors=[]；整轮Ok2763B、Seen123/Pintrue。最低外8,381,300,736B/内11,742,314,496B。 |
| `Q039/legacy-green-01` | 同源b8f11+62eb，runner31691/PGID31702，普通父3/3、三个专用worker各1/1；总11.096秒/exit0/source稳定。optional Ok2769B；epoch2/1仍明确Err；合法oversize gap**25,448,407B**仍完整Err24851/4096KiB。三例均Seen123/Pintrue、mandatory broadcast访问、8optional/6reading拒绝、无错误身份/fixture异常；失败devices无follow/无混入断言通过。最低外8,357,163,008B/内11,752,132,608B。 |

最终运行binary SHA **01a54827c259dfd534c03107e324d6f26f66414c9b1a1b8a17f816b4cf64797a**，size193244448，binary-after记录path/mtime。无guard触发，29562/31177/31702各已退出，最后两组killpg(signal0)已确认不存在；结果产生后立即交还Cargo/native槽。此为真实Node客户端+受控legacy协议fixture历史结果，不是完整旧binary证明；没有回退生产另取legacy业务红。

随后用户明确无需旧端兼容，root立即撤销本支线。作者**未执行git commit、未集成新增legacy测试、未继续Cargo/探针**。先将完整b8f11 test、binary diff、任务卡快照、三轮Cargo log/process/result及全部341源manifest、340项mtime前后记录复制到内盘：

`/Users/zgh/Library/Caches/khor-code-quality/recovery/inflight-source/Q039-legacy-final`

backup-manifest记录33文件SHA，restore-result记录校验与恢复。原外盘证据未删除。随后仅把本次唯一改动`sync_doc_isolation.rs`恢复到HEAD8d87的已提交原文，最终SHA **dbfd51271ab79ad2eb1fb1fe223cacca2c152bcf1b58551da0b1e0a4afaa5c74**；生产link仍 **62ebbe6b46fb29dc52a832d406553a0c840c97c9758cfb4075525d9ac378ea1e**，其他文件字节未改。原分支 **8d87af99fb97da3d3d61435df82dcd7bae3c5eef tracked clean**，原modern两例保留。

此历史3绿不成为当前协议设计/集成门禁要求，不再沿旧兼容方向扩展。当前Q039设计由root按用户纯净开发决定重新收敛；大差集本身收敛仍开放。STATE由root维护；本支线没有活动进程或槽占用。
