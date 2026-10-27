# 当前代码治理状态

更新：2026-10-28。完整目标 active，未完成；当前在 P5 真实 App 验收准备。继续先读本页和 [PLAN](PLAN.md)，核实际 refs/diff/agents/PID。此前详细记录保留在[历史 checkpoint](history/STATE-before-real-app-acceptance-20261028.md)；历史 handle 不得重启。

## 用户边界与当前源码

- Clean Code 贯穿命名、职责、共享规则、状态/资源归属、副作用和失败语义，见 [ENGINEERING](ENGINEERING.md)。格式只是机械小项；必要协议、并发和平台注释保留。
- 用户决定：**开始了的做完，新独立问题只记录下一轮**。不追加新模块、框架或测试矩阵；既定验收必要的最小接线继续。
- 未上线，统一当前协议；认证、epoch、完整性和实际 ACP 能力保留。
- **无 main 合并/ship 授权**。main `/Users/zgh/Desktop/working/khor@8704a503a27b36ad58f922690de50e766f8bd485` 与用户 App/服务/release 不动。

集成 `/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`，`integrate/code-quality-wave1@495d096d54ea083304031423ca052f87dd9bc75e` clean。完整 2388 输入 frame SHA `36f15241f5524d1e900d9b160087eb26b512a5db3f217c0713a869500efaf07e`，含 gen157/dist277/模块锁。生产与21b1一致；56bde只有原030失败 snapshot diagnostic，495d只有 Peer fixture 精确 Busy→Refused；全部原测试体不变。N2作者130fb、R2作者828f、退出作者3707均已集成，恢复仍核真实 refs。

## 唯一执行槽：正常 GUI build 已启动

**root 是唯一 native owner，当前 session23822**，执行 `P5/Q033-quality-gui-build-plan05/run.py --execute-authorized`，OUT `P5/Q033-quality-gui-build-run02`、TMP `/tmp/kgui34`。不并发 Cargo/native、不启动 App/服务；旧 run08 session74788 已 exit0/完整归槽。新 controller PID 以现场为准；缺失 handle 先核现场不重启。

GUI05 runner `ffdc80058ae0b4007eed4dbe4007865ab64f5534195acb36dca8e9a2b3b77266`；data `92bcbe88327bcedae255452d66cb4daa420d20d261f12dabf27ef833876a736e`。沿已独审 runner 逐字不变，此前4个data来源字段刷新495d/36f152，当前仅3个OUT/internal/TMP字段改新run02，root核精确delta；runner/源/配置/profile/时限全部不变。单正常 dev/no-default/devtools bin，非test、opt0、freshfalse；真实 Pi bundle af955、quality JSON 1d16、REMOVE_UNUSED_COMMANDS显式unset。入口恢复6582原pins，外层PATH去掉恰好首个toolbin前缀，原runner prepend后全文一致。

正常GUI run01 session22658已exit1：137.116s时外盘低于5GiB门而有界终止编译，实际未产GUI，非Q055结果；source/gen/dist/helper稳、20dual/owned全退/TMP清，root已独核，resulta1c848802844032df257d6d13e8e69ed337843b6bb23727fcbaaa4f28631b823。原nongreen保留，不改门/时限。

为完成既定构建，仅可逆迁移旧cache：P5/cache-offload01（local旧profile5954文件+4旧GUI exe）及02（2旧iOS static archive），逐file原stat/SHA/恢复路径、copy2后完整核验再移除确切原cache；源码、normal dev依赖、current CLI/core和所有证据保留，非global clean。备份在内恢复cache-offload-20261028-01/02，metadata双盘；02两完全相同备份已硬链接去重，源原stat仍在plan。旧local/旧iOS/cache artifact路径不再直接可用，需先按清单恢复且不能假定inode证明仍有效。实际外盘约7.66GB、内盘约7.66GB后才启动新GUI05；正常资源门保持。

**Q055已知限制保持**：自动来源 helper 未覆盖 TAURI_CONFIG/REMOVE_UNUSED_COMMANDS，必须保留实际 verify 异常与 `provenance_complete=false`，整体 nongreen/exit1。正常 build、实际 artifact、source、cleanup 可分别独核；不得伪造 accepted receipt 或为此修 helper/新开工具专项。其他失败立即停保。

## 既定组合验收：115项全部已执行

原115父精确集合：**113通过，040与062两父失败登记下一轮，0未执行**。不声称整体全绿。证据按确切版本保留，helper不充父，旧失败不回填。

- 40已过的旧fixture记录带独审影响映射复用：guard4、同生产21b1 run03的28、原030 diagnostic一次通过1、run04的7。其中8个Peer父都要求finish成功，旧Node::open?任一错误会使finish失败，因此它们未命中新Busy分支；成功dispatch/events/hold/reply与全部测试体保持，immutable home校验等价。不能称40已在新helper重跑，原030 Query异常未被产品修复。
- `receipt-peer-busy-run01` 原038/039完整强业务断言实绿：2父/168.560s/result `61e8755bae26bc6d3b9219fc5892dbdcb8510247320ddba42d88b41445ede8f3`；26dual、source稳、114组160owner全退、TMP kpeer40清。root和非作者审查后以同blob495d提交；无sleep/retry，所有其他错误硬失败。
- `integration-affected-combination-run06` 五必要producer均0/verify、原040 deadline强断言红：result `66c55f9e4f1809272562634550825af321927b4ad4c5c5595b7fa983f8d254b7`，raw `2a0093a1eb5ad2ca71491f51d9999015585c0f040e663256dd0fdde95e01d8c5`；51dual/232.532s/source稳/50组101owner全退/TMP kcomb41清。raw无at/due，未定位产品/夹具/时钟原因，不诊断或重试变绿。
- `integration-affected-combination-run07` 原041–061的21父/24positive summaries过，原062 helper在runtime_close_tests1051 Node::open Busy→父789 restart失败：result `1df41700a82bb2e77fe168c75454c0fbddbeebffdb5b3003a1039f8a0d9e9ed5`，raw `c73e8a3aa2fcd142206911c302318cce10a9179565fdd7f40e7f1fe1c4f0038d`；57dual/47.590s/source稳/99组402owner全退/TMP kcomb42清。该父归既有Node打开准入候选下一轮，不新增修复。
- `integration-affected-combination-run08` 最后50父/53positive summaries全部过：result `b5043df74c80a22fa4e2be0049939ad891df89590a12ffd3348d33644e825bd5`；85dual/157.487s/source稳/owned全部退/0forced-guard-cleanup错误/TMP清。root逐raw核原summary、完整集合115=113+2，并在GUI前独核7receipt selected的16个产物文件实际SHA/全stat。该controller只允许已完全清理的真实exit101失败记failed继续，整体无failed才green；本次failed为空，f7/fcb不变、每父只一次、成功断言逐字保。

当前7来源（GUI可合法覆盖共享依赖，不能事后回填旧来源）：Node/CLI来自 `P5/receipt-peer-busy-run01`，SHA分别 b99f629626ebfaa63535932999287b3e4b628aa49c37199202848377037e1bec / 22fd52264cdb2ee4c16a7df2e15fe4e961ba98f1b6798d2f1b75ee2b00e28df7；stub/A2A/wire/core/pairing来自run06，全部实际verify+run08 require通过。完整paths/hash在plan08 data与run08原件。

33c8基线另外4个原大数据网络父均实绿/各22dual/source与owned-TMP清：Pair29,223,902B/原90s父19s；N0 25,446,089B/visit1约10s；两32MiB父14.688/17.023s、actual=charged33,031,394/33,030,354B。新生产两源只初始化guard，数据热路径未变；保影响映射，不冒旧binary已覆盖新bootstrap。21b1新入口另有原wire、Pair真实SIGKILL恢复和io发布/拒绝4父实绿（guard-run01整体曾因fixture晚写保cleanup非绿，独立postcleanup已归档，原件不变）。详历史checkpoint。

## Clean Code与工程成果

18c0a9d1用请求类型和Issuer/Joiner明确Pair角色，删废分支、共享固定base纯规则；3e5e25a4共享精确input/sticky和private setup终态规则；357c7070让UploadRelay集中拥有fixture线程/socket/gate及有限清理。均独审、保布局/认证/副作用顺序；不能把fixture整理称整个产品架构重构。当前组合结果如上，Q033真实App仍待。

d26e608f仅216 Rust机械格式；33c8bc12把共同检查放main/显式root、默认check/显式write，受限import探针0副作用。450第一方check0、392第三方SHA稳；后续实际改源单独同版formatter check0。见[Q053](tasks/Q-053-format-baseline.md)，不重复格式迁移/不把它代替结构成果。

21b1恢复/register_self/bootstrap共持同一现有root fence，bootstrap借guard、返回Node前释放；不跨await/不新锁/不重试，Ready/坏next/认证不变量保。495d仅测试协议适配，不是Node并发准入治理。

## 接下来：既定真实 App 验收，尚未执行

1. 核当前GUI04实际build/artifact/source/cleanup与已知自动proof缺口，归槽。
2. 其后执行 `P5/Q033-core-helper-producer-plan03`（9dual），runner与已审01逐字同，只更新来源和最新run06/core prior `1b73ca9c38ba6bac4b1b2cc53758cf14fb1f34344a37ed007e33fc20093cf21a`。data SHA `2d757278b098423ac599c9be28de107ffeec9824418ce76971289f1166473105`。按原GUI→helper顺序，标准无embed环境原prepare/verify/require，必要单目标构建，不能假定旧core仍有效；GUI bin独立SHA不变。
3. `P5/Q033-real-app-owned-plan04`执行真实Q033：env-i私有HOME/ROOT/TMUX/cwd，无SIDECAR/凭据/GHOST；真实native/wire/ACP phase快照，4实际PNG由CUA选择，A上传时Stop→B相同草稿/附件重发Allow→smallC Stop→实际mode/option更新→原raw oracle确认→CUA正常Quit→host stop/finished/observer有界退。原helper270/firstPart30/外300门保，人工accepted不代替业务oracle。
4. 同一正常artifact完成Q035实际page/双私有peer借网/最后owner释放；Q043真实OS拖放、全屏、window-close、正常Quit。私有relay须两新Root首次open前非空only_mine配置；仅KHOR_RELAY仍追加public tier。Q035原4 native程序化资源场景使用已审端口替换plan01，单列程序化证据。
5. 任务卡/STATE/AUDIT末尾按真实结果对齐，保留失败/未验/旧版本复用边界；Q010/011真实iOS/Safari已有确切版本与影响映射，不新开物理机/IME/Safari矩阵。无main/ship。

真实UI统一CUA。此前只有只读inventory；当前turn已恢复CUA文档，尚未启动App。继续UI前若上下文压缩，先rewriteDocumentation；先listApps确认已由owned launcher启动，再getApp，避免工具自行无隔离启动。

## 资源、恢复与下一轮

唯一target `/Volumes/disk0/Khor/caches/rust/khor-quality-target`；jobs1/inc0/lockedoffline/短私有TMP/双盘各>=5368709120B/actual源工具产物/有限owned PID-PGID全成员/TERM-KILL-reap/dual。常规300step/720total/default512；历史冷local门仅登记，不放宽产品30s/90s。禁global clean、重放旧cleanup、碰用户进程。

工具链事务已complete，不重复恢复/rustup升级/增组件。真实sysroot `/Users/zgh/.rustup/toolchains/stable-aarch64-apple-darwin`，Rust1.95.0/LLVM22.1.2/rustfmt1.9.0；真实Cargo/RUSTC/RUSTDOC与绝对RUSTUP_TOOLCHAIN/AUTO_INSTALL0。精确原env/PATH取 `Q039/integration-bootstrap-guard-run01/toolchain-pins.json`，SHA6582c4471645016dfe900e561685bf29c3d605d8d9e0f52be8797fbd0cf79533，不重复prepend。fcb模块/f7entry固定不改。

权威为本目录；外证据 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`，内恢复 `/Users/zgh/Library/Caches/khor-code-quality/recovery`，掉盘读RESUME/逐SHA镜像再核现场。下一轮见[AUDIT](AUDIT.md)：040 deadline、Node打开Busy/fixture晚写、Q055、原setup owner gap等；Q041d fresh-onset、remote lease、Linux/musl/真实SSH等原未实施项不变成本轮新前置。
