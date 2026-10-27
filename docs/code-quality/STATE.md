# 当前代码治理状态

更新：2026-10-28，已整合功能批与独立纯格式批，进行已改边界Clean Code收口。完整目标active、未完成。继续先读本页与[PLAN](PLAN.md)，核实际refs/diff/agents/PID；历史收据与失败保留在任务卡和[上一状态](history/STATE-before-clean-code-closeout-20261028.md)。禁止按摘要覆盖源码或重启旧handle。

## 用户决定与目标

- Clean Code贯穿功能修复与工程治理：命名、职责、规则一处、状态/资源owner、副作用、失败/取消与必要约束。原始目标是全仓审计与分批治理，不把格式统计或局部测试当完成；稳定准则见[ENGINEERING](ENGINEERING.md)。
- 最新边界：**不用冻结，开始了的做完；之后新独立问题只记录下一轮。** 当前只收口已经修改的边界和既定验收，不追加模块改造、功能或矩阵。
- 未上线，统一当前Khor协议；无旧端fallback/迁移矩阵。认证、epoch、完整性和真实平台/ACP能力保留。
- **无main合并/ship授权。** main `/Users/zgh/Desktop/working/khor@8704a503a27b36ad58f922690de50e766f8bd485` 保既有基线，用户App/服务/release不动。

## 当前实际代码

治理集成 `/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`：`integrate/code-quality-wave1@33c8bc1254d26335ddc5ea409d215a8b426bd30a` clean。内部整理与格式入口已分批提交，实际前端/绑定构建通过；当前唯一native槽正在最终四网络边界。

- R2精确input取消：作者828fbd31→aa9f80ca已整合，原59 distinct实绿、311.919s/94dual；HTTP无能力仍Unknown。
- 正常serve/GUI agent退出：source03作者3707c1fd→dc98d327已整合，原37 distinct实绿、normalTauri check0；自动GUI producer因环境输入缺口保非绿，仅记录下一轮，不改Q055补门冒全绿。
- N2 Pair/Vouch：作者树 `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-overwrite-recovery@130fb99788ca9dd8f3d5b69c5a24a6225ec6b728` clean，source24 exact13已整合14931af7。root核13路径增删完全匹配、10整文件相同，lib/link/proto其余R2/退出增删全部保留；无冲突。
- N2 source24原清单46/47已真绿（state18 1、runtime19 1、runtime20 44，helper不充父）。runtime20 114.582s/58dual/45组与TMP清，root逐原raw/summary/双盘/source/PGID核；**当前整合四原网络父已在最终整合源全部实绿；原N2剩46待当前115组合。** source22安装优化Pair04曾21s绿，不继承给新源；dev90s红和各setup/cleanup原非绿不回填。
- Q033既定真实App上传验收前置：test-only驻留host source03已a3a7cbde提交，普通业务断言保留；尚未编译或App绿。
- 纯Rust格式已单独d26e608f提交：实际check1→write0→check0，449第一方中216需调整，392第三方SHA全稳；9dual `P4/format-migration-14931`，非作者已核841当前after/216 before精确匹配。此后新结构修改需一次共同check。

## Clean Code收口：只审已改边界

| 范围/所有者 | 实际内部整理与不变量 | 当前状态 |
| --- | --- | --- |
| N2，shutdown_owner实现、原作者独审 | runtime SettlementRequest/Issuer-Joiner表达替代平铺实参和true/false；删除废弃devices_only支；固定devices/epoch/base纯规则共享但两处enforcement/错误/owner顺序保。serialized structs、原settlement body和exact-op helpers保。 | 4源18c0a9d1dccbca6f8161df92d28bda2c2dbc642a；原作者独审通过，最终native待。 |
| R2/SH，node_transfer_owner实现、shutdown_owner独审 | ChildReceipt input纯判定与sticky合并一处；5行private agent_setup只共享JSON终态判定。publication/re-read、JSON字段、cancelled-only guards与native join归属不变。 | 6payload 3e5e25a40a9e23e301c1d1e0972de57c6fbb1c6f；独立逆展开5旧源byteexact，最终native待。 |
| Q033，root实现、node_test_review独审 | UploadRelay集中网络/线程/socket/gate归属，Fixture负责session/host；for_app命名构造。network block原样、业务tail仅relay字段资格；停止/Close/有限join顺序保。新增Drop的poison锁二次panic已窄修，join失败仍失败。 | 单源357c70707ab674825c90572aea8abf7b0754ef98；原网络块/业务tail语义保，poison窄修独审通过，实际fixture/App待。 |
| P4入口，shutdown_owner草稿、node_test_review整理、root审 | 默认check/显式write、workspace edition/独立模板/第三方排除、真实formatter禁auto-install；main入口与显式root，import --write探针无I/O副作用。 | 33c8bc1254d26335ddc5ea409d215a8b426bd30a；实际450第一方共同check0/392第三方稳，import探针0副作用，CI接同入口。 |

**当前唯一Cargo/native槽：node_transfer_owner，115组合run01执行中。** session85294/controller74706/Cargo74754（最初CLI）；OUT P5/integration-affected-combination-run01。metadata0/.877s、build-cli0/67.439s已过、当前后续producer。runner0cc2cf83c0aa0b79ecc0d46f517cecf0718ce6f57888f21bac0193db7b3a135e/data7da2589dba02eac0d40b08d357b0cf9c9f13dd2bd46f66a08f1a97cc7ffd33f4经独审；300step/720total/5GiB/default512/entryf7-modulefcb不变，root授权严格115/6必要dev producers串行，失败停不修/重跑/加时/接App。Node86只复用下方budget03 actual receipt；root clean33c8/full2388，禁止并发native/App/重启同handle。

**当前整合四原网络父全部实绿、owner后验与root独核均完成。** Pair01总646.454s/build621.07s/父19.00s、29,223,902B/原90s/首次重试/VV/content/epoch1/1/2rows/sameRoot/trusted/第三方拒绝全过；result2326207322fdf29ecca97961e257fc9bb31cd5c4f297d17b8887b585ed89163e/provenance528702135233e476de4234e90b8974c551a94ef17405b9acd159b19d11217669。N0 run02总474.977s/build458.410s/case10.494s，25,446,089B/visit1/VV/content/Seen123/Pintrue/capreject0，resultc3151f5f342ce04f2ea6d89f984b48ef1e5eb7aa818f057a7493e19561fccea0/receipt02f5435264bff9e8bdb25f42312e36a696a8b8dde95f424bf05357de7be243c1。budget03 build708.601s/Chat14.688s/Devices+Agents17.023s/总745.646s，原两32MiB/30s/256KiB/SeenPin/公平不变，actual=charged33,031,394B与33,030,354B、boundary/last_chunk_reached/target_exchangefalse原标记全过；result05ced0a6df6fd34aa8f0f4f306c073a3e069abeba6371ca40fed8595e623287d。各22dual/2388源前后/current等/clean/owned-TMP清，原worker不充父；budget03 15组ESRCH。旧controllers23369/41010/54600均退，不重启。

budget03真实Node libtest receipt `Q039/integration-final-network-budget-run03/node-provenance.json` SHA f58237ba66e6141694b7ff5322b4ad96543cd9a3c4a7aa564fc6bc98431f0289；实际selected local/deps/khor_node-2f2d90e2d715d97d、opt3/cfgtesttrue/freshfalse、binary SHA b7ad00065f6f9387f8139931e2fa404da78218c10daeab99f3705c0e1cae3099，root实核。

四既定父/三个local targets已顺序完成；未重编dev Node libtest。当前115组合按原最小集合复用最终local receipt；失败立即停，不自动修源/重启/加时/扩大；fcb/f7不改。

前置原npm run build（gen→tsc→Vite）99.915s exit0，当前33c8 tracked/source1954不变；gen157/tree a5f8941e…30c82，dist277/tree5c487ed4…d135e，非正常CLI/Node producer声明。整run因250个Node26 compile-cache TMP文件保非绿；root在全部组退出/Lsof空/原inode与SHA核后dual ZIP归档3.36MB并移除私有TMP，仅postcleanup，原result不变。外/内Q055/integration-final-frontend-run01及独立postcleanup01。

P5外部Q033 observer/driver/qualityconfig三文件已静态准备，真实invoke仅旁观/原Promise/无fake；未启动服务、App或浏览器，不抢native槽。当前组合计划E/P5/integration-affected-combination-plan01双盘6文件已node_test_review独审通过，已用最终Node receipt启动。清单已去重：R2/SH+Q033共69父，与原N2剩46零重叠；当前四全绿后最小剩115（最终Node local libtest复用86、pairing6、A2A18、wire2、core3），连当前四总119 distinct。其余targets及正常CLI/ACP-stub只defaultdev必要编译；组合已在四完整后验后顺序执行。GUI实际build覆盖caller编译不重复check，此处尚不宣称组合或App绿。

## 原完整目标余项

- 首两故障及autosave/冲突/锁、身份/预算/脱敏、上传收据、poll、页面owner/事件、SSH本地deadline等已有分批实现与独审；详[AUDIT](AUDIT.md)/任务卡。原Q010真实Tauri iOS/Safari已有证据，最终全部同源App仍待，物理iPhone/IME未全验。
- 当前已实现批的既定P5仍保：Q033真实ChatView→Tauri→host→独立ACP上传Stop和同socket后续输入；Q035真实page/双本地peer borrow；Q043 OS拖放/正常Quit/全屏；当前组合正常producer与App Exit。Q045本地owner原135回归与真实SSH/master的证据边界单列，不能把本地reap当远端lease。类型/fixture/旧分支通过不当全部App/平台通过。
- 按最新范围核卡：Q041d fresh-onset仍ready/设计待实施，原卡明确不授权新编译/源码及Linux/musl/SSH外部写入；现33c8该链无WIP。它和未实现的remote lease保原目标待办/下一轮，不新增为当前App前置；物理iPhone/IME与跨OS运行缺口仍诚实保留，不冒已验证。
- 原P1结构清单和未知项仍在，不冒逐行深审；P4规则/职责/历史注释的收口依已改范围，必要协议/平台/并发注释保留，不追行数/注释数。

## 仅记录下一轮

AUDIT末节：Q055 TAURI_CONFIG/REMOVE_UNUSED_COMMANDS输入覆盖；Node重打开Busy与fixture setup失败后晚写；Q033既有Self构造前setup owner gap。没有为此新增锁/owner框架。run17两私有残留已dual归档postcleanup17后移除，原非绿保留。最初normal/Pair候选中已被原case11实际阻断的部分在N2 source24必要修，未把未证实候选冒作已修。

## 恢复与执行资源

唯一共享target `/Volumes/disk0/Khor/caches/rust/khor-quality-target`；唯一Cargo/native槽、jobs1/incremental0/lockedoffline/短独占TMP；内外盘均>=5368709120B，actual源码/producer/artifact、owned PID/PGID全成员、有限TERM/KILL/reap和dual。常规300/720；已登记冷编local仅1200/300/1500与有限8192历史门，产品90s/30s/原数据断言不放宽。禁止全局clean/重放旧cleanup/碰用户进程。

真实原1.95工具链已从事务备份完整恢复，restore-result complete=true；不得重复restore或运行rustup升级/增组件。后续RUSTC/RUSTDOC/PATH固定 `/Users/zgh/.rustup/toolchains/stable-aarch64-apple-darwin/bin` 真文件，绝对RUSTUP_TOOLCHAIN、AUTO_INSTALL0，核实际SHA；formatter显式真实RUSTFMT，禁代理自动安装。Q055模块fcbf4fe1…0ef8不改；安全entry f7ff0426…085e1，默认历史512。

权威本目录；外证据 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`，内恢复 `/Users/zgh/Library/Caches/khor-code-quality/recovery`。掉盘先RESUME及镜像再核现场refs/WIP/agents/PID，不按摘要覆盖源码。历史14大Rust副本已可逆归档，见STORAGE/ARCHIVE.json，旧binary路径不可直接复用。

当前P5准备（非运行绿）：Q033-quality-gui-build-plan01 6dual（原工具prepare/单正常GUI build、真实af955 bundle/显式quality JSON及REMOVE unset，Q055 gap保留；root静态核），Q035-native-reuse-plan01 3dual（原run11四场景仅端口替换、root逆核exact），Q033-real-app-images01四有效PNG/6dual。GUI one-off原控制器窄适配由shutdown_owner只静态准备，node_test_review只准备生产App page返回旁观和已有正常CLI私有relay步骤；不启动服务/metadata/源码写。CUA仅getState只读inventory，尚未启App或任何UI动作；后续实际UI全走CUA。
