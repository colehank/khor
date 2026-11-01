# 第二轮当前状态

更新时间：2026-11-01T12:34:52.346848+08:00。这是当前协调入口；历史过程归档见末尾。目标 **完成第二轮，29反馈＋5遗留＝34项，active，未完成**。用户授权实施与Lab，**没有本轮main合并或ship授权**。新独立问题只登记NEXT；已启动的原范围做完。工程与协作准则读[PLAN](PLAN.md)、[ENGINEERING](ENGINEERING.md)、[ROUND-2-EXECUTION](ROUND-2-EXECUTION.md)；逐项需求/验收读[ROUND-2](ROUND-2.md)、[ROUND-2-ACCEPTANCE](ROUND-2-ACCEPTANCE.md)。不要把历史的待开工/已发布语句当当前授权。

## 立即恢复：版本、所有者与活进程

- Root工作树 `/Volumes/disk0/Khor/worktrees/current/khor-round2`，`integrate/round2@a228baef4540ac4fe03c40879d4cb50f89da69c4`，当前clean。main仍e7c335c8未改。f23后的四小提交为TerminalPane过期注释清理7ad4454e、Processes绘制回归1ca20968及readiness c5e76f36、最小正文label避让a228baef；不涉及Rust后端。
- 协调工作树 `/Volumes/disk0/Khor/worktrees/current/khor-code-quality-plan`，plan/code-quality；仅Root写STATE/总账。任务卡及独审文档属于各作者，禁止全局stage/stash/reset。
- **当前所有实际槽FREE**。device库exec3007已终态2：实际3compile phase0/124.732s/errornull/194ALLgone/source稳，唯一originaloverallfalse仅SwiftPM TMP非空。非作者compile尾核f5754aee通过；Root双审后fresh精确62成员/lsof/ALL单次coldretain至diagnostics/swiftpm-tmp（ad8a9e3e），原97965e98 false字节保。当前库aarch64-apple-ios .a1083289296B/b926ba26可作后继App输入，不是App/签名/设备通过。App/browser槽FREE。16142原cold300 false/未知TMP仍保留。
- Root是唯一Cargo/Native/App实际运行者。input现仅为最终a228手机App准备最小stage/link/sign coordinator，device库pins空/readyfalse，不运行构建或改Root源码。G作者树 **khor-round2-history** 的GUI叶已交接；其中27个Rust后端较旧，**不得在其中运行/更改后端**。
- reviewer已独审f23 producer、Copy/boot及两C009场与G最小修/注释；已只读尾核91894实际绿通过c83f7d1f。后续物理库builder源3已作者reviewer/input非作者独审，仍ready=false且head/frontend留空，等最终源与真实前端绑定，不在f23盲重跑。H此前usage-limit errored，其尾审已由input/reviewer接替。
- 所有旧CUA绑定都dead：aaCopy2/copyFinal/sAppA/sAppB/sCold/endedApp/c009f23/c009Geometry/gTable等，**不可再观察或调用**，否则可能自动重启App。下一App先外部查实际PID/出生时间/完整exe，再getApp同CurrentG路径。compact后先cua.rewriteDocumentation。

## 当前合格程序（a228）与历史f23来源

- **最终前端actual4570**：fullnoEmit＋freshVite两phase0/17.562s，2340源/188gen/278assets，source/env/tools/helpers稳、ALL净/TMP空；receipt `6a4200fbfe42cc1b5ff046ebf2824e5a062b9838d7e3b8330aa66c36da7e40bb`，目录 `integration/frontend-current-a228baef-prepared-1`。最终normal3 actual55538已五phase0/274.749s且独审06fb51de。
- 前端actual35387：fullnoEmit＋freshVite0/15.5458s；receipt `b02797d911c4a0d4ef482c90378f5c635743fa4b34ae85ed623cd4df98b7ece0`，2340源/188gen/278assets；input尾审4fe8b9c7。目录 `evidence/round2/integration/frontend-current-f23a167b-prepared-1`。没有生成器/Cargo重复运行。
- **最终normal3 actual55538**：五phase0/274.749s，原no-prior/current V/source-env-gen-assets/ALLTMP净，result `a51297945d0077a9942c2f1b2ce9d488c57a08c39c461a5b748615f619254fda`。canonical GUI365cfdc9148cce141430deedfe06fd437f7f1316ce45616b122757a39a3d3d4f、CLI09cdf574d9c1a2c93225309929f30d4b911252d0e388ae8f5d1dfb487b4d83d6、bridge原e60c；proof gui ddbe17d5/cli9e7b8d01/bridgec2cdaf02，独审06fb51de。目录 `history/session-recovery/production-current-a228-prepared-1`。
- normal CLI/bridge/GUI actual86710：五phase0/339.578s，原no-prior/current V/source-env-gen-assets/ALLTMP净，result `091df6d970b223d7364cbede2451e51ca8522cce3ea4d54932499922d258c448`，input尾审94a6d82a。目录 `evidence/round2/history/session-recovery/production-current-f23-prepared-1`。
- immutable canonical GUI bf58434fdc74bc7508691714ea6c4787f2aec394ccd020689d21a854cab7cbec；CLI327a17f6b1fad16219de797c2b9f8aaf1bfac4298b71321f267e7b9e236bec30；bridgee60c7614aab939409594713694a1c99fea537422e95743737efa5c1b540d6776。真实proof分别gui/cli/bridge-provenance.json。原始22项producer环境在mac-copy-f23-preparation-1/producer-environment.json，不与13项App runtime环境混用。
- 唯一测试App固定在 `evidence/round2/platform/mac-current-acceptance-data-1/runtime/KhorCurrentGLab.app`，原目录inode36364898；已由mac-current-a228-preparation-1原stage脚本单次只换两hardlink至a228 canonical（actual59892，stageab6b2e4c），plist/qualityID **io.github.colehank.khor.quality.round2productiong** 不改。不复制/rename第二App，不再次stage同一版本。
- 本次GUI原OUT36397755/283件6603309B已fresh核identity/全部成员/SHA/UID/nlink/lsof/owner后单次可逆留存，retention合同43b4c999，见production-current-f23-prepared-1；新GUI OUT是另一次真实生成。**不要重复move旧OUT或把retention时source_absent冒当前不存在**。

## 已闭合且不受影响的主要证据（按范围复用，不重矩阵）

- S：真实Node host死亡/provider仍活81606＋原R7各exact1/0/0已独审，Unknown/None、不重复初始化、原Meta/binding/raw/history保；Load70988/Resume57844/Unsupported4303/Shell9775/Offline63373的有限真实业务与旧false分列在总账/任务卡，不重跑。
- 完整App恢复A42497：FIRST真实GUI→durable计划仅AppA90205 SIGKILL9原非0→AppB91587先读FIRST再SECOND，sameSID/run/host/provider及无重复setup全到。B17620：旧roles全退后cold93799先读旧body再真实菜单Check and recover一次，sameSID/fullnative/newrun/Resume1/Load0/New不增/prefix与earlyNote全到。两场最终正常CmdQ/ALL净/source-env稳/无force；原overallfalse分别1462bfe1/3e0aafa8仅3已知why，独审11f1aa14。Root已精确cold retain各scene/tmp→diagnostics，receipt7f450784，原result不改，不重业务。
- Shell历史：87058只读两旧SID ended0/attachablefalse/can_controlfalse、90x41/1100B screen；62253真实冷App两screen可见，normalQuit/ALLTMP净。setup缺real_home/peer字段的前两失败均未启动role，原件精确保；不是产品失败。AX辅助Terminal input不代表可交互，row word Idle不作为进程仍活证据。
- H：长历史67MiB/53262记录分页、head搜索/取消/原文184613B完整系统Export已真验；原parent RSS单点不是WebContent峰值，不新增峰值矩阵。小消息读取边界AA先旧源红再Find33绿。
- Copy：实际28765小14B菜单Copy→系统Paste/Save exact016ac216；giant保38B原sentinel但提示泛化。最小两叶39a190→Rootf23修复已原DOM先59580红再29753全部8组绿/独审4d6192f7。**17415 f23实际giant按钮显示明确too_large原文、原38B sentinel真实Paste/Save保持378d917d，normalQuit/ALLTMP净**；UI fc9eeef5/resulted77b6ed；review047c3586。不重小14B或Copy调查。旧69036过夜中断300timeout/forced原false保，ALLTMP净，不称产品崩溃。
- Boot纯回归69866：原有 regular exact `live::proc_identity::tests::exact_birth_namespace_and_group_facts_do_not_conflate_pid_reuse_with_an_owner` 真1/0/0、无helper；一次当前no-prior libtest compile/current V，98.324s，全批0/ALLTMP净/privatecopy已退，resultdce41e5f/独审047c3586。只模型判定，不冒实际OS重启。
- C008 GUI四intent11244＋14739已真过。C009既有42114 ordinary wrappedfile/OSC8，加新62032的dir普通/PlainURL普通+Cmd/fileCmd/dirCmd（Finder＋inside文件fullURL）/OSC8Cmd全部真到。newSID3690e9a8，UI0e5b0999/result794cdabd。29379实际1080→904→1544，选区不导航、904四行末→fullfile marker、1544两行首→同fullfile marker；145处两个printf硬行no-open，随后软行正控立即打开。newSID3ab1d6f5/UI7247bf6e/result254d72af。各已观察原App-host-child、blankline sh0+reap、App normalQuit/ALLTMP净，baseline保；独审2a615088。硬行负例未提交partialpath，不新增每glyph/appmouse/remote矩阵。
- G目录菜单：Light/Dark三材质宽窗、原Dark904红经PathField d4b修复后68689真实闭。W0235794外写文件/目录/branch/commit实时＋os13保exactdraft/外写V3保稿/CAS Save mine已真实独审；W冷树/右内容/Back14739、commit独立tabs/71历史/39jump菜单/阅读位置80153、四公式80153、退出草稿三入口88179/81735/33349均按源码未变复用。B真实WK加载/失败Retry/背景/桌面PinDelete60137及旧Dark证据复用。具体边界读总账，不泛称手机已验。

## 当前剩余：原范围，不缩小成功标准

- **G桌面原缺口已闭**：88916真实Processes ghost→77782同几何绘制红→91894原unit15/15与paint绿；最小修a228已整合。最终55538真实产物经原stage59892只换唯一App两link后，93241实际PID22748/birth13:01:31 Showall200、scroll1＋.35，两settled截图列标题清楚无旧glyph、正文渐隐/无divider；正常CmdQ0、无timeout/force、ALL/footprint净/TMP空/source-env稳。UI48e75459/result91e6205b，独审b2024571。不重菜单、diff或整主题矩阵。TerminalPane两处过期注释已独立7ad4454e纯//清理并独审，无行为变化。
1. **手机App前置待执行**：exec3007已真实构建当前a228离线device库，3phase0/124.732s，非作者f5754aee尾核通过；原overallfalse仅TMP，另经独审f5b3f8a1/freshgate的62成员0B诊断单次保留ad8a9e3e后tmp现无。库1083289296B/b926ba26，target末44927807488B（约41.843GiB）<50；仍不冒App/link/sign/install/真机。源3单次共享600与原ALL/4096/jobs1/offline/50GiB/资源门已非作者独审3ee3e970；当前真实前端receipt6a4200fb、source2340/gen188/assets278已绑定，Root fresh target38.3327GiB＋8<50及外10/内2峰＋20/10保留通过才准入。原16142 cold300 false/未知TMP/部分cache保。库后手机App计划在 `mobile/physical-current-a228-preparation-1`：只29小输入/当前Info保权限、一次newdevice.a、单privateDerivedData与本地既有profile/identity，原App builder300、ALL不减；待库结果后Root绑定/核容量与执行。不复制旧库/UI/DerivedData、不联网provision/不探测或安装手机。
2. **真人Mac中文IME/Shift选择、真实iPhone原范围仍未验**。用户明确当前不方便，不复用旧“方便”答复开限时实例，不重复询问；其它工作照做。手机验网页同页viewport/底栏、左滑两动作、W窄布局、G固定区、三入口连续候选、软键盘和自定义组合键行；模拟/pure/桌面不冒真机，不加物理双机/全厂商矩阵。
3. **34项末审索引已完成，最终平台未闭**：`ROUND-2-CLOSEOUT-REVIEW.md` 非作者核34唯一行、每包具体CleanCode边界及原证据复用，当前SHA ec8a1444。真实当前平台结果继续逐行更新验收账。R001旧卡死/R016原拒绝唯一因果缺现场，按原限定调查如实unknown/未证修复，可交付限定调查结果；不加用户批准unknown前置、不无限扩调查。main/ship未授权。

## 存储、恢复与执行约束

- 读[STORAGE](STORAGE.md)。唯一target `/Volumes/disk0/Khor/caches/rust/khor-quality-target`；jobs1/incr0/offline。每重批fresh外20GiB/内10GiB保留＋声明峰值，5GiB只是紧急线。内盘约13GiB/外盘约108GiB，当前源库producer有连续原guard，不能跑并行重批或放大deadline。
- 已完成simcache精确退役45169，外实际释放7233753088B/约6.74GiB，原sim证据及0B冷现场保。不要执行无-p cargo clean（会清整target）；不要扫未知目录/用户数据/旧证据，不新增target或完整binary gzip/内盘镜像。
- App realHOME=/Users/zgh，私有KHOR_HOME data1；每批13项runtime env、新空TMP与fresh port；producer22 raw env只用于原V有限事务并finally恢复。Shell是非login /bin/sh，无ENV/BASH_ENV/ZDOTDIR。程序不写真用户历史、不调用真实agent/model/API。
- 所有原失败、timeout/forced/业务红保留；adapter/setup/清理失败与产品行为分列。独审后仅精确已知诊断可单独保留，不能回填原false、白名单unknown TMP或删ALL身份。
- 新源码bundle `sticky-processes-a228baef.bundle`相对f23，外内各6734B/verify0/SHA c978be262e2fa63c2318a602fb3c00a5d5ffc119be797450b11187848549afdf；前f23/AA/d4b/fb01链保，内部latest-source-bundle.json是当前索引。只小源码/metadata入内盘。
- 内盘恢复入口 `/Users/zgh/Library/Caches/khor-code-quality/recovery/{STATE.md,RESUME.md,current-runtime.json,latest-source-bundle.json}`。disk0断开先读RESUME，不创建假/Volumes/disk0目录，不以摘要覆盖外盘未提交内容。Root Git对象在共享repo，原G作者未提交需按真实diff恢复。

## 历史只读入口

此前累积STATE完整字节已归档 `/Volumes/disk0/Khor/evidence/round2/integration/state-checkpoints/STATE-before-20261101T123452.md`，SHA `8d07f1bb2c8d8f846563ec3e110d1796712383d20ef33f25a644e0eb187d052f`。旧计划commit99628260也保当时版本。档案中的旧pending/live/head以本页与实际handle为准，不能重复move/stage/发布/kill。详细日志/截图/原始收据在 `/Volumes/disk0/Khor/evidence/round2`、维护记录在 `/Volumes/disk0/Khor/maintenance`；本页只恢复当前目标与行动。


### 当前库冷构建终态

16142原result bd3d85392a678ce11870147d267f6ecb4ec80525e3b34f8763579432a65e89ae；原1256身份/登记组全部gone，source稳；Cargo已生成多项真实artifact但未到GUI库，日志无compiler error。target由37971677184增至40898142208B（约38.09GiB，仍<50），外盘105GiB/内盘13GiB。未知TMP因严格门原处保留，cleanupfalse；不删除/重命名或回填原result。下一在G旧源paint红后最小候选，库任务下次应在当前源最终收口后按构建实际预算另行独审与Root明示准入，不自动同源重跑、不冒原300已过。

### 最新当前库准入

Root为exec3007仅JSON ready/auth绑定，driver97703e45不改，合同f34232d9/input4c096cc4；库builder600并非App交互时限，原16142不改。已装SDK iphoneos26.5与std/当前source-gen-assets逐SHA冻结。当前库result97965e98原false保；library-facts8d4eabd7证明实际3phase0/当前staticlib；source/gen/assets/SDK工具稳，194ALL无。后续精确coldretain ad8a9e3e另证TMP现无，不回填原green。库只在现有唯一quality target内。

### 当前库后续App输入交接

当前newdevice.a `/Volumes/disk0/Khor/caches/rust/khor-quality-target/aarch64-apple-ios/debug/libkhor_gui.a`，1083289296B、SHAb926ba26eec1c42824817a646c892e4ac1e2f23185401b720f38f50254b0da4c；facts原CargoJSON/actual head a228、receipt6a4200fb与source2340/gen188/assets278绑定。原compile结果97965e98始终false，仅已知SwiftPM TMP，post-retention resultad8a9e3e为独立真实状态。input正在同一App计划目录准备最小stage/Xcode coordinator，未运行/未复制库/未签名；原App builder300、唯一deriveddata、现本地manualprofile、无下载/provision/install/设备probe。
