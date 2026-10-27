# 当前代码治理状态

更新：2026-10-28。完整目标 active，未完成；当前在 P5 真实 App 验收。继续先读本页和 [PLAN](PLAN.md)，核实际 refs/diff/agents/PID。此前详细记录保留在[历史 checkpoint](history/STATE-before-real-app-acceptance-20261028.md)；历史 handle 不得重启。

## 用户边界与当前源码

- Clean Code 贯穿命名、职责、共享规则、状态/资源归属、副作用和失败语义，见 [ENGINEERING](ENGINEERING.md)。格式只是机械小项；必要协议、并发和平台注释保留。
- 用户决定：**开始了的做完，新独立问题只记录下一轮**。不追加新模块、框架或测试矩阵；既定验收必要的最小接线继续。
- 未上线，统一当前协议；认证、epoch、完整性和实际 ACP 能力保留。
- **无 main 合并/ship 授权**。main `/Users/zgh/Desktop/working/khor@8704a503a27b36ad58f922690de50e766f8bd485` 与用户 App/服务/release 不动。

集成 `/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`，`integrate/code-quality-wave1@495d096d54ea083304031423ca052f87dd9bc75e` clean。完整 2388 输入 frame SHA `36f15241f5524d1e900d9b160087eb26b512a5db3f217c0713a869500efaf07e`，含 gen157/dist277/模块锁。生产与21b1一致；56bde只有原030失败 snapshot diagnostic，495d只有 Peer fixture 精确 Busy→Refused；全部原测试体不变。N2作者130fb、R2作者828f、退出作者3707均已集成，恢复仍核真实 refs。

## 当前执行槽空：真实App原流程待完整闭环

**当前无native/Cargo/App/服务，App run03 session77772和run04 session31945都已exit1并有限清理；不能重启旧handle。** GUI/core实际producer仍如下，未重新编译。真实Q033已执行两次，原结果均非绿：
- run03 278.611s：系统文件选择/多次CUA往返耗时，helper270.07s deadline101；A23MB首Part→真实Stop/显式release→原稿4图恢复，checkpoint-a已捕；B/C/normalQuit未达。首次会话open Busy→UI Retry成功，原4图并发put两rejected，经UI补齐；保真实问题归已有Node打开准入下一轮。source/helpers/dist/bundle稳、guard空/owned和TMP清，未伪造accepted。
- run04 277.263s，resultaf91822399463c5e69450ddb5ec8651ca71e9fadd4722fcbf5dc1a70379ce373：四图逐张正常加入；A/B同547B文本SHA与四PNG hash/order完全相同，A取消reconnectfalse/无commit；B新tokencommit→真实Allow/chat_answer→EndTurn、checkpoint-b恰1prompt；C无附件小消息commit→Ask→Stop/Cancelled；option/model two成功，新Settings实存，单连接。mode与最终oracle/正常Quit未达，helper270.06 deadline后原outer清理，source/helper/dist/bundle稳、guard空/owned-TMP清。独立app_runtime_review逐raw核过，不能称完整成功。

为纠正执行层约50次工具往返超时，仅root CUA把已观测确定的文件选择步骤在单调用内串行执行，每一步都fresh AX唯一真实元素定位后click/setValue/pressKey；不使用driver UI脚本、DOM/IPC伪操作，不增加原270/30/外300时限。`P5/Q033-real-app-owned-plan06`6dual，run/launcher与04/05逐字同、data72850c645e8ee0ff3eccd939842665dd754712802c825d24a91c5655b309b0b7仅4路径key改新run05/q033ui5，root和app_runtime_review短审通过，可仅此接续一次完整原流程。原capture A/B/最终raw与CUA正常Quit全保，真实producer五SHA齐。下一次直接该计划，不重编/重跑115/改源或加时。

core-helper-run02 producer已完成81.979s/24dual/来源true/所有owned-TMP清，result89715726663ccd10094d09039c62232e556725c58c1541a5cab94a79a00e0d4f，core-provenance8115c4d2b35760839818aa80e8b0f9b1a8629954689d7423455126e195b12d2b；exe chat_upload_cancel-a98ef2c294b70f0c SHAfa77eac90f78bc7df46506d24a982bda54c42d9aa3051756cf25cd924d8d0552。原1320已exit0。

core04与已独审03 runner逐字同，仅3个OUT/internal/TMP字段改run02；data8b40397ff350355ffcc40caf6f84a8e741fc3129eec66333131dde204816688e。标准无GUI embed环境、真实当前来源，不能将GUI自动proof缺口传播为伪core receipt。

**无需再规划或编译。下一步用已短审 `P5/Q033-real-app-owned-plan06/run.py` 完成本页上述原Q033。** 五个必需实际producer path+SHA已在双盘 `P5/Q033-real-app-actual-producers01.json` 逐项固定（status明确App尚未启动）。恢复6582五pin，外层PATH仅去掉恰好首个toolbin:，原App04 prepend后等于core实际环境；原外层自己设置CARGO_*并清runtime helper变量，TAURI_CONFIG/REMOVE/KHOR_AGENT_BUNDLE保持unset。先CUA文档/只读inventory准备好，再按既定唯一owned launcher启动；不要由getApp先行无隔离启动，也不重跑原115/构建或重启旧handle。

## 正常GUI已产出，空间中断原件保留

`P5/Q033-quality-gui-build-run02` 已正常完成（session23822 exit1仅自动proof拒绝）：metadata/build均0，102.620s，29dual、source/gen/dist/helper与bundle稳、owned/TMP清。root独核实际selected freshfalse/testfalse/opt0/features仅devtools、当前manifest/main.rs，GUI executable SHA **f76302af7b61b59c1d01172ee4209c1c9cb6c80bd6381ee760468527624c865d**、374,932,072B；result SHA **ec56eb347c058b1a167e5fd8807fdd7cf1a57891cc93cae9e6d3f1019385e2bf**。实际选择/完整stat见 `gui-selected-actual.json`。真实App已实际执行，完整闭环尚未通过，见上。

**Q055保持非绿**：实际verify拒绝 `Build script names an unrecorded environment input: TAURI_CONFIG`，provenance_complete=false/verification_status=rejected/整体nongreen，normal_build_completed与artifact_checks_passed均true。未伪造accepted receipt/修改prepared或helper。GUI05静态data92bcbe88327bcedae255452d66cb4daa420d20d261f12dabf27ef833876a736e，runnerffdc8005…77266与已审GUI04逐字同，只3个输出路径改run02。实际启动PATH去掉6582首个toolbin前缀，原runner prepend后全文一致。

两个受控中断原件不回填：
- GUI-run01/session22658，137.116s因外盘低于5GiB终止编译；未产GUI，source/gen/dist稳、20dual/owned与TMP清，resulta1c848802844032df257d6d13e8e69ed337843b6bb23727fcbaaa4f28631b823，非Q055 verifier结果。
- core-helper-run01/session92324，39.499s因内盘低于5GiB终止；未完成producer或provenance，21dual/source/helpers稳/owned与TMP清，result9045587e03cb5a31231bbecdce33ad98dfea13e67370a218e295042524658b29。迁入内盘的备份挤占编译余量，root先压缩后按原门重新启动，未降低门/加时。

缓存空间准备只属既定构建：offload01旧local5954文件+4旧GUI exe；02两个旧iOS static archive；03三个已完成functional test exe（Node cfgtest/wire/pairing）。源码、normal dev依赖、current CLI/core/GUI与全部证据保持，非global clean；逐file原stat/SHA/restore路径、copy后校验再移除确切旧cache。**原raw备份目录已由逐entry SHA核验后的ZIP替代**：内恢复根 `cache-offload-20261028-01.zip`（1,304,712,152B）、02.zip（503,951,410B）、03.zip（188,683,166B）；各SHA与恢复方式在双盘 `P5/cache-offload0N/compressed-backup.json`，原stat在plan.json。历史cache路径不再直接可用，恢复bytes/mtime不恢复inode或自动证明旧receipt当前有效。压缩时无验收进程；恢复两盘原空间门后才启动core04。归档session37388已exit0/62.394s，内盘约8.75GB。

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

## 接下来：既定真实 App 验收收口

1. GUI-run02与core-helper-run02均已实际完成并独核归槽，不重编译。Q033 run03/run04均已真实执行但未完整通过；下一次使用已独审plan06，输出新run05。
2. root CUA逐步fresh AX批量执行确定的文件选择，减少往返；原helper270/firstPart30/outer300及全套原raw oracle保持，mode和option、正常Quit均需实际完成。
3. 完成A上传中Stop/release→capture-a→B同稿四图Allow/EndTurn→capture-b→smallC Stop→mode bold/model two→最终oracle ack→CUA CmdQ→accepted。失败保原证据，不用人工marker替代业务判断。
4. 同一正常artifact完成Q035实际page/双私有peer借网/最后owner释放；Q043真实OS拖放、全屏、window-close、正常Quit。私有relay须两新Root首次open前非空only_mine配置；仅KHOR_RELAY仍追加public tier。Q035原4 native程序化资源场景使用已审端口替换plan01，单列程序化证据。
5. 任务卡/STATE/AUDIT末尾按真实结果对齐，保留失败/未验/旧版本复用边界；Q010/011真实iOS/Safari已有确切版本与影响映射，不新开物理机/IME/Safari矩阵。无main/ship。

真实UI统一CUA。此前两次实际App流程保留失败证据；当前CUA文档已恢复。继续UI前若上下文压缩，先rewriteDocumentation；先listApps确认已由owned launcher启动，再getApp，避免工具自行无隔离启动。

## 资源、恢复与下一轮

唯一target `/Volumes/disk0/Khor/caches/rust/khor-quality-target`；jobs1/inc0/lockedoffline/短私有TMP/双盘各>=5368709120B/actual源工具产物/有限owned PID-PGID全成员/TERM-KILL-reap/dual。常规300step/720total/default512；历史冷local门仅登记，不放宽产品30s/90s。禁global clean、重放旧cleanup、碰用户进程。

工具链事务已complete，不重复恢复/rustup升级/增组件。真实sysroot `/Users/zgh/.rustup/toolchains/stable-aarch64-apple-darwin`，Rust1.95.0/LLVM22.1.2/rustfmt1.9.0；真实Cargo/RUSTC/RUSTDOC与绝对RUSTUP_TOOLCHAIN/AUTO_INSTALL0。精确原env/PATH取 `Q039/integration-bootstrap-guard-run01/toolchain-pins.json`，SHA6582c4471645016dfe900e561685bf29c3d605d8d9e0f52be8797fbd0cf79533，不重复prepend。fcb模块/f7entry固定不改。

权威为本目录；外证据 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`，内恢复 `/Users/zgh/Library/Caches/khor-code-quality/recovery`，掉盘读RESUME/逐SHA镜像再核现场。下一轮见[AUDIT](AUDIT.md)：040 deadline、Node打开Busy/fixture晚写、Q055、原setup owner gap等；Q041d fresh-onset、remote lease、Linux/musl/真实SSH等原未实施项不变成本轮新前置。

## 2026-10-28 失败运行重复产物归档

run03/run04双盘原manifest所有条目逐SHA核过，四份同SHAf763/374932072B的历史staged GUI executable已可逆归档，原日志/results/manifest及其余bundle文件保持。ZIP在内恢复根q033-historical-staged-gui-f76302af.zip，102641293B/SHA8a9bf6443392b6a2e71faf473c069bcab8d00211349f997faf2ccaba62f56c30，恢复索引双盘P5/Q033-historical-staged-gui-archive01/archive.json。恢复原4路径前，历史全树manifest不可直接声称当前完整；bytes/mode/mtime可恢复、inode不可恢复。归档后外盘6479036416B、内盘9085059072B，未降低5GiB门。

发现工具在run04已清理后重新唤起私有历史App37569（05:27:43/ppid1，原owned App为30087）；root已CUA CmdQ并ps确认缺失。它不是原流程正常Quit证据，未回填run04结果；无用户App被退出。避免旧handle再次访问，第三次只绑定owned launcher已启动的新run05精确App路径。
