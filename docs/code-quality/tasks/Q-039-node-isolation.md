# Q039-A：真实 Node 同轮文档失败隔离回归

状态：integrated-awaiting-integrated-validation；真实Node修复后两个场景均绿，非作者最终源审通过，两提交已进入治理集成；精确集成运行待。
负责人 native_helpers；独立审查/生产 owner root。

## 任务契约与现场

- 用户授权全仓治理，多 agent 分批；root 本轮委派只新增 `crates/node/tests/sync_doc_isolation.rs`（如需 helper 必须独立新文件），不改现有 util、link.rs 或产品开关。root 独占后续 link.rs 修改，必须等真实 Node 红再修。
- worktree `/Volumes/disk0/Khor/worktrees/current/khor-quality-native-io`，原地 `fix/quality-sync-chunked`，HEAD **9c61993ea4a8f79f1bc3ac20ab2ef84d309ca1be**（d9 + Q039 研究 fcd）。未切分支/新建 worktree，旧 Q043 作者 overwrite 树仍 clean 冻结。
- 权威入口中央 STATE/PLAN；研究结论 `tasks/Q-039.md`。本卡只由作者更新，STATE 不改。
- graph-first：此树无 `.codegraph/`，跳过 CodeGraph；CRG semantic queries `sync_now/hermetic/sync_everything/DeviceDoc` 为 built54aa、HEAD mismatch 且缺 Rust 节点。随后核现场 link/store/devices/seen/pins/road、既有 pairing/util/subscriptions 测试。没有以旧索引代替现场，没有自行索引。

## 测试场景与可观察的判据

普通父测试两个，各自启动当前 test binary 的 `--exact node_doc_isolation_worker --ignored --nocapture --test-threads=1`。ignored 只用于 worker 入口，两个业务父测试默认实际运行，父必须看到 started/passed case 标记与 child 成功，不能将0测试计绿。

1. `an_oversized_devices_gap_reports_failure_but_seen_and_pins_still_sync`
   - 两个真实 Node 身份，经真实 Node::pair 完成认证/roster。A 自有 resident serve，B 为 one-shot client，无 B resident/inbound 请求循环。
   - 通过公开 `devices_dir + store::load_pile<DeviceDoc>`，在已存在的 **B 自己 id** 上生成同研究4096次×64合法 `2001:db8::/32` IPv6 socket地址变化，然后恢复真实 name/addrs，flush。没有额外假远端、padding字符串或绕过 wire cap。
   - 导出对 A 的真实差集必须 >4MiB 且 >24MiB，记录实际 raw_gap；只观察长度，导出 Vec 随即释放。已配对设备数仍2，最后实际需拨号 peer 只有A。
   - 同一公开 DeviceDoc 写B自己待解析 `receive_dir` 指向 scratch 子目录，只改文档，不调用会立刻解析的 Node setter；记录原 receive_for。实际 road.rs 核实 follow_receive_dir 会创建该目录并更新 receive_for，故它可观察失败后是否错误 follow。
   - B 公开 SeenDoc mark独立session **123**、PinDoc set true，各自flush。确认A Seen0/Pinfalse/待解析目录不存在后，**仅一次真正 B.sync_now**。
   - 必须得到唯一配对A的 outcome **Err**，且保留实际cap4096KiB诊断，不能用无关dial失败当业务红；A Seen必须123、Pin必须true。A未吸收被拒devices历史/receive_dir、epoch不变；B未错误创建receive目录或更新receive_for。
2. `a_different_devices_epoch_stays_apart_but_seen_and_pins_still_sync`
   - 同样真实pair/one-shot round，B devices真实落盘 `.epoch` 改为 `store::EPOCH+1`；公开load_pile重新确认，而非增加产品测试开关。较新epoch不会被本版本load_pile自动降级重seed。
   - Seen/Pins仍同epoch，必须推进；devices setting/VV不得混入A，失败/跳过的devices不能触发B follow。
   - 按root明确决定保留现代既有 other_epoch→skip+continue，outcome **Ok**；不把本批变成新的epoch整轮失败产品政策。

当前旧现代 link 在 mandatory 非epoch错误提前 return Err，预期第一例在看到真正cap Err后 `seen=0/pin=false` 红；第二例是既有隔离行为对照。**这是源码预期，尚非运行证据。** 不以原 Q039 wire-driver 红冒充本 Node transport 红。

## 隔离、所有权与截止

- 复用原 `util::hermetic`：仅测试自己的 loopback relay，only_mine=true；assert n0关闭、relay_map只有1条，worker移除继承 KHOR_RELAY/KHOR_RELAY_ONLY，不让操作者relay环境穿入。mDNS由原util关闭，不改util。
- 用既有公开 `Node::open_hosting(..., false)` 关闭与同步无关的 vendor/process/tmux发现，仍跑真实 Node pair/serve/sync_now；不是新产品开关，也没有手写Node同步策略。
- 每个worker有自己的private HOME；移除CODEX_HOME/CLAUDE_CONFIG_DIR/XDG_CONFIG_HOME/KHOR_HOME/KHOR_NAME，仅worker环境变化。provider URL限制到loopback拒绝端口，无用户store/登录/外部provider请求。
- parent Worker 从创建scratch后即持RAII（child Option），任何spawn/断言失败都会仅kill+wait自己确切child，然后清自己独占scratch。stdout/stderr落同一自有文件，不会因pipe写满死锁；退出后完整打印日志给Cargo证据，清理不丢父Cargo日志。
- child Serve guard失败即abort自己的JoinHandle，成功主动abort+await。child独占util relay线程及其runtime，由worker进程退出回收；不修改全局util生命周期，不杀别人服务。
- 单操作90秒、child完整场景Tokio180秒、parent额外墙钟210秒。父在独立进程外计时，可打断同步栈/首次系统网络触碰卡住；不把tokio timeout描述成可取消已开始的同步Loro调用。runtime成功路径shutdown_timeout(2s)。
- 默认scratch使用std temp；正式运行应设 `KHOR_NODE_ISOLATION_SCRATCH` 到 Khor 分类证据下的scratch，两个父测试建议 `--test-threads=1` 避免无谓并行25MB导出。

## 当前固定源与轻量检查

- 新测试 SHA256 **dbfd51271ab79ad2eb1fb1fe223cacca2c152bcf1b58551da0b1e0a4afaa5c74**。暂停前最后收紧已实际保存：不再仅匹配数值4096，必须包含真实 `khor_catalog::msg::outbound_too_big(gap_bytes/1024, MAX_BYTES/1024)` 完整原因，避免把端口等巧合数字或dial失败当业务红。
- 未改生产 `crates/node/src/link.rs` SHA256 **43caada39f097f68208dfb194c10eb31aa1104bfbe537e5cc02b84c4bc12f23c**。
- Cargo.lock SHA256 **d06488f91b040ce65b0429ae6e97aefb64baa37ee7891d7ac899b8d046330f1f**。
- `RUSTUP_AUTO_INSTALL=0 rustfmt --edition 2024 --config skip_children=true ...` 格式化及 `--check` **exit0**，仅新测试，未触util；git status仅此一个untracked文件。此检查只证明解析/格式，不声称类型或运行通过。
- 没有 Cargo/Xcode/依赖下载；唯一构建槽仍为 fanout_owner ACP阶段2。最近只读容量外盘约5.2GiB/内盘7.9GiB，靠近阈值，编译前需root重新核>=5GiB，不自行清缓存。

## 排队运行命令与证据要求

worktree根运行，先固定新测试+link.rs+Cargo.lock+相关wire/store来源指纹，编译/运行期间不得改source：

```
KHOR_NODE_ISOLATION_SCRATCH=/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q039/node-isolation-red/scratch \
CARGO_TARGET_DIR=/Volumes/disk0/Khor/caches/rust/khor-quality-target \
cargo test --locked -j 2 -p khor-node --test sync_doc_isolation \
  -- --nocapture --test-threads=1
```

先只跑业务红，可在 `--` 前添加 `an_oversized_devices_gap_reports_failure_but_seen_and_pins_still_sync`，并在 `--` 后加 `--exact`；epoch控制同理。**顶层不用 `--ignored`**，父会明确启动ignored worker。

必须保存编译exit与真正父/worker执行数量、实际gap/epoch/per-peer outcome、Seen/Pin/follow断言、Cargo完整日志、前后源指纹/容量及owned PGID。不把编译错误/timeout/0测试/前置pair失败当业务红。旧link真红后才由root改生产，再以同一测试源复跑绿。

## 首轮实际编译：前置失败，不能算业务红

root 独立源审未见首轮阻断并授唯一槽；fresh清理后外盘6,726,348,800B / 内盘8,437,710,848B。固定新测试dbfd5127、link43ca、其余19项源指纹，启动 `node-isolation-red/run.py`，runner8934 / Cargo独立PGID8950，jobs2/quality-target/offline/禁止自动安装；只有oversize普通父exact selector，顶层未用ignored。

- **exit101，15.74秒**，`source_stable=true`；护栏未触发。最低外盘6,639,644,672B / 内盘8,428,331,008B；结束外盘6,839,517,184B / 内盘8,409,681,920B。
- **未进入目标业务测试**：正常 `khor-node` lib 编译时报 `crates/node/src/acp_process_io.rs:1` `use futures::io::{AsyncRead, AsyncWrite}` E0433；当前 `crates/node/Cargo.toml` 的 futures 仅在 dev-dependencies。新增integration test先编正常库，暴露已有生产依赖缺失；此前仅--lib测试可被dev依赖掩盖。不是新测试的行为断言失败，不把它称为Node oversize红。
- 证据 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q039/node-isolation-red/`：run.py、完整cargo.log、process/preflight/result.json、source-before/after.json均保留。
- 作者遵守仅新test allowlist，没有改Cargo或生产；root已收到确切报错与依赖位置。没有无谓运行epoch再次碰同一编译前置。PGID8950已结束、无worker启动，唯一Cargo槽已明确交还root。
- 待root独立修复/确认合法正常依赖基线，再保持同一test源重跑oversize和epoch。不得把此次前置失败或未来0测试当目标真红。

## 修复编译前置后：真正现代 Node 目标真红

root 将既有 futures0.3 从dev移到正常依赖，集成独立提交d20d7121，cherry到本树 **e6a2af1cc6f7496d49cbca2ae150836e4a1b9e6e**；Cargo.lock不变。测试仍dbfd5127、link仍43ca不变，Node Cargo.toml新SHA 3d42fd65cc4a530f6d3933e1d4c5ec93dbaeb010a210ca8a9b6da7509e71a9fd。首轮编译前置原始exit101目录完整保留。

root再次授唯一槽，第2轮 `node-isolation-red-02` runner12230 / Cargo独立PGID12248，完全相同oversize普通父exact命令，jobs2/quality-target/offline与容量/墙钟护栏，运行中未改源：

- **编译成功36.13秒**；真正父1/worker1实际执行，worker7.60秒、父7.67秒。总44.13秒，exit101，是业务断言失败，不是编译/0测试/pair/dial/timeout失败。
- 真Node pair完成，**合法raw gap=25,448,146B，cap=4,194,304B，epoch=1/1**；实际唯一peer outcome Err包含完整 `24851 KiB > 4096 KiB` 既有超限原因。收紧的完整cap_reason断言已通过，没有KiB变化导致的测试前置误判。
- 实际A `Seen=0`、`Pin=false`，原始日志完整记录；最终业务断言为 **Seen expected123 / actual0** 真红。一次sync_now之后没有绕路直接wire同步Seen。其前面的未错误follow（目录/receive_for不变）、设备字段不混入/设备source VV仍缺失/epoch不变断言都已通过。
- `source_stable=true`，19项source-before/after包含新test/NodeCargo/锁/link/wire/store及catalog；见 `Q039/node-isolation-red-02/{cargo.log,run.py,process.json,preflight.json,result.json,source-before.json,source-after.json}`。
- runner运行期间观测min外盘6,247,145,472B/内盘7,786,639,360B，result结束记录外盘6,461,333,504B/内盘7,786,582,016B，未触发停止护栏。但收口后的fresh df/独立shutil观测外盘突然约1,946,832,896B，已保存 `post-run-capacity-alert.json` 并立即报告root。没有在低容量下启动epoch或其他新验证，没有自行清缓存。
- PGID12248和runner均已退出，无本任务worker/serve残留；Cargo槽已明确交回root，等待其核容量增长并恢复。epoch既有控制仍待，不能把oversize真红或原研究control冒充它已运行。

## 同源已编 binary：epoch既有隔离控制通过

root将容量APFS/statvfs连续fresh核回安全线后授一次同源binary运行，不重复Cargo重编。直接执行red-02编出的 `/Volumes/disk0/Khor/caches/rust/khor-quality-target/debug/deps/sync_doc_isolation-526aa197410dcd89` 的epoch普通父exact selector；19项source-before严格等于red-02编译后的source-after，binary SHA256 **7246047fe8b85eb085b7512576f84ae9ab1c37144c285efa6ac2876dde0f6d2a**、size192465472、mtime均保存。

- runner22296 / test独立PGID22371，**exit0，3.63秒**，真正父1/worker1各通过；不是0测试或只手动起ignored入口。
- 真实pair后 devices epoch **2/1**、rawgap2429，唯一peer outcome **Ok(216 bytes)**；A Seen **123** / Pin **true**；devices setting/source VV不混入、receiver epoch不变、B未错误follow全部通过。
- `source_stable=true`、`binary_stable=true`；最低外盘12,375,801,856B / 内盘7,781,892,096B，无资源护栏触发。
- 证据 `Q039/node-isolation-epoch/{run.py,test.log,process.json,preflight.json,result.json,source-before.json,source-after.json,binary-before.json,binary-after.json}`。没有Cargo调用或源码变化。
- PGID22371已退出，无本任务worker/serve；已明确交回唯一Cargo/native槽和生产link写权。先前APFS容量波动归属未查明，root没有归责本任务或其他agent；只保留实际观测与暂停边界。
- 新测试独立提交 **2a11f8d4f6fa5c3b6b132bcf91f921460f2edfdf**，仅一个sync_doc_isolation.rs，SHA dbfd5127不变。这笔记录原生产真红+epoch对照绿，**不是生产修复已绿**。

## root生产候选独立只读源审

root在HEAD2a11f8d4上独占修改link.rs，123加/149删，统一sync_document/sync_documents。作者按review-changes先minimal context、detect/flows/impact/tests_for；CRG当前link仍无节点/0flow且索引HEAD mismatch，不能据此认为无风险，转现场diff/macro/rounds/known_wide核对。

审查固定link SHA256 **5d51f7260237bbb302db4ca349e36330fc1d47687175b82221cbf6d4eaa06c3d**，前后相同，没有改源或编译。未发现阻断项：

- with_doc所有加载?落在sync_document内独立async块，嵌套Result将Load与round/Exchange分开；?不会逃出聚合循环。Load包括optional仍hard failure，optional Exchange保留原best effort，首个hard failure不被后续doc/readings refresh覆盖。
- modern Known other_epoch保持skip，legacy Bootstrap mandatory other_epoch仍Err但继续后续doc；devices仅成功follow，保留legacy成功0也follow、modern成功>0才follow。broadcast take_arrived仍仅modern成功>0；legacy无此新增副作用。
- ROUND_DOCS顺序与各legacy reading arm保留。modern部分推进也刷新readings，refresh失败保留此前首错；legacy先完成所有doc/reading后finish，不能假报全轮成功。
- 低风险时点差异已告root：legacy现在先枚举known_wide，旧版在global docs完成后才枚举。known_wide仅扫描磁盘目录，不依赖roster；固定输入无回归，但并发新目录可能晚一轮才进入此次legacy同步。root可保留或恢复原枚举时点，本审查未擅改候选。
- 仍缺真实legacy拒Versions+authenticated wire::answer运行、optional Load/远端拒绝分类与first-failure/refresh失败顺序的故障注入。两个真正Node例只覆盖modern，不能把共享helper源审写成legacy兼容已实测。建议同源modern红→绿后补legacy门。

## Legacy fallback可行性评估（未实现、未运行）

公开 `khor_net::identity::load_or_create` 可重新打开测试自己的A identity.key；可在真实pair后，用同authenticated endpoint identity和只放行真实B remote_id的受控旧端transport，拒绝 `Request::Versions`，对 `Request::Sync` 调真实 `wire::answer` 的公开Doc/BlockStore，测真正客户端 `sync_everything`。不能仅再抄一个driver函数来证明Node策略。

仍需解决旧A resident完整退役/端点换代（不能同时同key监听）、真实端点addrs更新及Vouch/基础reading答复、server每连接task RAII与timeout，并明确只控制旧端协议能力，不伪装完整旧Node。现代真实Node红优先，这条另作第二阶段测试，不把当前两例当legacy兼容已证。root可据实际现代红后安排。

## 最新交接

当前 `fix/quality-sync-chunked` / **2a11f8d4**；新test已独立提交且冻结dbfd5127，原始有效现代oversize红与同源epoch控制绿均完整。root独占未提交link候选5d51...，作者已完成独立只读源审暂未发现阻断，未改生产/未重编/未重复原始红绿。唯一Cargo/native槽已交回root，由其协调ACP与生产修复后测试绿。作者只更新本卡，STATE未碰；本任务没有Cargo/worker/serve进程。后续可根据root安排补真实legacy能力fixture，但当前未实现/未运行，不先声称兼容闭环。


### 最终legacy boards时点微调的独立复核

root将仅有的低风险时点差异恢复：先ROUND_DOCS global同步，再调用known_wide扫描并同步boards；新增SyncProgress::merge累计moved，并通过fail/get_or_insert保留已有global首错，无已有错时才取boards首错，reading顺序不变。作者已只读核对当前 **HEAD8d87af99fb97da3d3d61435df82dcd7bae3c5eef** tracked clean、link SHA256 **62ebbe6b46fb29dc52a832d406553a0c840c97c9758cfb4075525d9ac378ea1e**，未见阻断。没有改源、编译或重复原红/epoch绿；该确认是最终tiny差异源审，不冒称新的真实legacy运行证据。

## root：最终生产绿与集成（2026-10-26）

生产最终 `link.rs` SHA62ebbe6b46fb29dc52a832d406553a0c840c97c9758cfb4075525d9ac378ea1e。
最后一个独立审低风险点已恢复：legacy global docs 运行后才枚举 known_wide；merge累计moved、保留首个hard failure。
现代epoch隔离、optional exchange best effort、optional load failure、follow/take_arrived成功条件及读数刷新时机保持原契约。

`Q039/node-isolation-green-01`：普通父2/2、由父启动的专用worker各1/1，exit0，48.433s含编译，
source_stable=true，无护栏触发。oversize合法gap25,448,154B，实际超限Err仍保留，A Seen123/Pintrue；
epoch控制仍Ok且独立文档推进，失败/隔离devices没有误follow。测试源dbfd5127未改变。

生产提交8d87af99，test2a11f8d4；native_helpers最终只读核62eb+legacy枚举/merge无阻断。
root已分别cherry进wave1为0da50818/8f55002b；该树当前HEAD8f55002b clean。
这是治理集成，不是main合并/发布；精确集成版本实际回归还需在唯一槽释放后执行。

此批仅关闭同轮独立文档饥饿故障，不关闭Q039整体。超过单帧的devices本身仍不能收敛，
大差集分块/ACK/落盘/公平调度及真实legacy拒绝Versions协议回归仍待。
