# 当前代码治理状态

更新：2026-10-28。完整目标active，当前进行已启动修改的同源组合与既定App验收；未完成。继续先读本页与[PLAN](PLAN.md)，核实际refs/diff/agents/PID。历史详[上个状态](history/STATE-before-bootstrap-integration-closeout-20261028.md)和任务卡；不按摘要覆盖源码或重启旧handle。

## 用户决定与准则

- **Clean Code贯穿治理**：命名、职责、规则一处、状态/资源owner、副作用与失败语义，见[ENGINEERING](ENGINEERING.md)。格式只是机械小项，不按行数、文件数或注释数计质量；必要协议/并发/平台约束保留。
- **开始了的做完，新独立问题只记录下一轮。** 不追加模块改造、功能或矩阵；既定验收的必要编译/接线和整合回归窄修继续。
- 未上线，统一当前协议；无旧端兼容矩阵。认证、epoch、完整性与真实ACP能力保留。
- **无main合并/ship授权。** main `/Users/zgh/Desktop/working/khor@8704a503a27b36ad58f922690de50e766f8bd485`及用户App/服务/release不动。

## 当前代码与唯一执行槽

集成 `/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`，`integrate/code-quality-wave1@56bde639f37ace27694b78282fa648bedabc2d07` ；现dirty仅授权fixture测试源。原fullframe2388 SHA ad57f6ffcb67cba13b2a82e306d83e9434026daad95460be315a31140e1f4525，含gen157/dist277/锁；生产源码相对33c8仅21b1的lib.rs/trust.rs两源变化；56bde639仅给原case030断言增加失败snapshot上下文，操作/时序/expected逐字保，其余2387相对21b1逐SHA同。作者N2树130fb、R2树828f、退出树3707均clean，refs镜像仍须现场核。

**唯一native owner为node_transfer_owner。** 原038/039定向source01+plan01已root与node_test_review最终独审通过，已授单次执行槽；不得并发native、重启旧handle或先编GUI。实际session17427/controller PID与PGID65454，OUT P5/receipt-peer-busy-run01/TMP kpeer40；旧run均已退。最终111 run03已完整后验：28父过，原case030 Query Unavailable红；source21b1-d21f稳/75dual/139组237owner全退/TMP清，result98039bdf…b43e。只补原assert失败snapshot的case030诊断run一次通过（原生产代码不改、不当fix）：25dual/sourcead57/10组35owner/TMP清，result387b0c7d…1461；其diagnostic已56bde639提交，原Query异常待下轮定位。

剩82 run04原session51972/controller41525已exit1/完整后验：5producer/list全0，原031–037七父过；原038 `a_bad_source_or_receipt_stays_unresolved_while_valid_siblings_cancel` 因peer=Err("document transfer state busy")实际红，余74未跑。56dual/sourcead57/64组113owner全部退/0forced或guard-cleanup错误/TMP kcomb38清；总239.328s/result3b7edeb0d7895ec68cc32f29f239f39719c3f1661481aa749b043a3d1eef56a3/rawf026f5c12a3ec79fbddd8216e5d8bf2e2791aaf4417ca85c03b1f4b24c94aab9。

run05继续最后74，原039 `pending_reference_recovers_only_by_its_hash_and_preserves_bad_bytes` 同样在Peer Node::open Busy提前退出，0新过/余73未跑；36dual/15.469s/70组89owner全部退/无forced或guard-cleanup错误/TMP kcomb39清；result105b0cb0b079e1812480627a95817e8f669d92819d12c91ca36da705d41bdb83/rawf03657cb63878fd247a34404e7bd3108525f43427569744edc145d5ea09b8deb。

两次Busy均在fixture respond的Node::open?处提前返回，finish先汇总peerErr，原siblings/hash强断言未执行；生产IPC Delegation会将同错误封为Refused继续服务。无法从why判具体gate，Node并发准入问题仍下一轮。为完成既定业务验收，按PLAN“必要接线最小修正”采用仅fixture适配：immutable home保原身份/binding/agent/operation校验，只有精确已知Busy写Refused，其他错误硬失败，成功open再核home；所有原test body/时限/强断言逐字不变，无sleep/retry/新case。`P5/receipt-peer-busy-draft01/fixture.patch` SHA a08000d519e7797ac2f8390157a47cf4df7079af7fa33cb14a5c67cd42acafbc 已root与非作者node_test_review通过；当前已授权作者apply/固定formatter/freeze并静态准备仅原038/039，固定真实formatter单源write/check0；格式后testSHA e02e1ed3b4828c4641d3a6b264826e04463307be29c8b0fb1c70132431cee68f，完整patch34ee655afe871695b3c6866fc4817315da3f54dbc5d3a3a8427d99033d6f9e44，当前full2388 frame36f15241f5524d1e900d9b160087eb26b512a5db3f217c0713a869500efaf07e。source01六dual、plan01七dual，runnerbac64e2c7e65312a2c8c9408ace8dcb69d285132980a8a58a171961b199d722e/datab32e7f90811653d6e368c975317641074b85c2ee83aa61eb674830e239690a8a均最终短审通过，已授仅此一次执行，尚无通过结论；OUT receipt-peer-busy-run01/TMP kpeer40。通用原case对象含stub env，本两父实际must-not-run fixture无stub读取，定向同原diagnostic只child注入current CLI、不造stub当前receipt，差异明记。这是对之前“夹具也只记录”的窄化修正，不实现Node准入治理；两个旧失败原件保留不回填。

- 当前Node dev libtest receipt `P5/case030-status-diagnostic-run01/node-provenance.json` SHA08565f1d662422b11ca59cb95f31025f34ccfbb1ff971e681f741c6c015bab03；正常CLI receiptb407bb525d063327a1b6c672130448d36fdf4c78c8b07aac398801cd1d84174e。必须恢复原pins环境真require后使用；不按mtime猜。
- 原115精确减guard-run01已过4、同生产21b1 run03已过28、诊断run已过1、run04已过7=40，另有原038与039两父fixture失败未达业务断言，**剩73未执行**；先完成两父适配后定向，再继续73。诊断只改未过case030的失败message，其余test body/全部生产不改；旧28与4带确切版本/影响映射复用，不重跑；原case030不当“已修复”。
- 接续控制只当前Node/CLI require→stub/A2A/wire/core/pairing五必要producer重证或build→原038/039定向后剩73原父，原case对象/flags/数据/helper/summaries/markers/Pi仅accounts/custom-wire普通listing全保。历史receipt只原matching-key/variant/files复用，freshtrue必须原verify selected.reuse=true，不把旧source selected直接当当前。
- fixture适配的已过40影响映射经node_test_review独核：其中8父用Peer且全部要求finish成功，旧Node::open?任一Err必使finish失败，因此旧绿未走本次新增Busy分支。成功dispatch/events/hold/reply尾段byteexact，immutable home前置谓词经成功后device==home同义；原038/039验证新增合法Refused后的真实Cancel/坏bytes/issue/starts强门。无需再增父；40只能记旧fixture版本实际过及等价映射，不冒已用新helper重跑，原030一次绿不称产品修复。
- 原真实toolchain-pins环境/PATH全文恢复，SHA6582c4471645016dfe900e561685bf29c3d605d8d9e0f52be8797fbd0cf79533；不重复prepend。custom wire harness=false仅all-list核完整3名/ordinary，不能按--ignored分类，不选nested Cargo parent。
- 300step/720total/5GiB/default512/f7-entry/fcb-module/owned/source/TMP/dual原门保持。下一计划须静态独审后另授槽；首错停保原件，不自动修/重跑/加时/接GUI。所有产品源冻结。

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

1. 原038/039最小fixture接线与定向、剩73完成并归槽，按实际结果收口；不提前覆盖共享debug libs。
2. 正常quality GUI build：`P5/Q033-quality-gui-build-plan03`9dual已实际更新56bde639/ad57，runner与已审02 byteexact，仅4个来源字段；node_test_review已短审无阻断，尚未执行。单正常dev/no-default/devtools，quality JSON、REMOVE_UNUSED_COMMANDS显式unset、现成真实Darwin Pi bundleaf955/26agent输入。Q055 TAURI_CONFIG/REMOVE env自动proof缺口明记，原prepared/实际verify异常保，不造完整receipt或全绿。真实artifact单独核。
3. 依原driver的GUI→helper顺序，`Q033-core-helper-producer-plan01`7dual已独审，实际core helper在GUI embed改写共享Node deps之后标准重证/必要构建（无embed环境匹配原require），不放宽fcb；GUI独立artifact SHA保持。真实App runtime env-i、私有HOME/ROOT/TMUX、无SIDECAR/凭据/GHOST，验证真实embedded提取/启动；Python ACP仅Q033独立peer。
4. `P5/Q033-real-app-owned-plan04`已静态交付，B1遗留data KeyError、B2 native/wire/ACP实际业务关联、B3 RUN/work已修；03独审只剩smallC实际wire应SayWith，04仅此单行替换已短核通过；未执行。不能用人工accepted bool替业务oracle。原helper270/firstPart30/外300、正常App仅CUA Quit、host stop/finished/observer有界退出、owned/source/TMP/dual门保。
5. Q033四有效PNG已双盘，`Q033-real-app-driver-preparation`原三文件保持；page旁观`Q033-Q035-app-observer-plan02`只增加4命令实际metadata/原Promise，root已读diff。真实UI全用CUA，至今只有getState只读inventory，未启动App。
6. 同一正常artifact后续验Q035实际page/双私有peer借网/最后owner释放、Q043 OS拖放/全屏/关闭/正常Quit。私有relay可用正常CLI `relay --dev`；仅KHOR_RELAY会追加public tier，两个新Root首次open前须按已审README的非空only_mine配置。Q035原四native程序化资源场景仅端口改动的3dual已逆核，不冒生产App手势。

首两GUI故障/自动保存/冲突/锁/身份/预算/脱敏/poll/页面owner/SSH本地deadline等已有分批实现与独审，详[AUDIT](AUDIT.md)/任务卡。原真实iOS/Safari证据有确切版本，物理iPhone/IME未全验；类型、fixture或旧binary不代替最终App/其他平台。

## 下一轮与执行资源

新独立问题仅记AUDIT下一轮：Q055 GUI env覆盖；原Node打开typed Busy与fixture setup/晚写Root；Q033既有Self构造前setup owner gap。Q041d fresh-onset仍ready/设计待实施，原卡明确未授权新源码/编译/Linux/musl/外部部署且无WIP；与未实现remote lease保原待办，不新增当前App前置。真实SSH/物理设备及跨OS证据边界分别保留。

唯一共享target `/Volumes/disk0/Khor/caches/rust/khor-quality-target`；jobs1/inc0/lockedoffline/短私有TMP/两盘各>=5368709120B/actual源与产物/有限PID-PGID全成员/TERM-KILL-reap/dual。常规300/720/512，历史冷local1200/300/1500与8192仅登记使用，不放宽产品90s/30s/数据门。禁global clean、重放旧cleanup或碰用户进程。

原1.95.0工具链事务已完整恢复，restore-result complete=true；不得重复恢复/rustup升级/增组件。RUSTC/RUSTDOC/PATH固定真实sysroot文件，绝对RUSTUP_TOOLCHAIN/AUTO_INSTALL0并核三SHA；formatter1.9.0禁代理自动安装。fcbf4fe1…0ef8模块与f7ff0426…085e1 entry不改。权威本目录，外证据`/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`，内恢复`/Users/zgh/Library/Caches/khor-code-quality/recovery`；掉盘读RESUME/镜像后再核现场。
