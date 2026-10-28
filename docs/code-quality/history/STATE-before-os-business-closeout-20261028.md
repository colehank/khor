# 当前代码治理状态

更新：2026-10-28。**目标 active，未完成；主体源码及 Clean Code 结构治理已集成，剩余真实 OS 验收有明确缺口。** 本页为恢复入口，计划与边界见[PLAN](PLAN.md)，前一详细检查点见[历史](history/STATE-before-pi-borrow-os-actual-closeout-20261028.md)。不要按历史“尚未启动/正在运行”重启旧 handle。

## 最新执行入口：新源 fullscreen 已验证，人工拖放第一消费者成功，最终 OS 全契约待补

当前 wave1/作者均 ecab575e3f978f6570bff2c05a9542be93fd7342；main8704不动。用户“纳入”已授权特定 fullscreen 查询grant，最小能力 JSON 已集成并独审；full2389 frame32101973…77438。实际normal GUI39b30026…458e0/374444376B，GUI producer40922 terminal exit1仅knownQ055证明环境覆盖缺口，normal build/artifact/source/cleanup成功；生成ACL独审ea482c96…0b63。新source Core producer13238 terminal exit0，result5a7fbeef…315/provenance6644ec80…6d50完整通过，root full require及非作者审查9d8fdabc…b559完成。五实际来源binder为P5/fullscreen-query-real-app-actual-producers01.json，不能再用旧pending状态或旧Core900b证明新源。

**人工真实拖放已成功**：fullscreen-query-os-owned-run01/App26358，用户明确“拖入了”；实际native drop两条广播观察、唯一chat_put_paths seq120成功，源与落地23B一致，无发送消息。实际query raw false→true→false已证明grant修复运行有效。原270s helper先到期，未完成terminal第二drop/guard/normalQuit，whole/manual保持false，result22d72023…d23e，独审a8dc670a…aaa。root末尾CmdQ工具在输入前因stale-state拒绝，不能称normalQuit。源/清理正确，原结果不改。

新原OS plan02仅5个namespace/reference数据键变化，run/launcher/observer字节不变，完整断言/270helper/5GiB等门均保。实际run02在ready前6.459s因内盘低于5GiB门停止，无用户操作，guard_failures仅Disk below acceptance floor:/usr/bin；所有owned/TMP清。不能重用死session或getApp重新唤起。当前无live native/Cargo/App。用户仍待新Lab验收，原人工第一drop不重复冒认第二drop。

资源：run01已独审后两份39b stage可逆归档到fullscreen-current-gui-stage-archive01/current-gui-39b.zip(SHA103530b1…214f)；run02stage暂保待独审。当前内盘余约4.27GB、外盘6.09GB，不降低门；root只整理明确task-owned已停测试重复副本。当前normalGUI/Core/CLI、source、原结果/断言保。final OS仍缺terminal第二consumer/原guard/Quit同一次契约闭合，随后独审/交付对照/文档恢复同步；不重跑已绿Pi/Borrow/115/14。

本节以下为历史记录，涉及e110/未授权grant/尚无人工事件的旧判断已被上述实际事实替代，不按其旧运行入口启动。

## 当前决定与执行边界

- 用户：开始了的做完，新发现的独立问题只记录下一轮。无 main 合并或 ship 授权，不碰用户发布 App/服务。
- Clean Code 贯穿命名、职责、共享规则、状态/资源 owner、副作用与失败语义；格式仅机械小项，具体交付见[交付记录](DELIVERY.md)与[工程约定](ENGINEERING.md)。
- 主体源码上一检查点在 wave1 `e110ce05af04a45695df3d645b2c713802eba690`，main `8704a503a27b36ad58f922690de50e766f8bd485` 未动。full2388 输入 frame `1862eba5203f6400873a4173aeed26aa2c4831b5da1e231e08f8119b5082d08a`。
- root 独占 native/Cargo/App/服务槽；目前 **所有执行均 terminal，无受控 App/服务/Cargo 在运行**。最后拖放91713 exit1、借网41123 exit0、OS61652 exit1，归档26340 exit0；不可重用这些 session 或死 CUA 句柄。
- 当前事实对齐与独审均已完成，仍有 OS 原验收的未证明部分。**不要重跑已绿 Pi/借网/115/14，不重新扩全仓审计，不把既有 ACL 或新独立 draft 覆盖缺口扩为本轮产品实现。**

## 已完成与未完成

| 范围 | 实际结果 | 边界 |
| --- | --- | --- |
| 初始键盘、commit 跳转 | 原红→浏览器/真实 Tauri iOS 模拟器/Safari 既定验收完成；d9→e110 三个相关源文件无变化可映射复用 | 不冒物理 iPhone、IME、所有平台验收 |
| 原组合115父 | 113 pass、040 deadline 与062 NodeBusy两原失败，0未执行 | 历史不回填；062已在当前e110定向通过，040留下一轮 |
| N1准入纠正 | 原 same-root Busy 与 LocalSet panic 均真实红→e110窄修；14父/22summary/19phase绿，独审通过 | 单registry/constructor/RAII、固定1s/64+64/取消waiter，control/meta/crossprocess Try；获准后同步FS不可抢占，可阻塞executor，非更深IO架构完成 |
| Q033实际多图/取消/权限/config | 真实四图A取消恢复→同稿B重发Allow/EndTurn→C取消、mode/option/正常Quit完成，42项独审补验 | 495d历史来源；原Vite143旧harness nongreen保，未重复跑 |
| Q035原四native场景 | 全执行，strict/cap两过，late-close/close-reopen两channel closed失败 | main同查询/关闭代码，唯一race未证；两失败下一轮，不称全绿 |
| **Pi实际嵌入启动/取消** | **当前e110完整实际通过并独审** | 无API key输入/无provider登录，不称全进程网络隔离 |
| **生产双peer借网** | **当前e110原五phase/实际正常Quit/清理完整通过并独审** | 仅fixture用既有KHOR_NAME首开区分同机channel；无产品改动 |
| **全局工作区未保存保护** | **OS run02实际拒写→阻止关闭→Don't quit→fixture恢复权限→实际保存与精确bytes均确认** | OS全组依然非绿，不能用此覆盖拖放/全屏缺口；独审完成 |
| OS 拖放/全屏 | 两次真实跨窗drag仍0 native event，terminal第二consumer未到；实际全屏进出但成功callback0 | 拖放工具/产品原因未唯一归因；fullscreen既有缺grant在当前配置下阻成功query。无放断言或权限修复，**全组未通过** |

## 最新真实 App 证据（E/P5 与 I/P5 同名）

E=`/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`；I=`/Users/zgh/Library/Caches/khor-code-quality/recovery`。

- Pi `node-open-pi-owned-run01`：exec22455 exit0/93.202s，result `33c789180e67eb9a8e50690fcd382dbe09aec6602a1dfbfac1e07e8c9006f512`。Quota/Register→本机home可用→OpenAI api_key→真实secret prompt空输入Cancel；实际Bun af955/76474082B/mode700，ownedPID93960；原5s退出门→CUA CmdQ/App92625 exit0→accepted→全部清。独审 `node-open-pi-owned-run01-review01/report.json` `6fa1a8f976def1274bc2fbf4fd194c302d9e849e75b0424dfa761a52cf66bce0`，16证据checks/860manifest（非新业务测试数）。
- Borrow `node-open-borrow-owned-run02`：exec41123 exit0/212.6575s，result `de4004174a1385f26254ce97a4c93ec346de5efc536bf4d72ade20c7e7535ad4`。两实际open_page同borrow/01889344be40fa00@59953；卸machine owner仍通；真关content后新TCP errno61；UI远端历史重开新borrow/68805b5df0b84999@60680再通；真关再拒绝。原5ack→Quit intent/ack→CUA CmdQ/App10450 exit0→6.932s内accepted→仅EXIT_ROOT CLI quit0/keeper143/relay0/observer143/originclosed，198owner/87group/TMP全清。独审 `node-open-borrow-owned-run02-review01/report.json` `95a4c8670ee54ac97bb1688012a64a72a3b2898cbdc8f7fe061fadae67810eee`，16checks/1000manifest归档前完整。首pair精确FinishPending1→原同ticket单Resume0/共享3.217s，未新grant。
- Borrow `node-open-borrow-owned-run01`：result `f50c7e0d6f9116aeeb5cdaf6061e7cda70af5e04fe1be5831ee6119e399d00c7` 原非绿保留，双同机fixed name Mac-mini、rename只改shown，API仍name→selfnetwork拒绝。源核main相同，既有KHOR_NAME专供同机双实例；plan02仅start中4行按真实HOME设置两freshRoot初始q035-app/q035-exit，删除此If后原launcher完整AST同，所有门/5phase/断言原样。独审e90f10a2…7ec93/956manifest。不得改写成原run也通过。
- OS `node-open-os-owned-run02`：exec61652 exit1/267.553s，result `24e3ddc5551ae8b7f466d8a069eb5e05a42473810e951c22bd4ce4a06c1a7623`。真实全局Workspaces/WorkspacePane；原write_text1–5拒绝，quit_requested preventedtrue；实际Not saved/1 files unsaved→Don't quit→guard-kept→原fixture-only restore→write6成功/精确`Q043 edited locally\n`。全屏成功query/drag/chat_put_paths/term_drop均0；CUA第三坐标误用在工具层拒绝，没有输入。root描述在freeze前`runtime/root-ui-partial.json`。root只关闭task Finder新窗并实际CmdQ/App18557 exit0，未再读取死UI；原checked/normalQuit/fullcontract门未到，未写quit-intent/accepted，**whole/manual=false保持**。原failed-path清理207owner/73group/footprint/TMP全清。原882manifest/883files独审完整，`node-open-os-owned-run02-review01/report.json` SHA `49087734f677f7a681faf8210018336b2a400ccf6154c3118eca9a2ed70ca506`，14项证据核验（不是业务测试数）。
- OS `node-open-os-owned-run01`：result `52185fe3ea147b5ee1db1647d24b5153c754c75e7151065a2685ad46955ce77f` 原非绿。root走KhorSession内部FilePage，write15/17拒绝但close preventedfalse；该入口本地draft未报全局，main已有，非监听未注册证据。后续CUA读取死窗口可能重唤私有PID7470导致原cleanup footprint/TMP异常；原结果不改，root独立ps→CUA CmdQ→ps gone/空TMP清另记`node-open-os-run01-postcleanup01`。独审 `c2e7579afcb041eb1d75a26a68ebb5bb77e45b7c38e57a2d9d557b7bdc66bd1f`/872manifest完整。

更早495d Pi Busy、OS270超时、借网setup/rename失败均保持历史。当前通过项只引用上述新source结果。

## 当前产物与运行纪律

- 集成：`/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`，作者复用树`/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-overwrite-recovery`，均e110 trackedclean。中央plan独立分支，恢复后先核refs/diff。
- normal GUI `.../caches/rust/khor-quality-target/debug/khor-gui` SHA`576678f1aa72d3557a04314cd4e87bcc353f01ecee240c00331f877320549fb2`/374443848B。实际normal build/artifact成功及独审debd3390…c8eb；**Q055完整自动provenance仍因TAURI_CONFIG/REMOVE_UNUSED_COMMANDS缺口nongreen**，不能伪造accepted receipt。
- Core `debug/deps/chat_upload_cancel-a98ef2c294b70f0c` d84f1167…ba1bbc8/75861904B；Core03 result2fc111e4…c2d5b/provenance900ba74d…cc8a7完整通过，root实际require和独审eed3f9dc…4d25完成。
- CLI `debug/khor` e0a4d3e6…f69b06/269289608B；receipt4c50a2da…2920a的完整require是合法GUI共享deps重编前历史，不冒它当前所有deps完整。GUI/Core后按实际exe/stat/source映射使用。
- 五实际来源binder `P5/node-open-real-app-actual-producers01.json` SHA f0031a21…a84668。原timestamp pending字段是历史元数据，不更新旧binder；当前事实以本页为准。
- 原f7 entry/fcb provenance不改，工具链5pins取Q039/integration-bootstrap-guard-run01/toolchain-pins.json SHA6582c447…79533；outer prepend一次，父PATH仅去恰首toolbin。唯一target/jobs1/inc0/offline/locked；原285inner/270helper/300step720total/双盘5GiB/512/ownedPID-TMP-lsof-source-dual门保，不重做工具链或全cold。
- getApp/getAXState可能重新launch已死private App；任何UI接管先ps核readyPID+exactpath仍live，正常Quit后仅ps/原harness观察，**不再读取死App**。用户94762/5467未动；task Finder新窗已关，原用户窗口保留。

## 下一轮与未证明部分

- SessionContent/SideWorkspace/MachineFiles编辑草稿未接全局reportDrafts：main既有覆盖缺口；全局WorkspacePane原路径已真实通过，不把两者混作已修。
- fullscreen缺grant阻query；本轮不增权限。真实drag未观测根因未知，不能称产品或工具已经定位/修复。**这是目标尚未完整验收的剩余边界。**
- 原040deadline、两个page_steps channel closed候选、Q055环境证明覆盖及已登记独立缺陷留下一轮。物理iPhone/IME、真实SSH/master/remotelease、Linux/Windows矩阵不新增。
- 不宣称全仓逐行重构、全测试全绿、所有平台通过或已发布。独立交付对照`E/P5/final-delivery-audit01`及其后补事实供审查，不代替本页当前结果。

## 可逆归档与恢复

旧源/结果不回填。历史producer/staged/cache归档详上一检查点与对应index；恢复bytes/mode/mtime前旧full manifest仅是历史，inode不能恢复。当前normal产物/源/depinfo/指纹保留，无global clean。

最新：`node-retired-cache-archive-relocation01`将583MB旧ZIP搬至`/Volumes/disk0/Khor/caches/archives/node-retired-variants-except-current-cli-669.zip`，SHA713709…43c9（copy验证后退旧I文件，非独立内盘备份）。`node-open-pi-completed-artifacts-archive01/completed-pi-artifacts.zip`142914326B/SHA5e5bc033430668a4349c71a9753164a138405d669907910c1017ed51cb7228ba，含真实GUI/Bun entries；Pi两stage+两runtime已退原件。Borrow run01/run02与OS run01两盘重复stage复用此ZIP退役，索引`node-open-borrow-staged-archive01/02`、`node-open-os-staged-archive01`，均在独审完整manifest与实际SHA/stat/lsof后；当前OS run02已在独审后按`node-open-os-staged-archive02`复用同ZIP可逆归档两stage，normal产物不动。ZIP在外盘、内盘保索引，不称独立盘备份。

关键事实后镜像STATE/PLAN/全docs manifest与refs到I；外盘中断先读I/RESUME.md，核实际SHA/refs再继续，不能按摘要覆盖未提交内容。


## 2026-10-28 剩余拖放窄验证检查点

上一目标轮是progress（Pi/借网/全局guard真实完成及独审），未宣告整目标完成。root graph先查但669旧索引缺TS/config覆盖，已仅实际读取native-events/window-drop/api/windowbuilder/config，未见显式禁拖；这不证明实际OS送达。`P5/drag-verification-followup01/root-preflight.json`保存源SHA和边界。`node-open-os-owned-plan03`只5个namespace/reference数据键变化，原run/launcher/observer字节同，非作者静态复核通过；fresh run03/TMP knopos3当前未启动。待q043原ChatView/fixture挂载条件窄核后，root仅最后一次文件图标→输入区实际拖放；不重跑既有guard/Pi/借网或改ACL/断言。若仍0事件保未证明，不靠重复相同gesture制造进度。

原OS plan03 exec91713已terminal exit1；不要重用session或任何dead CUA对象。source/normalproducers未改。


## 当前需外部输入的验收边界（第二个连续目标轮确认）

- 本轮graph-first窄核排除了显式禁拖配置/fixture capability gate的静态候选。q043只读报告`P5/Q043-drop-source-readiness01/report.json` SHA c1f8238fa99702860ccde17e656ce939c95079acfcc80a6a02d71ba4be626d75：ChatView直接listenNativeDragDrop，live inApp且非still/gone挂载；text-file落地不受imagecapability限制。它仍不证明实际OS事件送达。
- 最后一次具体不同手势已执行：`node-open-os-owned-run03`/exec91713，实际选中Finder IconView文件图像→相对坐标[250,98]拖到左侧composer[-580,1378]，仍无附件/native_drag/chat_put_paths/term_drop。用户已被邀请在具体live Lab中手动复核，等待超过60s且原exec/PID反复确认live；未收到回复/实际事件，不把等待当通过或批准。到原时限已关闭任务Finder新窗、用户原窗保留，当前不能再按旧邀请拖到已关Lab。
- 原helper270.08s/101先于末尾root退出动作，App32738由原failure路径SIGTERM15；随后CUA读到空窗口可能重唤私有同App，紧接CmdQ，实际ps确认无sameprivatefootprint。不能把root描述中的“before deadline”意图当实际normalQuit。run03 result `10dd44360e8076957ab5c5cd9c3b3196c3ca40813e03aa5ad61181b9e6028f00`/275.4389s，source/helpers/bundle稳/原cleanup remaining-footprint-TMP清/manualfalse；原outer最后publish双盘断言失败，原result/manifest不回填。
- 独审已确认实际双盘差异：同875-entry manifest；外盘.DS_Store在freeze后晚写变更，内盘保期望；root-ui-drag-result.json位于helper/launcher已终止与manifest之间，E有原manifest期望字节、I缺失。两版DS_Store和rootpartial原bytes/stat将保到独立review，不补原I以充完整绿。
- 当前剩余同一阻碍：自动实际drag始终无事件，需要人工手势/外部操作条件改变；既有fullscreen grant缺口又在“新问题下一轮”边界外，却阻原OS全契约。已通过异步问题请求用户裁定是否把该权限缺口纳入本轮，或保留下一轮并接受带例外交付；预选/等待均不算批准。该处等待已被后续用户“纳入”决定替代；按下方最新范围执行。
- 上一目标轮在完成Pi/借网/guard后已明确同一OS验收缺口，本目标轮又以不同手势及源条件核实，连续确认计2；本轮有具体新证据，仍不满足complete，也未到blocked三轮门槛。后续若无用户输入/外部条件改变且同一真实阻碍第三轮持续，按goal规则核定blocked，不继续靠重跑/重复状态保active。


## 最新用户决定：fullscreen特定查询grant纳入本轮（2026-10-28）

用户对具体范围问题回复“纳入”，即授权修复fullscreen现有权限缺口并保原真实false/true/false验收，PLAN已追加原话/作用范围；**不再以无授权阻挡这项实现**。其余新独立问题仍下一轮，main/ship不变。旧用户人工拖放邀请因原270helper结束已撤销，不能据旧窗口操作。

root native槽目前空；q043在复用作者树overwrite-recovery/e110新branch fix/quality-fullscreen-query准备最小capability JSON grant，原fix/quality-node-open-admission必须保留。不得加default权限/其他query或改native事件代码/原断言。候选交非作者独审后root集成，再刷新新source正常GUI/Core与原OS实际验收；不重跑不受影响的115/14/Pi/借网。completion_audit接手OS3完整差异报告收尾（app_runtime因capacity errored，未改原件），随后候选非作者审查；两agent禁native。连续阻碍2轮的旧判断被本次真实用户scope输入改变，当前有可推进的授权工作，不能据旧计数mark blocked。

当前唯一native/Cargo槽：root Coreproducer exec13238 live（新source ecab/2389，原runner0f421…9165；传入新实际GUI resultd159…522f）。GUI indepreview实际生成out/capabilities.json中fullscreen-state仅main/query、acl-manifests允许is_fullscreen已证，不是仅看源码。原Q055 gap保。Core已启动，禁止其他agent编译/服务或改target；待实际Core完整来源后才启动App。
