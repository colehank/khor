# 第二轮当前状态

更新时间：2026-11-01T13:53:06.252953+08:00。目标**完成第二轮，29反馈＋5遗留＝34项，尚未完成**。当前等待真人验收；目标工具状态是最终权威。用户授权本轮实施与Lab，**未授权本轮main合并或ship**。新独立问题只记NEXT，原范围已启动的缺口做完。

## 立即恢复

- Root `/Volumes/disk0/Khor/worktrees/current/khor-round2`，`integrate/round2@a228baef4540ac4fe03c40879d4cb50f89da69c4`，源码clean。main仍e7c335c8。
- 协调树 `/Volumes/disk0/Khor/worktrees/current/khor-code-quality-plan`，plan/code-quality。Root只写STATE/总账/本次真人指南；原作者任务卡保持原owner，禁止全局stage/stash/reset。12份pending卡精确内盘备份见central-docs-pending-latest.json（1100940B），不是转移卡owner。
- **所有实际槽FREE，无编译、App或浏览器验收在跑**。最近exec78507只读补验终态0；此前80494/57058/81494/3007也均终态，不能重启旧handle。详情以 `/Users/zgh/Library/Caches/khor-code-quality/recovery/current-runtime.json` 为准。
- 所有旧CUA绑定（包括gFinalA228等）已dead，不可观察，否则可能重新启动App。新真人实例须先用户新的可用性，再核实际PID/出生/完整exe，再绑定；compact后先cua.rewriteDocumentation。
- input/reviewer本轮来源准备与非作者尾核均已交接，无进一步独立源码阻断；H此前usage-limit错误不再承担尾核。Root是唯一实际Cargo/Native/App运行者。

执行标准见[PLAN](PLAN.md)、[ENGINEERING](ENGINEERING.md)、[ROUND-2-EXECUTION](ROUND-2-EXECUTION.md)。完整范围与实际证明见[34项总账](ROUND-2-ACCEPTANCE.md)、[结案审查索引](ROUND-2-CLOSEOUT-REVIEW.md)及各包卡；旧卡早期pending不覆盖后继实际。

## 当前合格Mac程序与真实桌面验收

- 最终前端actual4570：fullnoEmit＋freshVite两phase0/17.562s，2340源/188gen/278assets；receipt `6a4200fbfe42cc1b5ff046ebf2824e5a062b9838d7e3b8330aa66c36da7e40bb`，目录 `evidence/round2/integration/frontend-current-a228baef-prepared-1`。独审e9194f8d；原19生成pins与27精确source增量保持，不重复生成。
- 最终normal3 actual55538：五phase0/274.749s，原no-prior/current V/source-env-gen-assets/ALLTMP全过；result `a51297945d0077a9942c2f1b2ce9d488c57a08c39c461a5b748615f619254fda`，目录 `history/session-recovery/production-current-a228-prepared-1`。独审06fb51de。
- 当前canonical：GUI `365cfdc9148cce141430deedfe06fd437f7f1316ce45616b122757a39a3d3d4f`；CLI `09cdf574d9c1a2c93225309929f30d4b911252d0e388ae8f5d1dfb487b4d83d6`；bridge `e60c7614aab939409594713694a1c99fea537422e95743737efa5c1b540d6776`。proof各在同目录，GUI ddbe17d5/CLI9e7b8d01/bridgec2cdaf02。
- 唯一Mac Lab固定 `evidence/round2/platform/mac-current-acceptance-data-1/runtime/KhorCurrentGLab.app`，目录inode36364898，quality ID `io.github.colehank.khor.quality.round2productiong`。原stage脚本2ec38227/actual59892仅换两hardlink，plist不改；当前GUI/CLI与a228 canonical同inode。不要复制第二App或再次stage同版。
- G最后原缺口已闭：88916真实Processes ghost→77782同几何绘制红→91894原unit15/15及paint绿；最小修a228只将held+flush Processes正文透明区推到居中label下沿，原header.bottom渐变终点不变。最终Mac actual93241，Showall200/scroll1＋.35两settled帧PID/Status/Command清楚、正文自然淡出、无divider，正常CmdQ0/ALLTMP/source-env净。result91e6205b/UI48e75459/独审b2024571。不重菜单、diff、主题或其它已绿矩阵。
- TerminalPane两处过期链接注释已纯//独立7ad4454e清理（2+/6-），必要平台原因保留；未改图片行为，不为注释重行为测试。

## 当前手机验收包：本地qualified，未安装或运行手机

成品唯一位置：
`/Volumes/disk0/Khor/evidence/round2/mobile/physical-current-a228-preparation-1/derived-data/Build/Products/debug-iphoneos/KhorRound2PhysicalLab.app`

- 私有显示名 **Khor Round 2 Physical Lab**；ID `io.github.colehank.khor.quality.round2physical`。App全17节点419051183B，snapshot `bf3f2aed005b5851863569e2dd70f16018c2820c0ba7a11cf9865739162749f7`。不要改签名App、另copy库/App/DerivedData或再build追strict绿色。
- currentdevice库actual3007真实SDK/metadata/Cargo3phase0/124.732s，source2340/gen188/assets278、SDK26.5/std58/tools均核；.a1083289296B，SHA `b926ba26eec1c42824817a646c892e4ac1e2f23185401b720f38f50254b0da4c`。独审f5754aee。原overall97965e98 false仅SwiftPM 0B TMP，另经双审/fresh62成员/194ALL/lsof单次可逆coldretain ad8a9e3e，原false不回填。首次cold16142原300超时bd3d仍保，不能说新全冷成功。
- 首次App81494是managed profile被误设Manual，Xcode65/原c92aa4c9 false保；已纠正为只匹配现本地profile的Automatic，无allowProvisioningUpdates/注册/下载。run11此前只是unsignedsim项目结构参考，不是物理签名先例。
- followup57058实际Xcode0、3Link/3CodeSign/BUILD SUCCEEDED，但原guard收自有构建服务导致forced，原wholefalse `5d62c7b6eb8d34a1444d1ae8b5144b4286a172bc05d6a83ccfe32d72b27fe761` 保，23ALLgone。不能称原strict build批通过。两次原Xcode TMP各仅1个0B文件，freshlsof均1双空，read-only recordb30e6c18，原处保诊断未删除。
- actual80494前8项读取均0：3own MachO（主stub/debug.dylib/__preview.dylib）均arm64＋IOS非sim，codesign deep strict0和team/ID0；其后Root optional参数空格错误令certificate1，原7824e012 false保。App/源未变/ALLTMP净，独审6516bdaa。
- **最终actual78507 focused补验0/9.691s**：只执行正确`--extract-certificates=<prefix>`公开leaf及entitlements两phase，复用80494原8项精确日志和sameApp/source链。leaf SHA1 `9308048ABD9F37512C6ED0CB4D5E94857AFC8DD5`，DER SHA2561b729aea，嵌入profile1d51869a，team QR8DMW493W、私有ID及get-task-allow均匹配；App全树前后不变，ALL/TMP净/无force。result `6e2dda0f3348b823185474b50ded6746c36f13a4ad1d0491c0f8667448bcc962`；`App-verified.json` SHA `bcecee378da5937d2bea392d317a1a09856b3b662bcc31a712baecad501a1283`；非作者实际尾核 `0c5f70ba5ac86863a3b92da74e36a3a6dc6c585c82884a9740f44c7860912f6d`。目录 `mobile/physical-current-a228-preparation-1/followup-1/verification-2`。
- 真正Rust在418183104B debug.dylib，原最终map是主stub map；当前来源链78250fec核actual clang动态链接、main.mm→object/filelist、6个-L的18候选仅newlibapp.a=b926、主stub对同debug引用。不要求错误的archive-in-stub-map，不造map/不重build。
- 此本地资格**不等于**安装/设备资格、真机WK、触屏、软键盘通过；未运行安装/配对/设备probe/真机App。后续必须新可用性与真实连接资格，不用SDK版本猜手机失败原因，不自动provision或改keychain。

## 已闭原证据：按源码边界复用

- S完整App A42497：计划仅AppSIGKILL后新App先读FIRST再SECOND，sameSID/run/host/provider；B17620全roles退后cold先读旧body，再实际菜单recover一次，sameSID/newrun/Resume1/Load0/New不增。原overallfalse仅3已知why，独审11f1aa14，精确冷留存7f450784另证，原false不回填。
- S Node hostAbsent/providerPresent拒重复81606/R7 exact、公开Load70988/Resume57844/Unsupported4303/Shell9775/Offline63373等有限原组合已独审。Boot69866是一个regular identity exact1/0/0，不冒真实OS重启。87058只读两旧Shell ended/无控制资格，62253冷App可见旧1100B屏，不伪造外部PTY。
- H原生67MiB/53262记录分页、未加载HEAD搜索与取消96355/17256，80153巨条184613B完整Export/系统Save；28765小14B系统Copy、17415 giant明确too_large＋原38B sentinel真实Paste保持均独审047c3586。无WebContent峰值测量，不新增峰值门或跨mount bookmark承诺。
- C GUI四intent11244/14739、终端42114/62032/29379有限主例闭：普通内部/Cmd系统exact目标，真实1080→904→1544软行同fullfile，硬行不误拼且软行正控、选区不导航；独审2a615088。不新增每glyph/全厂商/remote矩阵或网络可达预检。
- W/G/B/Q等80153/91690/68689/5794/14739/60137与退出88179/81735/33349按任务卡复用：布局/71提交/39jump/数学四分隔符/宽窄目录菜单/实时外写及CAS/真实WK进度失败主题/三文件ASCII最后字退出均已有实际，不冒真人IME或手机。
- [结案审查](ROUND-2-CLOSEOUT-REVIEW.md)逐行34项、每包具体CleanCode接缝与原证据，最新非作者SHA f4b8a0c1c79218d2e2738fcbbd67e68804707bc93a13d1e0769a68b3fdef8d5a。R001旧卡死/R016旧拒绝唯一因果仍unknown/未证修复，按原限定调查交付，不加用户接受unknown审批、不无限扩调查。已知F1/F2与bookmark等仍NEXT。

## 唯一剩余：原真人平台验收

见[可执行清单](ROUND-2-HUMAN-ACCEPTANCE.md)。

1. 真人Mac系统中文IME：逐字/选词/删除/中英切换，保存/外部或迟到更新/切窗切标签/选区撤销与落盘；先工作区再会话文件。逻辑/合成/ASCII证据不能销账。
2. 真实iPhone原范围：同页同态WK网页视口/底栏与加载终态/当前主题初始底色，左滑置顶删除、W窄布局、G固定区、三入口连续候选、自定义组合键行与软键盘。沿原一组路线组合，不增加主题/全厂商/双机矩阵。

用户最后明确暂不方便，连续至少三次goal turn未有新的可用性；此前原方便回复已过期。现在无进一步独立源码/本地构建阻断，不能用反复跑已绿项制造进度。Root将在完成当前记录后按目标blocked审计标记等待真人验收。当前原E源入口也已准备：platform/mac-ime-current-a228-preparation-1，workspace/session两独立原300场，当前55538实际pins已固定但readyfalse/实际port及env待新可用性；独审889b49d2，未创建fixture/App/计时，不补新框架或重开R028。**不得宣布整轮完成**；不反复询问、不先开限时实例。新用户回复方便后，再准备确切版本/入口/专用测试正文并开始，允许按Mac/手机分组。

## 存储与恢复

- 读[STORAGE](STORAGE.md)。唯一quality target，jobs1/incr0/offline；外20GiB/内10GiB保留＋fresh峰值，5GiB仅紧急线，target50GiB复核。当前系统约13GiB、外约101GiB可用，最近target约41.73GiB。无活跃重批，不新增cache根或大payload内盘镜像。
- 已完成simcache45169退役；当前版本切换后仅f23/bin三份旧可重建program fresh SHA/UID/nlink1/lsof后精确退役1095907400逻辑B，receipt `2bc1e3d1e7cee7c91d523cea5fe40f5818195e187079fdc52da670c1b05549b9`。所有旧source/日志/结果/失败保留，当前a228三程序逐SHA未变；f23过去stage‘old_canonical_retained’是当时事实，现旧payload不再可执行，不能重新索取或重建追绿。
- 产品源码小bundle `sticky-processes-a228baef.bundle` 外/内各6734B，SHA c978be262e2fa63c2318a602fb3c00a5d5ffc119be797450b11187848549afdf，相对f23，前f23/AA/d4b/fb01链保。恢复索引latest-source-bundle.json。内盘只小源码、STATE/refs/runtime与资格索引，不复制App/.a/binary/logpayload。当前acceptance源码小包82文件2679297B（含29小型原项目资源），manifest51ce2a06；索引current-acceptance-source.json，历史ready/auth不是新执行授权。
- 内盘入口 `/Users/zgh/Library/Caches/khor-code-quality/recovery/{STATE.md,RESUME.md,current-runtime.json,current-delivery.json,latest-source-bundle.json,central-docs-pending-latest.json}`。disk0缺失先读RESUME，不建假挂载目录、不按摘要覆盖未提交。
- 前版STATE完整字节存 `/Volumes/disk0/Khor/evidence/round2/integration/state-checkpoints/STATE-before-mobile-qualified-20261101T135306.md`，SHA `e2b51b6e911c639831bd5ed1f9c164679dc2118d7a91fbd6093d8afa1d60542f`。更早历史仍见原STATE-before-20261101T123452.md与plan commits。旧live/版本/授权必须由本页与真实handle覆盖，不重复move/stage/发布/kill。
