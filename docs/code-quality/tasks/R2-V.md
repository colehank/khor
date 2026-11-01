# V 构建来源与OS验收契约

状态 **done（既定R2-L04、R2-L05）**，2026-10-29最终独审核验通过。Root集成ca83的实际GUI/CLI/core严格构建与focused L05正常退出/持久宿主认证重连均成功；只关闭这两项，不关闭全轮、不冒完整Q043/IME/编辑器guard/GUI重启恢复。下文旧working/无权限/PID均是阶段历史，当前执行读STATE。

## L04 已有准确定位

当前integration 8f9506d3的 scripts/native_provenance.py::environment 在66行附近，显式names与CARGO/RUST/KHOR等前缀均未覆盖TAURI_CONFIG、REMOVE_UNUSED_COMMANDS。原实际Tauri producer已声明这两项，verify_build保守拒绝是既有证据，不得删未知env拒绝或回填旧run成功。

限定下一实施：将这两个已证输入（包括未设置状态）纳入哈希化环境快照与producer key；新增必要回归证明设置/修改/移除均失效、同源稳定，并保持未知rerun-if-env-changed拒绝。仍不打印配置原值。真实整合App build后用原strict verify验证，不复制构建系统、不清空target、不重跑无关业务矩阵。原Q055详细过程见Q-055卡，只有本缺口待本轮处理。

## L05 范围保持

OS退出harness需区分GUI拥有的临时进程与应跨GUI退出保留的持久宿主。调整归属/退出契约前核真实脚本和当前产品生命周期，不把仍活着的用户agent误当泄漏并kill。正常GUI退出及重连/恢复验收保留，不能以忽略所有进程存活追绿。原失败独立保留；实施时先给具体owned PGID/host identity/cleanup语义及最小验证。

L05只读定位已完成（完整绝对artifact指针见R2-REVIEW的V段）：旧launcher把全部观测App后代都要求随CmdQ消失，误包括已证明的私有持久Shell host/child。修正应先按private home+sessionID+host启动身份/子树分类，Quit门验GUI临时集合终止，另验持久者活且可重连；accepted独立检查，fixture结束再显式清理自己私有host并验无remaining。不能忽略全部后代/杀真实用户agent，也不能声称原10s末唯一阻塞PID（旧log无末态名单）。本定位未改harness/未跑App，原失败保留。

## 资源与完成边界

唯一native槽、共享target、jobs1/incremental0、外盘TMP沿执行计划；源码/生成输入/产物真实指纹与accepted run关联。纯Python检查只能证环境key规则，不能代替实际Tauri producer。main/ship未授权。


## L04 已固定（待实际producer复验）

生产＋测试 `959cf6b837498fac22b673467bd3468963a6a564`，仅 `scripts/native_provenance.py` 显式names集合加 TAURI_CONFIG、REMOVE_UNUSED_COMMANDS，未设置仍记录key(None)，继续只哈希原值。未知rerun-if-env-changed拒绝保持，未改全局输入/复用策略。

新增3个Python测试先红：两个缺key、六种set/change/remove producerkey不变，以及真实generated_output拒录TAURI_CONFIG；原3测试6FAIL/3ERROR/exit1保留。最小一行生产后整25项/.200s/exit0，包含旧未知env/file拒绝；不打印实际env原值。独审无阻断。收据 `/Volumes/disk0/Khor/evidence/round2/validation/L04/receipt.json`，red/green.log单存。source SHA4de2d3547378deb9393e4924bccef8e2973d10d16a1cac47e479f5086bfde0bd，test SHA9144616e6a32dfc49b37ac2dc1a5b9a8fc34e73244a4cec8ba47da0247d66495。没有运行Cargo或Tauri，实际producer/strictverify由后续冻结集成源验，不计L04原生完成。

## L05 helper已固定，caller离线准备（未实机）

固定 `e92893ce5cf8439012cb69dc8be68f1de519e9dd` 两文件：scripts/run-node-acceptance.py 与 tests/test_node_acceptance.py。persistent_session_owners 只接本次freshfixture正规home/session路径、meta显式SID/pid、host marker PID、owned/current出生身份/可执行与host独立group/child parent，沿已证parent链取后代。remaining_temporary 先运行原strict remaining(all)，只扣proof中相同PID/start；未知同PGID、同名临时者、复用PID仍阻断。原remaining/stop/run语义不动。6项私有文件/受控process表＋原2项真正自有临时shell/sleep清理共8绿(.669s)，未运行产品native/App。

原旧caller `launcher.py:232` 的真实lambda对受控persistent-only postQuit表仍False，caller-predicate-red.json独立保留；classifier-red仅新helper API不存在的开发红，不冒原产品OS复现。旧snapshot和旧run失败未改原字节，SHA均核原副本一致。

root批准新 `/Volumes/disk0/Khor/evidence/round2/validation/L05/launcher.py`、observer.mjs、test_launcher.py；仅离线准备。launcher pin自身/helper/observer及host/proto/shell_start/core term四协议/Ready源SHA；freshrun先resolve并持dev/ino。observer仅旧allowlist加term_open/term_ready，不增正文抓取。preQuit按proof SID核成功open→Ready API完成→fixture drop；Ready只API事实，认证replay marker另证host内容。Quit原strict结果减精确持久proof；accepted仍独立；postQuit复核samebirth和单份host marker原bytes SHA，再当前v5 Hello/Version＋15s总deadline/1MiB raw replay匹配显式既有fixture marker。无Input/Ready/agent prompt；Version会走resize分支，不宣称零副作用。正常成功须显式清自己的persistent后App/helper/observer原strict all空，finally也只收已证privatepersist并报remaining。

离线test_launcher 11项绿(.011s)：8假socket/顺序合同＋3实际新caller lambda门（persist-only通过、未知同组/复用PID挡住）。AST和observer node --check绿。这不是实际TCP、重连内容或GUI退出验收。收据 `/Volumes/disk0/Khor/evidence/round2/validation/L05/receipt.json` 串2 helper源、5 caller/原副本源与4协议源SHA；必须用新字段launcher_sha256、entry_sha256、observer_sha256、host_protocol_sources与persistent_replay_marker生成冻结contract。不得复用旧contract/binary冒新来源。

独审helper/caller初无阻断，最后self pin/单次marker bytes与3实际lambda检查后只补固定SHA核，不扩离线矩阵。下一动作由root集成工具并冻结完整source，另授实际native build/strictverify与freshfixture实机验收；当前无App/observer/真实socket启动权。W/23837继续冻结，V后按root调度回W，不关闭全V或第二轮。


## Native 首次strict实际运行：catalog graph拒绝（停留，未执行App）

root冻结集成0a885e25并授唯一native槽后，按同env实际链启动 `validation/native-0a885e25/build.py`（driver42159、exec94753）。metadata阶段成功，catalog Cargo42249正常exit0；strict verify于5.690s拒绝：`Actual first-party rows do not cover the prepared build graph`。metadata按package graph把catalog的dev-dependency khor-core也纳入selected；普通`cargo build -p khor-catalog --bin gen_ts`只实际生成catalog build/lib/bin三行，未有core行。没有伪造行或放宽proof，没有继续gen/frontend/App。source/environment稳定、owned进程剩余空；本次空TMP已单独安全移除，原result保持false，cleanup-after-stop.json另存并镜像。

已报root下一最小命令修正：对相同保守graph用实际`cargo build -p khor-catalog -p khor-core --lib --bin gen_ts`，使要求的core lib也真正编译；新run目录保留失败，不改source/proof/target。原日志/commands/prepared/result均位于native-0a885e25，内盘small receipt mirror齐。native槽仍由本作者持有等待这个invocation修正，未宣布释放。


Native第二次run已按root批准启动：`validation/native-0a885e25-2/build.py`，driver45208、exec94911，初catalog Cargo45267。仅实际命令增加`-p khor-core --lib`覆盖prepared保守graph，明确不是gen_ts链接依赖；request/proof/model未动。源仍冻结0a885e25，单native槽本作者持有，运行阶段以该目录running/progress/result和原exec终态为准，不按旧PID重启。新run补完整自身TMP身份/空目录/lsof清理收据（沿旧owned wrapper规矩），不删sharedtarget缓存。


Native第二run正常编译后strict因显式root-lib发布形态dep-info不支持于12.809s早停（native-0a885e25-2，源码/env稳、owned空、TMP已清）。root决定该normal published lib配对能力记下一轮工具欠项，不改本轮verifier。第三run `native-0a885e25-3/build.py` driver50809、exec69462，经批准改catalog命令仅`--bin gen_ts --test gate`，gate只compile额外validation目标让core devdep实际产生，不执行测试、不扩allworkspace。catalog strict真实通过且gen_ts已运行发布actual catalog；当前正常bindings Cargo51725，后续原pipeline自主顺序。阶段以该run running/progress/result/真实PID为准，不按本卡旧PID重启。


## Native第三run终态：GUI capabilities目录未keyed，严格停（未启动App）

native-0a885e25-3，driver50809/exec69462已exit1，289.642s。catalog normalgen＋额外gate compile、strict验证和真实catalog执行均成功；normal bindings严格build/verify后真实生成Root ignored bindings，tsc及实际Vite生产assets成功；新CLI与chat_upload_cancel指定test no-run strict都成功。GUI实际Cargo55169亦正常exit0，normal lib/bin均真实编译，但最终strict verify拒绝唯一实际Tauri声明目录 `apps/gui/src-tauri/capabilities` 未入producer key。actual-buildscript-declarations.json已一次枚举全部firstparty输出：TAURI_CONFIG、REMOVE_UNUSED_COMMANDS都recorded=true，其他declared files/Node bundle/web dist均已冻结，仅这个目录缺；不以“env修了”冒整体GUI来源已过。没有新App/observer/真实连接。

已报root必要窄修候选：仅khor-gui明确capabilities tree加入现extra-input快照（沿webdist模式），set/change/remove/symlink及未知目录仍拒绝；须新源独审集成再freeze，不旁路verifier。尚未获写权，Root仍0a885e25 clean，native槽本作者保留。

第三run source/environment稳、owned剩余空。原cleanup因本次Node26自动compile-cache(157regular文件/3,058,436字节)保守保留，原result false不改；postcleanup核uid/非symlink/lsof闭合，原inode rename到run/retained-node-compile-cache保存，再移除空TMP，cleanup-after-stop.json单记。最终小收据/日志双盘SHA全部复核同。内盘约5.1GiB仅略高5GiB floor，外盘17GiB；后续guard低于floor仍停，不删共享target/用户进程。下一执行点等待root对唯一GUI声明目录输入的安排。


## capabilities已获授、固定，native槽已还root

root在第三strict停后明确收回native/App槽供A4小批测试；作者已确认归还，目前只Python，不启动Cargo/App/observer/真实TCP。root用户显式新增MD数学界符需求使本轮34项，renderer由root独立owner，作者不触碰；Root0a仍冻结，待MD及本工具补丁集成后root重freeze、另授native，不拿旧GUI来源冒顶。

精确capabilities补丁 `8bf8c4245bad53e037fd4a8c3ec1a063604aec3d`，仅native_provenance.py两行khor-gui额外目录tree和test_native_provenance.py三项回归，沿现webdist snapshot机制；没有任意目录准入、没有改strict/unknown/symlink模型。原实际GUI来源拒绝和四现有文件均已hash/仅缺目录接纳的facts保留。测试使用真实GUI路径结构；实际generated_output同unfrozen错误先红，补后declared-dir通过，再把目录声明改Root仍拒；set/change(保mtime)/remove keys、symlink root/member保持拒绝。整28/.199s/exit0，旧25全保。初fixture位置与missing-extra断言红另存，仅red.log的generated_output异常与实际Native同类，不混当业务失败。

独审无阻断，精确两源SHA及边界见validation/L04-capabilities/receipt.json；尚无修后实际Tauri rebuild/strict，后续root新冻结源才验。tracked树clean，内盘validation-v-20261029恢复备份更新。下一原生执行时仍沿原guard，不清sharedtarget/用户进程；内盘5GiB floor保持，taskNode缓存迁存规则单列，不回填native-3 false。

## root集成b338：目录接纳已过，Darwin复制时间反例

capabilities窄修8bf8c424→2a5a416b，集成28/.237s绿。与A4首slice和MD合为Root b338，root持唯一native全链：catalog/bindings实际gen、types/Vite、CLI、指定core test compile strict均0；GUI Cargo0，240.681s停在生成文件mtime规则（无prior），源码/env稳、owned空、TMP清理，没运行App。

一次GUI-only受控重生成保留exact旧OUT_DIR（uid/regular/SHA/inode/lsof核）至新run retained，不改旧文件；同源/env/4合法prior，真实Cargo0后strict仍拒绝/31.391s。已停止重试，不伪改时间。fresh-output-timestamps事实为capabilities.json与acl-manifests.json新复制后mtime/birthtime保留旧值，ctime更新至本批；其他新文件mtime新。时间判断是否准确现由root仅离线最小回归/独审，仍属于本轮L04实际GUI来源缺口，不新增构建系统。原两次resultfalse/无App证据保留，后续动作另核授权与源码冻结。

## copytime固定与f02实测范围

两源011a8f24→f02b7d69已独审；generated-only保存ctime，marker仍mtime current，不改源/其他artifact/key/prior。实际copy2旧mtime单反例红→整32/.213s绿，另三反例保护两时间旧/旧marker/旧编译artifact。集成32绿、原28未删；精确SHA见validation/L04-copytime/receipt.json。

f02全链实际262.622s终态false：GUI生成目录的新ctime反例已严格过，失败实际在catalog/detect旧marker（同key另一OUT_DIR的seed误复用）。当前不扩variant proof模型，只保守GUI-only不传prior给prepare→全firstparty真实刷新；同源四合法receipt仅保CLI/core字节依据，exactGUI out仍可逆retain再实际生成。新批native-gui-f02b7d69/exec12323由root独占，尚未终态/未App；原false记录不改。variant refresh欠项留下一轮工具，不拿旧整批false冒GUI来源通过。

## 必要GUI staticlib形态：一行修正（ca83）

GUI-only f02真实106.884s/Cargo0/generated全过后，artifact_files对deps/libkhor_gui.a错误推导libkhor_gui.a.d。实际khor_gui.d同时声明三产物、自身及正确src。仅suffix加入.a/.dylib，其他规则不动；不是normal top-level published alias欠项，不扩新工具架构。d1c443ae→ca83fe58，红2primary suberrors→34/.220s绿及集成34绿，独审精确两源无阻断。证据validation/L04-library，真实旧row只作该配对点offline核证，旧整批仍false。

当前native-gui-ca83fe58/exec34145，root独占，GUI无prior→fresh actual，随后CLI/core actual/strict，不重已相同的前端/生成器。尚未终态/未App，最终scope看真实result。若再失败先诊断，不盲开全链/放宽模型；产品A4/W工作不得被等待工具挤出作者槽。

## 最终完成证据

`validation/native-gui-ca83fe58/result.json`实际green/254.051s，GUI+CLI/core各actual Cargo及current strict通过，source/env稳、owned空、TMP空；产物复制SHA固定。`validation/app-ca83-l05-term/receipt.json`及14小件双盘SHA核齐，独审原始native.jsonl确认open/Ready/drop各1，seq13→14→15，chat_put_paths/chat_say各0、真实native drop1。用户Finder拖放，root Cua CmdQ；app-exited真实0、未向App信号→退出后v5认证replay139B见原fixture marker、无Input/Ready→显式私有host清理strict[]，helper0/observer143，outer0/source稳/remain[]。这一focused oracle经独审，仅L05生命周期，不冒原全OS套件。

首app-ca83-os测试宿主硬270s到期panic101，原false与outer killpg PermissionError保留；第一次GUI附件与全屏事件可单列事实，不能回填全套通过。仅确认的私有host/child两PID经fixture元数据和出生身份核后逐PID清0，收据post-failure-private-cleanup。后续工具项记录270s人工流程拆分及非正常退出group清理诊断；本轮未再扩通用工具框架。Mac中文IME/ShiftLeft/编辑器Quit留E，完整重启恢复留S。


## 2026-11-01 发布决定与历史记录边界

用户最新明确授权“先直接commit到main并ship”。本卡以上实施、失败及阶段权限记录原样保留；本次授权及发布状态以[本轮发布记录](../ROUND-2-RELEASE.md)为准。原 Mac 真人中文 IME 与真实 iPhone 操作仍未验，按用户决定移到发布后，不登记为通过；本卡记录提交不代表已合并 main、完成任一 SHIP 目标或已发布。
