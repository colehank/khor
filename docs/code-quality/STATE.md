# 第二轮当前状态

更新2026-10-30。目标**完成第二轮 active**，**29反馈＋5遗留＝34项**，不缩范围。用户已授权实施；本轮main合并/ship未授权。main保留e7c335c8基线，不动。逐项完成标准见[验收总账](ROUND-2-ACCEPTANCE.md)，调度/Clean Code/compact见[执行计划](ROUND-2-EXECUTION.md)与PLAN/ENGINEERING。

## 恢复与当前现场

先读最新用户消息、本文件、自己的任务卡，核真实refs/diff/运行句柄后再动作。外盘作者dirty源优先，不按摘要覆盖。旧源/失败/运行语句保存在[此前状态](history/STATE-before-startup-sdk-20261030.md)及更早归档，不能把旧PID当live。内盘RESUME、current-runtime/current-worktree-refs和逐SHA文档镜像是掉盘恢复入口；逐文件镜像不假称全局原子。

**Native/browser/App全部FREE。** H B7 actual37853已terminal0/原7组全绿、源/env/tools/Chrome版本/owned/TMP净，prod8f/test410已整合Root84c81dd0。N SDK79152正常帧红绿闭包与L01/L02实际已归档；input继续EventStore源，H准备生产origin完整App平台验收，当前无实际CLI/store/App/native。新批仍先固定来源/独审/Root授槽，再按真实handle/进程核运行，不复用旧PID。

| owner | 当前来源与下一职责 |
| --- | --- |
| Root | `khor-round2` / integrate/round2 **84c81dd0ff6aa04316531c4ef3e5b74cdb5b4315 clean**，含H raw/notice/内容高度修正与独立回归脚本、SDK七链。协调集成/源与证据/真实App，不成为第三条大产品实现线。最新严格App/CLI是**666200f7**，早于notice与SDK整合，不能冒最新整合产物。 |
| round2_history | `khor-round2-history` / fix/round2-browser-shared **410c1f1b**（生产8f65423e），H后端/前端/共享出口固定。当前H七组收口完成，准备真实WK/原生平台验证；之后H实际App与S完整恢复按依赖交接；不并写input的ACP/Node启动叶。 |
| round2_input | `khor-round2-input` / fix/round2-startup-bounds，最后fixed **e3268412** SDK绿组件，继续既定EventStore/spool/finite fence、ACP lib/lifecycle/event_fence及必要Node gui_host/journal启动交接；新dirty需保存。N旧95acd与pristine SDK链保。 |
| round2_reviewer | 非作者只读。已核SDK红绿/120文件整合、L01/L02实际与H notice source；Browser5最终PIN也已核。偶有capacity报错可Root作非作者最小尾核，不擅改model/不冒未审。 |

## 本阶段已经实际推进

### 当前统一构建与两个遗留

Root666的 `validation/native-current-666200f7` actual19660 terminal0/**226.646s**：metadata/Vite/CLI/GUI五phase全0，selected CLI/GUI均原CargoJSON/no-prior strict，source/env/179gen/assets/owned/TMP净；result **a98257f54cd281fe218971d20245bb3e37801ced9f12d1edce2f5e4dc188176e**。私有CLI **320d5c7b…8b80**、GUI **878b1bff…0586**。这是真实private devURL Lab，未ship/main，且不是当前82的新whole-source产物。

**L01当前契约回归已通过**：`history/l01-current-666200f7` actual10596 terminal0/**110.241s**，只额外一Node libtest actual no-prior V producer＋唯一原ignored exact **1pass/0fail/0ignored、8.45s**；用上述匹配CLI，无ACP stub/模型请求。result **8744edb810356841d87fd03ce4639b193198e53164aa20856f3f2b154b09f1c6**。源/env/gen/assets/owned/TMP净。due1793339710658；旧4run retry在due+3/+4ms，initial=quiet，fresh早于due，13直接CLI reaped。生产remote_cancel/mcp_worker/khor_agent三源仍等原基线；4f7仅清理后诊断输出。不能把本绿归因N Busy或冒原040唯一cause已知。

**L02限定原两native场景已通过并独审**：`validation/app-l02-current-2` actual83173 terminal0，inner **31.857s/phase complete**。真实WK late-close、close-reopen两case true、known final handles0、全部actual completed；exact私有GUI/App50340经**CUA确切新App路径**正常CmdQ exit0，短attester在终态前核PID消失/app0后写原ack；observer143，no forced、remaining/footprint[]、TMPabsent/source-gen-fixture-contract稳。receipt **4e8631b7f9085226134ab4383936d24d6d773870ba75ec9b8c83596ab31c3341**；outer **f2fcc588…dab1**、inner **dfffc481…40c8**。不扩原四Q035 case、E真人IME或完整S恢复。

首 `app-l02-current` 的两case/App0虽成立，但整体timeout/forced cleanup false原样保。ack确实晚于result，原未采timeout阶段，无法判定app-owner wait或ack wait；retry仅诊断/控制短命令，**所有原门/时限/fixture不变**，不反推首失败唯一cause。两个App bundle仅hardlink私有qualified GUI副本，未复制380MB或动user/target。

### H：实现与production browser闭环已整合

H基础五片＋转换事实已在Root，实际17phase/Node4+Core3+tables4＝11通过、真bindings等既有证据见R2-H。新增原文export8后端源 **44fa2fef**＋搜索/复制14源 **5daaad3d**已Root整合511；actual92510 Native6/Core2+tables4＋严格catalog/完整noEmit通过，result **09447d8a…4210**。177管理bindings不变，真实catalog **84476d70…33d07**；Root总179gen/noEmit0。首56136编译E0624/0行为原false保，最小root参数接线后重试。

Raw共享出口七源 **cb03c482→Root666**，e3694a44候选逐SHA同，作者29pure/7真实Worker/type0，Root完整noEmit0/179gen不变。单一ChatView export owner给Raw/Search复用；物理边界依source metadata，不因JSON合法升级；损坏JSON仍有原文出口、fragment必须先定位完整边界。Native完整原文走现file_handoff，同job/cancel、64KiB IO、桌面OS Save/原子发布和手机share，尚缺本批真实系统出口验收。

Browser2在真实UI暴露旧local复制错误遮住已完成原文export成功，saved/full range实际正确。**425d919f notice两叶→Root3eeecd05**，current export优先，新local intent清旧idle export notice/不清active owner；非作者源码审过，RootfullnoEmit0/gen179不变。此后SDK整合不含新TS变更。

Browser记录均不回填：B1先3组后fixture不存在words locator错；B2真实viewport前三过/第4notice业务红；B3 Chrome launch180s/0page0组，原因未知；B4 Chrome恢复/实际6组过，第7fake ledger把成功IO仍算active，另killpg0 EPERM strictfalse。B4旧PGID事后无成员/ESRCH只作新准入，不改原false。无已证SDK或用户Chrome关联。

**B5实际失败，源/环境/工具/owned/TMP净**：第7两阶段ledger已正确后，document阶段held100确已cancel/drop；zero-height阶段held104 claimed且未cancel、仍有newer轮询。当时只读发现visibility/canFollow只看含padding的clientHeight，尚缺运行几何；随后B6已用原门采样确认并由8f修正，详见最新闭环。B5原false仍保，不倒填。

B5 script **c6f3fffb…6b37** / wrapper **da0b1b84…bad4** / admission **0461eba4…bb85**。只纠正fixture实际生命周期：成功page/record逐key Drop；projection本地解析但home pending待显式cancel；held取消只stop仍claimed，先证明exact cancel恰1/无未取消active，再释放IO/finally后ledger0与不续读，unmount最终0。原7组/300总/180launch/strict不变。当前Chrome launcher/info与Playwright版本前后pin、DEBUG actual argv/stderr、最多三次只采已证私有PID/profile；无用户Chrome/profile访问。未证明WK剪贴板/OS Save、手机或整H完成。

### S启动：SDK红绿组件已整合，EventStore尚未实现完成

原SDK3.5.0锁定archive **9bbe69c8…bd75a**的114 pristine文件保许可证与原manifest/lock；未升级SDK/derive/schema。原third_party excluded会漏跨run package key，已按现helper `vendor/`规则独立迁移，未改工具。实际Cargo生成path lock只删除同SDK entry的source/checksum，其余parsed TOML全同，**8df88cbc…7e6e**，保生成失败原结果而不手写/回滚。

原d181→作者e326七提交 **84736111→8ff41e2d→a154a46a→3781b5b4→9c342a9f→0298b20d→e3268412** 已Root无冲突整合至**82b45cf3**，**120最终文件逐git-show/SHA全等作者**。handoff **e3b5156c…df8e**，整合receipt `integration/startup-sdk-82b45cf3` **76b280bd…5765a**已独审。源bundle `round2-notice-sdk-82b45cf3.bundle` 内盘verify0，依赖666并包含notice两叶。

真实baseline `ingress-red-source-4` actual90088 terminal0/**11.071s**，现full host-filtered locked metadata＋原no-prior V/actualACP libtest producer＋sole业务101，fixture未改。peak **25,240,210B/384帧**超过8MiB/256，normal最大65,730B/oversize0。DriverClosed、真实child61073身份/gone、credit0、无Ready/Load1/New0均在最终窗口断言红之前成立。receipt **e2088a68**独审/全镜像同。

容量/waker源码e326只在同物理producer入口背压，whole-frame ownership跨原channel/ConnectTo/dispatch/drop；register→锁内fit、closed/减账解锁wake，巨帧独占分支按实际长度计费。**同原用例** `ingress-green-source` actual79152 terminal0/**10.750s/1pass0fail**，原V/所有生命周期/strict净且源/env稳；peak **8,347,600B/127帧≤8MiB/256**。另待解析完整行65,730B明确另计；RSS采样3760→19360KiB仅该进程样本，不是kernel peak或8MiB硬RSS。真实child65884身份/gone、DriverClosed、credit0、Readyfalse/Load1/New0全到达。receipt **f7044dd3**已实际独审。

这只关闭**正常帧AcpAgent/ConnectTo/Lines饱和取消SDK入口**，不冒ByteStreams、巨帧、tiny-count独立压力、EventStore、长base或整个App有界。原预备false：15877 no-deps未写锁；47991无host filter被offline Android依赖拒、但实际两行锁已生成；17068 fullmeta0/compile E0004，后仅to_json借用递归适配。前三均未跑pressure，不当业务红、不下载、不改V门。

**下一实现是原S EventStore闭包**：一个per-transport owner，真实8MiB/256编码窗口＋8MiB分段spool，head/tail/accepted-read seq与两个FD/单decoded领取结果受控；receiver未返回也能持续落盘。控制/Ready/Close/failure不堵payload；互动credit到回答/drop才释放16/128KiB。finite fence同store+accepted前缀，GUI用remaining_through不追后来len，canonical完整前不HostReady。取消要唤醒各等待者/64KiB检查/真实shutdown，磁盘失败明确拒绝且不后台续写。不截断合法长base、不退回32768 count cap，不单靠MPSC boundedawait制造启动死锁。此片未native，NodeByteStreams长base/原4096 fence合同也待真正证明。

## 其余包与真实验收缺口

| 包 | 当前可用证据 | 仍需完成 |
| --- | --- | --- |
| N | Rootd181整合十提交/6源全等95acd；actual当前CLI IO6＋root/meta两positive＝8父/2helper，原ID与锁序/等待额度保，actual ca63545a…8edd | 不外推原集群唯一cause或所有网络/GUI；当前组合按影响范围核。 |
| E | 唯一buffer/草稿owner、composition/迟到读取/退出保护的定向门；ce69旧独立GUI/CLI仍保 | 真实Mac中文IME/拖选/最后一键→正常Quit与失败保存取消；新鲜人工可用性再问，不能据历史答复自行开限时实例。 |
| W | 最终布局/独立commit preview-pin、当前分支完整历史、watch分层Core/CLI/DOM过 | 当前实际App文件变更/连续导航/菜单/深浅三材质与窄屏。 |
| M | 公共跟手/DOM、键行pure/UI及真实TerminalPane→PTY→隔离Codex press+keyup与常规自动化键盘路径一致 | 真手机候选手感/软键盘；不冒实体键盘或该Codex自带ShiftLeft选区。 |
| A | 能力policy/A3/A4真实链与40项源码矩阵（22自动/18手动） | 最终原生组合，不冒40个安装vendor实测；外部观看不夺控制。 |
| S恢复 | canonical hydration/NoLoad source与旧38+inner1/补6分源证据，L01/L02当前限定回归过 | EventStore启动→完整host丢失/重启与GUI/Shell历史/控制/状态恢复；不能用L05或SDK单例替代。 |
| B | pure规则/Core2/GUI12/IO2/真typedgen，L02本地WK仅关闭交接 | 真WK进度/主题/错误运行路径、原网址及手机同页视口/删除。HTTP200不是WK故障证据，非WK不伪比例。 |
| C/G/Q | 打开意图/折行/回cwd、共用连续渐隐、括号公式同数学链与定向门 | 当前App点击/Cmd/窄屏/文件渲染/滚动/材质和手机组合。 |
| D | 限定调查/采集步骤已留证，原现场缺栈/IPC | 原冻结唯一cause unknown；无当前复现不冒修复，见R2-D。不新建watchdog框架。 |
| V | L04实际源/环境/ACL/生成规则与L05实际正常Quit/认证持久host replay/私有清理已完成 | 不替代E/S；最新whole-source App还须新真实产物，原工具失败均保。 |

## 执行下一步

1. H七组已闭环，接着按冻结平台计划完成正确production variant的当前CLI/GUI、真实系统保存与UI组合，不重旧Native6/11/catalog或无关L01/L02。新whole Root产物须包含当前visibility/notice+SDK，旧666只保真实已测范围。
2. input继续同S EventStore/NodeByteStreams长base闭包，保存dirty/合同与最小反例；新native单独准入，Root不并第三大产品线。
3. H在稳定UI交付后可按Root交接做Core/GUI完整恢复，但Node gui_host/journal/lifecycle仍input独占，接口有稳定片再转交，不双写。
4. 对34项逐条填实际证据再完成审计；D/原WK故障未知不被其它绿关闭，只有用户明确接受延期才缩范围。新独立问题仅记NEXT-ROUND。main/ship等用户指令。

## 磁盘与旧验收副本

5GiB门不降。9个已结束test exe及4个旧0478/ca83私有CLI/GUI副本均逐UID/regular/nlink/SHA/lsof→gzip→完整解压SHA核再归档，外盘共约3.40GB释放；恢复index在内盘round2/archived-test-executables-current99与archived-old-lab-binaries-1。旧plain路径已不存在，不能拿原收据假称仍可运行；ce69人工备份、Root99CLI、当前666二进制、用户/sharedtarget未动。新Khor开发/证据仍在既有分类目录，不散落根盘。

## H最终浏览器闭环与平台准备（最新）

B6实际trace7193已证父/root高度0、scroller.clientHeight24=padding12+12、原RO contentRect0，旧visibility/canFollow误resume，held105未cancel并续newer，明确产品红。8f65423e仅ChatView installHistory缓存实际内容高度（初测扣padding、同现RO更新），两判定共用，不改scroll/anchor坐标或增observer；Root8bae915d完整noEmit0/179gen不变。独立test410→Root84c81dd0，脚本df9db83e逐gitshow=实际执行。

B7 actual37853真实0/7of7，result **fe3a6d879a470eb38a7069415440fc9978edf1469d12887b3a8de929a6c656d5**；fixedmap47651442同。document101/zeroheight105各exact Cancel→IO release/finally Drop→最终leases{}，无迟到续读/unmount0；raw受控job完整range/saved，source2205/UI623/179gen前后稳，strict无timeout/forced/remaining/footprint/TMP，25小件双盘SHA尾核通过。B1–6所有false保，不外推真实WK剪贴板/OS Save/share。最终源码bundle依赖82b45已verify。

下一真实Mac平台准备 **plan6b111a7b444673091ae62d43e06f567a50a94a4e5d12ce8a48816309b8c3cf36**（history/search/mac-platform-preparation）：一次默认custom-protocol+devtools完整私有App＋当前CLI，真实原入口ws add/state hook/import在专用HOME/KHOR_HOME/KHOR_VENDOR_HOME种一份合成Claude历史，无模型/第二writer/手造binding；1201记录403053B、末大record184544B仅纯检查，尚未CLI/store/App。实际GUI列表/worker/native IO→原文OSSave fresh私有路径→原bytes核→CUA正常Quit，外240/内200。Clipboard Copy/Paste合并到既有真人Mac IME新鲜可用性场次，由用户正常动作确认synthetic；不读取旧clipboard、不写JXA/AppKit、新API或替换index。

生产主文档URL应tauri://localhost/（DOM origin/secure等须实际记录），`khor-ui-assets:/assets/index-*`仅UI_REVISION标识。旧666明确no-default-features+devtools+devURL23846，只能证明localhost WK，不能外推生产Clipboard。Root下一构建应保默认custom-protocol+devtools、隔离identifier/资源/原strict来源；选对variant后一次实际当前产物，未授权ship。H完整S重开只读链也已存session-recovery/full-app-readonly-preparation（efc35f85），不写input启动叶，不用一个NoLoad场景冒崩溃/host死/远端离线全覆盖。
