# 当前代码治理状态

更新：2026-10-28。完整目标active，当前进行已启动修改的同源组合与既定App验收；未完成。继续先读本页与[PLAN](PLAN.md)，核实际refs/diff/agents/PID。历史详[上个状态](history/STATE-before-bootstrap-integration-closeout-20261028.md)和任务卡；不按摘要覆盖源码或重启旧handle。

## 用户决定与准则

- **Clean Code贯穿治理**：命名、职责、规则一处、状态/资源owner、副作用与失败语义，见[ENGINEERING](ENGINEERING.md)。格式只是机械小项，不按行数、文件数或注释数计质量；必要协议/并发/平台约束保留。
- **开始了的做完，新独立问题只记录下一轮。** 不追加模块改造、功能或矩阵；既定验收的必要编译/接线和整合回归窄修继续。
- 未上线，统一当前协议；无旧端兼容矩阵。认证、epoch、完整性与真实ACP能力保留。
- **无main合并/ship授权。** main `/Users/zgh/Desktop/working/khor@8704a503a27b36ad58f922690de50e766f8bd485`及用户App/服务/release不动。

## 当前代码与唯一执行槽

集成 `/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`，`integrate/code-quality-wave1@495d096d54ea083304031423ca052f87dd9bc75e` clean。当前完整2388 frame36f15241f5524d1e900d9b160087eb26b512a5db3f217c0713a869500efaf07e，含gen157/dist277/锁。生产源码与21b1完全一致；56bde仅case030失败snapshot diagnostic，495d仅Peer fixture Busy拒绝适配，所有原testbody不变。N2作者130fb、R2作者828f、退出作者3707均已集成，refs镜像仍须现场核。

**唯一native owner当前为root。** 接续现场核无旧agent/运行handle且run08 OUT/TMP均不存在后，root按原已独审plan08启动，session74788，OUT P5/integration-affected-combination-run08/TMP见冻结data。当前63父过、040与062两父失败保留下一轮、50正在执行；0metadata/Cargo/源码改动或重试。不得并发native/先编GUI。原038/039 session17427/controller65454已exit0完整归槽，不重启旧handle/先编GUI。最终111 run03已完整后验：28父过，原case030 Query Unavailable红；source21b1-d21f稳/75dual/139组237owner全退/TMP清，result98039bdf…b43e。只补原assert失败snapshot的case030诊断run一次通过（原生产代码不改、不当fix）：25dual/sourcead57/10组35owner/TMP清，result387b0c7d…1461；其diagnostic已56bde639提交，原Query异常待下轮定位。

剩82 run04原session51972/controller41525已exit1/完整后验：5producer/list全0，原031–037七父过；原038 `a_bad_source_or_receipt_stays_unresolved_while_valid_siblings_cancel` 因peer=Err("document transfer state busy")实际红，余74未跑。56dual/sourcead57/64组113owner全部退/0forced或guard-cleanup错误/TMP kcomb38清；总239.328s/result3b7edeb0d7895ec68cc32f29f239f39719c3f1661481aa749b043a3d1eef56a3/rawf026f5c12a3ec79fbddd8216e5d8bf2e2791aaf4417ca85c03b1f4b24c94aab9。

run05继续最后74，原039 `pending_reference_recovers_only_by_its_hash_and_preserves_bad_bytes` 同样在Peer Node::open Busy提前退出，0新过/余73未跑；36dual/15.469s/70组89owner全部退/无forced或guard-cleanup错误/TMP kcomb39清；result105b0cb0b079e1812480627a95817e8f669d92819d12c91ca36da705d41bdb83/rawf03657cb63878fd247a34404e7bd3108525f43427569744edc145d5ea09b8deb。

两次Busy均在fixture respond的Node::open?处提前返回，finish先汇总peerErr，原siblings/hash强断言未执行；生产IPC Delegation会将同错误封为Refused继续服务。无法从why判具体gate，Node并发准入问题仍下一轮。为完成既定业务验收，按PLAN“必要接线最小修正”采用仅fixture适配：immutable home保原身份/binding/agent/operation校验，只有精确已知Busy写Refused，其他错误硬失败，成功open再核home；所有原test body/时限/强断言逐字不变，无sleep/retry/新case。`P5/receipt-peer-busy-draft01/fixture.patch` SHA a08000d519e7797ac2f8390157a47cf4df7079af7fa33cb14a5c67cd42acafbc 已root与非作者node_test_review通过；当前已授权作者apply/固定formatter/freeze并静态准备仅原038/039，固定真实formatter单源write/check0；格式后testSHA e02e1ed3b4828c4641d3a6b264826e04463307be29c8b0fb1c70132431cee68f，完整patch34ee655afe871695b3c6866fc4817315da3f54dbc5d3a3a8427d99033d6f9e44，当前full2388 frame36f15241f5524d1e900d9b160087eb26b512a5db3f217c0713a869500efaf07e。source01六dual、plan01七dual，runnerbac64e2c7e65312a2c8c9408ace8dcb69d285132980a8a58a171961b199d722e/datab32e7f90811653d6e368c975317641074b85c2ee83aa61eb674830e239690a8a均最终短审通过，已授仅此一次执行，随后两父已执行通过，详下一段；OUT receipt-peer-busy-run01/TMP kpeer40。通用原case对象含stub env，本两父实际must-not-run fixture无stub读取，定向同原diagnostic只child注入current CLI、不造stub当前receipt，差异明记。这是对之前“夹具也只记录”的窄化修正，不实现Node准入治理；两个旧失败原件保留不回填。

`receipt-peer-busy-run01`原038/039均实绿并root独核：0385.730s/raw803dc5b3af2780977c21a1f57136b36fa74ebaacfdefaf601c8a03d8a4750eb4；0398.883s/rawb189bd556aff7afd8d87aaf1ce2e672788c91cfda39f9ec69eb8ff4c7d2a571a。总168.560s，result61e8755bae26bc6d3b9219fc5892dbdcb8510247320ddba42d88b41445ede8f3/26dual/source稳/114组160owner全退/0forced/guard-cleanup错误/TMPkpeer40清；Nodebuild70.574s/CLI66.405s均freshfalse/opt0/verify通过。tested源码已495d提交，blob相同；两个旧失败不回填。**现在42原父已过（40带版本映射+此次2），剩73未跑。**

- 当前Node dev libtest receipt `P5/receipt-peer-busy-run01/node-provenance.json` SHAb99f629626ebfaa63535932999287b3e4b628aa49c37199202848377037e1bec；同目录正常CLI receipt22fd52264cdb2ee4c16a7df2e15fe4e961ba98f1b6798d2f1b75ee2b00e28df7。必须恢复原pins环境真require后使用；不按mtime猜。
- 原115精确减guard-run01已过4、同生产21b1 run03已过28、诊断run已过1、run04已过7=40，再加原038与039定向2=42过，**剩73未执行**，plan06精确后缀已短审并授单次执行槽。诊断只改未过case030的失败message，其余test body/全部生产不改；旧28与4带确切版本/影响映射复用，不重跑；原case030不当“已修复”。
- 当前静态计划 `P5/integration-affected-combination-plan06`7dual，runnerabefed141504be5c4fbd13d2caa2198bd8b5b62d7ba31371057ec86165512b16/datadc9d428f334222be52ea6524f4ca2e7bc75f494d8de53e0a6d81934090ac3934，精确115−42=73（Node49/A2A14/wire1/core3/pairing6）；root与node_test_review短审，73原case对象逐JSON保、整个case loop/cleanup对plan04逐字同。OUT run06/TMP kcomb41已结束：5producer/9list与当前7receipt require全过，但首原040 worker-start deadline断言红，0新过/余72未跑；232.532s/result66c55f9e4f1809272562634550825af321927b4ad4c5c5595b7fa983f8d254b7/raw2a0093a1eb5ad2ca71491f51d9999015585c0f040e663256dd0fdde95e01d8c5。51dual/source稳/50组101owner全退/无forced-guard-cleanup错误/TMP清，root已独核。raw无at/due，原因未定，记下一轮，不诊断/重试/改时限变绿；42已过、040已记录失败、余72。
- runtime-only run07已root逐raw独核：原041–06121父/24positive summary过，原062 shutdown_worker在1051因Node::open Busy红继而父789restart失败，0额外helper父，余50未跑。47.590s/result1df41700a82bb2e77fe168c75454c0fbddbeebffdb5b3003a1039f8a0d9e9ed5/rawc73e8a3aa2fcd142206911c302318cce10a9179565fdd7f40e7f1fe1c4f0038d，57dual/source稳/99组402owner全退/无forced-guard-cleanup错误/TMPkcomb42清。原062归已登记Node打开准入候选下一轮，不新诊断/源码修/重试。
- 剩50外部controller窄适配已root与node_test_review短审通过：`P5/integration-affected-combination-plan08`7dual，runner86d1c0df08a06b49bef3af3e60f4a177a9ea62be90036f9362d63c3825946a76/data473252975c49950a53bf85598d108a79f29f1df5025d59a9106cf3c46b2f6e3a。只在原entry已证明int exit101、完整最后FAILED0/1/0 summary+本父行、logpath/SHA、timedout/forced/interruption均否、owned清理门通过后保失败并继续下一个原case；任何其他异常仍停。成功断言逐字保，aggregate failed空才green、success+failed精确50，每父只一次；f7fcb不改。当前7receipt真实current，0metadata/Cargo/mtime。原63pass/040与062红来源独列，不混pass。
- 接续控制只当前Node/CLI require→stub/A2A/wire/core/pairing五必要producer重证或build→剩73原父，原case对象/flags/数据/helper/summaries/markers/Pi仅accounts/custom-wire普通listing全保。历史receipt只原matching-key/variant/files复用，freshtrue必须原verify selected.reuse=true，不把旧source selected直接当当前。
- fixture适配的已过40影响映射经node_test_review独核：其中8父用Peer且全部要求finish成功，旧Node::open?任一Err必使finish失败，因此旧绿未走本次新增Busy分支。成功dispatch/events/hold/reply尾段byteexact，immutable home前置谓词经成功后device==home同义；原038/039验证新增合法Refused后的真实Cancel/坏bytes/issue/starts强门。无需再增父；40只能记旧fixture版本实际过及等价映射，不冒已用新helper重跑，原030一次绿不称产品修复。
- 原真实toolchain-pins环境/PATH全文恢复，SHA6582c4471645016dfe900e561685bf29c3d605d8d9e0f52be8797fbd0cf79533；不重复prepend。custom wire harness=false仅all-list核完整3名/ordinary，不能按--ignored分类，不选nested Cargo parent。
- 300step/720total/5GiB/default512/f7-entry/fcb-module/owned/source/TMP/dual原门保持。plan06已短审并授槽；首错停保原件，不自动修/重跑/加时/接GUI。所有产品源冻结。

## 已修改边界的Clean Code收口

18c0a9d1以SettlementRequest/Issuer-Joiner替代平铺实参和角色布尔，删废弃devices_only支、共享固定base纯规则；3e5e25a4共享精确input与sticky规则、5行private setup终态判定；357c7070用UploadRelay集中线程/socket/gate owner，for_app命名构造，poison Drop保失败。均独审、数据布局/JSON/认证/副作用顺序保，实际受影响组合在上方进行。

33c8bc12统一格式入口main/显式root，默认check、显式write，受限import --write探针0副作用；原d26e608f仅216 Rust机械排版。共同check覆盖450第一方、392第三方SHA全稳；两源最新guard候选另经真实同版formatter静态解析/check0。格式证据见[Q053](tasks/Q-053-format-baseline.md)，不代替结构/行为成果。

**21b1a467必要整合修复：** 原wire父实际红`trust settlement recovery is pending`，trace有效Delegation→delegation_local→Node::open→恢复返回→bootstrap第二次门。普通Doc journal的registry.next可能误触第二次保守判定。现在恢复/注册自身/bootstrap共持同一现有root fence，bootstrap显式借guard，返回Node前释放；无跨await、无新锁/重试。原needs_recovery/Journal/Ready/坏next/其他信任mutation门全保；root与node_test_review独审通过。

原四父在候选上实绿并root逐raw核：wire015 2.482s、Pair anchor/flushed真实SIGKILL恢复4.439s/3summary、完整未提交image与全部partial .724s、unknown/corruption/semantic拒绝.196s。2build Node70.018s/wire52.457s；4distinct/6summary，总138.490s。source01 patch43ffb32f…ae8b，blob=candidate=tested=commit；未改fixture/oracle。

## 证据边界与保留的非绿

- 原R2作者59、退出作者37与N2 source24原46已有实证；当前完整整合不直接继承它们，按上方当前矩阵核。
- 33c8基线四原大数据网络父均实绿/各22dual/源稳/owned-TMP清：Pair29,223,902B/原90s父19s，N0 25,446,089B/visit1约10s，两32MiB父14.688/17.023s，actual=charged33,031,394/33,030,354B。新两源仅初始化guard，数据热路径未改；不冒旧binary已覆盖新入口，最终明确影响映射，不自动重编三个local目标。
- P5组合run01六producer与前14父过，在wire准入被generic listing误判；62dual/result69c3c8e…0329/321.993s，原非绿保。外部plan02恢复原wire listing后，run02实际wire父红；27dual/resultc3b48525…229/8.518s，其余100未跑。旧controllers74706/88779均退，不重启。
- guard-run01上述4业务父全过，但整体因fixture alpha Root晚写15files/9033B保cleanup非绿，原result a534e62b30e9fd46f595335f6e64fe3fc01c367cc3feb2dca240b3ef7c4b3bdf。27dual/source稳/10组ESRCH/0forced或live footprint。root按原stat/hash/lsof空，独立双盘ZIP归档后仅移除确切owned files/空目录，`integration-bootstrap-guard-postcleanup01`，archive adce829180b9f97862b833c2ec9964fe15669c4d04bddf312bac26a3007d6c3a；原result未改。现/tmp/kguard35不存在，controller2166已退。
- 当前npm gen→tsc→Vite原build0/99.915s，gen157/dist277当前合法输入；原run因Node26 TMP cache保cleanup非绿，已独立ZIP/postcleanup，未改Q055或重复build。

## 后续既定App验收（未启动）

1. 原038/039已完成，当前剩50完成并归槽，按实际结果收口；不提前覆盖共享debug libs。
2. 正常quality GUI build：`P5/Q033-quality-gui-build-plan04`10dual已静态刷新495d/36f152，runner与已审03逐字同、仅4个data字段；root已核delta，无阻断，尚未执行。单正常dev/no-default/devtools，quality JSON、REMOVE_UNUSED_COMMANDS显式unset、现成真实Darwin Pi bundleaf955/26agent输入。Q055 TAURI_CONFIG/REMOVE env自动proof缺口明记，原prepared/实际verify异常保，不造完整receipt或全绿。真实artifact单独核。
3. 依原driver的GUI→helper顺序，`Q033-core-helper-producer-plan03`9dual沿02仅prior_receipts换最新run06/core receipt1b73ca9c…f21a，495d/36f152及runner与已审01逐字同，root核1key delta通过，实际core helper在GUI embed改写共享Node deps之后标准重证/必要构建（无embed环境匹配原require），不放宽fcb；GUI独立artifact SHA保持。真实App runtime env-i、私有HOME/ROOT/TMUX、无SIDECAR/凭据/GHOST，验证真实embedded提取/启动；Python ACP仅Q033独立peer。
4. `P5/Q033-real-app-owned-plan04`已静态交付，B1遗留data KeyError、B2 native/wire/ACP实际业务关联、B3 RUN/work已修；03独审只剩smallC实际wire应SayWith，04仅此单行替换已短核通过；未执行。不能用人工accepted bool替业务oracle。原helper270/firstPart30/外300、正常App仅CUA Quit、host stop/finished/observer有界退出、owned/source/TMP/dual门保。
5. Q033四有效PNG已双盘，`Q033-real-app-driver-preparation`原三文件保持；page旁观`Q033-Q035-app-observer-plan02`只增加4命令实际metadata/原Promise，root已读diff。真实UI全用CUA，至今只有getState只读inventory，未启动App。
6. 同一正常artifact后续验Q035实际page/双私有peer借网/最后owner释放、Q043 OS拖放/全屏/关闭/正常Quit。私有relay可用正常CLI `relay --dev`；仅KHOR_RELAY会追加public tier，两个新Root首次open前须按已审README的非空only_mine配置。Q035原四native程序化资源场景仅端口改动的3dual已逆核，不冒生产App手势。

首两GUI故障/自动保存/冲突/锁/身份/预算/脱敏/poll/页面owner/SSH本地deadline等已有分批实现与独审，详[AUDIT](AUDIT.md)/任务卡。原真实iOS/Safari证据有确切版本，物理iPhone/IME未全验；类型、fixture或旧binary不代替最终App/其他平台。

## 下一轮与执行资源

新独立问题仅记AUDIT下一轮：Q055 GUI env覆盖；原Node打开typed Busy与fixture setup/晚写Root；Q033既有Self构造前setup owner gap。Q041d fresh-onset仍ready/设计待实施，原卡明确未授权新源码/编译/Linux/musl/外部部署且无WIP；与未实现remote lease保原待办，不新增当前App前置。真实SSH/物理设备及跨OS证据边界分别保留。

唯一共享target `/Volumes/disk0/Khor/caches/rust/khor-quality-target`；jobs1/inc0/lockedoffline/短私有TMP/两盘各>=5368709120B/actual源与产物/有限PID-PGID全成员/TERM-KILL-reap/dual。常规300/720/512，历史冷local1200/300/1500与8192仅登记使用，不放宽产品90s/30s/数据门。禁global clean、重放旧cleanup或碰用户进程。

原1.95.0工具链事务已完整恢复，restore-result complete=true；不得重复恢复/rustup升级/增组件。RUSTC/RUSTDOC/PATH固定真实sysroot文件，绝对RUSTUP_TOOLCHAIN/AUTO_INSTALL0并核三SHA；formatter1.9.0禁代理自动安装。fcbf4fe1…0ef8模块与f7ff0426…085e1 entry不改。权威本目录，外证据`/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`，内恢复`/Users/zgh/Library/Caches/khor-code-quality/recovery`；掉盘读RESUME/镜像后再核现场。
