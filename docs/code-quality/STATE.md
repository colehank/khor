# 第二轮当前状态

更新2026-10-31。目标 **完成第二轮 active：29反馈＋5遗留＝34项**，未完成。用户要求已开始的做完，新独立问题只入NEXT；不考虑旧端兼容。**无本轮main合并/ship授权**，main发布基线e7c335c8不动。

本页仅当前恢复入口。逐项要求/最小证据读[验收总账](ROUND-2-ACCEPTANCE.md)；执行规则读[PLAN](PLAN.md)、[工程约定](ENGINEERING.md)、[存储](STORAGE.md)。此前完整逐批状态/原红及来源细节已原字节归档[43ff前检查点](history/STATE-before-43ff-checkpoint-20261031.md)，SHA aa4f4d21a237942ae79ec1e840cf21a0f245b48b601bb1432d508d93ad4e3134。不得把归档的旧“当前”/live语句覆盖本页、实际Git或实际运行handle。

## 当前代码、所有者与实际运行

- Root：`/Volumes/disk0/Khor/worktrees/current/khor-round2`，`integrate/round2@43ff9baa902614ed1e7e1baf53fa13baef22ed60`，已核clean。Root协调、精准cherry-pick、唯一实际运行/GUI；作者不得自主Cargo/App或改Root。
- H：`khor-round2-history`/`fix/round2-chunk-grouping`，GUI/Core/平台。最新author aeee1051已review→Root90f80429；当前只源准备/编译器具体调用点支持。
- input：`khor-round2-input`/`fix/round2-recovery-contract`，Node/ACP。author687e0e07已review→Root43ff9baa；当前只两个normal/focusedNode运行适配，不能把source可能红线索冒实际红。
- reviewer：非作者源码/真实尾核与CleanCode，不能自主运行。职责卡可能dirty，**不全局stage/stash/reset**。
- **所有运行槽FREE**。最新native actual35829已终态1/102.664s：metadata0、compile0、唯一ignored exact实际0过/1失败/0ignored，失败为业务Unknown/Native而非期望Unknown/None；无timeout/forced。原handle已关闭，不重启。此前55254 normalCLI＋bridge两个producer成功且已独审。input仅修既定S资格门，reviewer独立审；作者无自主运行。
- 最近browser actual7832终态0；最近App80119原300秒超时终态1，owned/footprint无、TMP absent。当前没有私有App。后继真实状态优先看实际handle及内盘`current-runtime.json`；超时观察不是进程终态，**poll原handle，不因观察超时重启**。

### 当前源变化与下一步依赖

1. 874c：exactChild waiter修复＋W019移除clean常驻Reload，已独审/定向与实际Unsupported闭环。
2. 0f606225：Quit改显式菜单走主窗口草稿门；aux关闭推迟main Destroyed，Cancel保Settings/code/overlay；缺main只逐窗guarded close，无强exit/新状态/RPC。实际三fileowner红→绿已闭环，见下表。
3. 90f80429：W019冷启动左树初值随同validated/restored `place.ws`，仅App初始化/必要注释；wide恢复workspace与右文件时左tree一致，narrow原剥ws仍list，Back/普通打开/tabs/drafts未变。源码通过/当前fulltypes通过；**新GUI启动/Back实际尚未验**。旧81735/33349启动左list＋右restored file是呈现红；不能据AX无aria-selected错称openWs=null。
4. 43ff9baa：仅追加Mac ignored exact `live::proc_identity::host_crash_tests::a_crashed_host_cannot_restart_its_exact_live_foreground`（1父/0helper）；原生产与旧测试前缀逐字同。真实normalCLI/Core HTTP、现stdin-stall、原生精确birth/knownPID sysinfo、host-only SIGKILL、provider仍活→期望Unknown/None＋typedUnknown、原raw/binding保持；错误也先精确清理/冷保留已登记crashpart再传播原业务红。**已实际编译/运行35829并业务失败**：host64092按精确身份SIGKILL后Absent，原provider64093同pid/group/boot/start/image/UID/argv仍Present且parent→1；execution Unknown通过，但1171断言action实际Native、期望None。后续显式恢复/replay/历史末断言未执行，不称已发生重复spawn。

55254两个normal及focused合同已实际准入并通过独审；35829使用同实际embedded bundle与TAURI_CONFIG，原require/来源检查通过。最新原result b739373b7423090e189a70e1f3b5e9de7dd74805e814d6c315d4729f0ac4dbe4；原日志25b37a1a…63aa3；observed ae56c37b…b6ec6。清理ea2c4d5e…749aa成功，所有已知host/provider/resident角色已退出；212B crashpart按原inode/SHA冷保留781cba76…a90cd（明确childTMP未空），outerTMP空、无残留/force、源2340/env稳定；私有test副本已严格退役。原false不改。input仅修既定S offer/action与显式恢复共用资格，reviewer独审，随后只受影响验证；不借机开generic Close/协议/存储新项目。

当前188gen沿actualcc92精确复用，当前43ff fullnoEmit＋新Vite已实际7832通过；不同源/生成/前端/原生/App各自分列，**旧0f606/874的wholeframe原proof不能对43ff盲require**。最终GUI等待host-crash事实/必要修复，避免中间反复GUI构建；最后真实Wstartup/Back及剩余UI。

## 已闭合的主要实际证据与边界

权威原件统一 `/Volumes/disk0/Khor/evidence/round2`，所有旧false保原件，不回填。下列有非作者盘尾核；界面观察来自Root实际CUA，不冒reviewer目击UI。

| 实际范围 | 已到事实 | 明确边界 |
|---|---|---|
| 前端7832，当前43ff | 15.250s，fullnoEmit0/Vite0；actual188gen复用、新278dist完整dict同receipt，2340源/env/tools/helpers稳；19priorpins与exact16 gen源码差已核，owned/TMP净。result e21360e0…044c2，receipt f59ef37f…57757 | 无Cargo/gen/行为/App；当前GUI尚未构建 |
| normal三producer37775，0f606 | 280.043s、5phase0，3 no-prior/normalV/实际bundle26；GUI6017b27a…e1d40(393050680B)，CLI7ace6db8(359304072B)/bridge4378e065(343548664B)，新OUT36359960/283file与fresh marker；188gen/278旧dist、源/env/owned/TMP稳。result2ed3567f…00159 | 属0f606，不能冒当前43ff整帧；本批源引用前端3b6 |
| 三fileowner实际退出保护，0f606 | W88179含Settings：真实FAILED-DRAFT/os13，Settings CmdQ→主窗NotSaved仍活，Don'tquit保完整草稿及Appearance＋Khor两窗；S81735真实EditorSession内容页；M33349真实Machines/Files/EditorMachine。各自normalSave/failedSave/Cancel/恢复权限/qSave/r后立即CmdQ，实盘全文FAILED-DRAFTqr；正常exit0、无force/timeout、源/env稳/owned-footprint空/TMP absent，三个精确parent0755恢复。 | 仅RootASCII，不替Mac真实中文IME/真人选区或手机。旧44266是真红：已接纳草稿＋真实os13后Predefined Quit直接terminate绕过草稿门；修后绿不回填旧红 |
| 完整Unsupported4303，874 | 6.260s；Init4/New1/Load0/Resume0/Prompt1，2次不同epoch Refused各同payload重放无新Init，旧Meta/base/binding/raw保，两host/provider原5s ANY PID/group含Z全消失，bridge期间活，最终noforce/owned/TMP净。aggregate2e464e00…a5d43 | 受控真实CLI/Core fake-peer完整该分支，无真实vendor/API。旧85368实际Z与33238 lossy `(Python)`资格false保；Unknown图像不授信号 |
| Shell9775，17f | 8.756s；真实devices.name/id路由，A命令仅Ready后一次；缺cwd DirectoryRequired＋同payload无spawn；fresh cwd/token开唯一默认/bin/sh B、空startup、previous/rootA，live/ended重放同B，旧A五语义文件/marker保，两PTY实际回收。aggregatec68daa86…536c3e | 本机真实PTY公有入口，不外推GUI/远机；旧12290空machine400无A原false保 |
| 完整两home Offline63373，0f606 | 38.783s；同Mac两真实Node/不同DeviceIds配对；唯一Pending400后原ticket仅一次completion(原60总界)，opaque registry快照保，id/nonce复用依已审源码而非伪decoded字段。online Join/replay→只停execution resident而host/provider仍活→exacthome Offline/None/noepoch＋原epoch3 Unknown；reader executionfiles/meta空、remote原Meta/Attempt/owner及SDK数保。samekey/DeviceId重开后原payload两次Join同SID/run/host29857/provider29858；Init2/New1其余0无增。所有新增后代仍strictowned、全部实际退出、双TMP净/noforce。aggregateb484b4db…/scene2f7f202b…d8115 | 不是物理双机、host崩溃/provider仍活、重启或真实模型。旧94947/27250 false、part/EPERM保；不把合法metadata后代一概当provider或豁免Z |
| 公有Load70988/Resume57844，17f | 原完整业务断言实际到且独审；同SID新run/Load或Resume仅一次、同token无额外writer、原opaque输入/旧record/raw保；旧原result false仅已知自有why诊断，另独立冷保留收口，原false不改 | 不重跑已证且未受影响的业务；post-ready why生命周期F1、generic close等独立问题只NEXT |
| 真实桌面80153/长历史96355与17256，17f | 71提交分页/独立固定预览、39文件单滚动/跳转位置、4种数学分隔符、巨条184613B原文系统Save逐字同；真实67,110,120B/53262条native历史尾页、取消、未加载HEAD全文搜索定位/WindowRaise与滚轮可读；正常Quit/源/TMP净。 | 源保持的映射复用；长历史parent RSS是单点，不冒WebContent峰值；旧paint不足唯一因果unknown，跨mount bookmark仅NEXT；系统Copy仍待 |
| 真实B/G有限 | 原URL lan.cibol.khor.cloud/guest在80153真实WK加载访客页，dark首帧黑、进度结束；闭端口56757实际拒绝＋Retry。91690当前0f606前继874同源相关UI：W019无clean常驻Reload/More仍有，6外观diff可读，后续材质和Light→Dark保持file06/07。 | 原历史拒绝原因unknown；完整progress trace/light背景/目录建议材质/手机仍待；首dark file05→首light file06位置差不冒像素稳定 |

### 新增真实内容交互观察80119（整场超时false）

0f606原300s场已终态1/原force timeout保，owned App/footprint无、TMP净/源env稳。Root排步过多，最后WK未做完；**不是产品已证崩溃或整场通过，不重跑已到行为**。
- 路径：长Directory path滚到真实末尾，鼠标从中段拖到框外松手后仍combo编辑态，Selected `acceptance-data-1/EditorSession`保；真正外点退出编辑。
- 网址：长未提交URL同类实际拖选松手，Selected `=abcdefghijklmnopqrstuvwxyz`保，text field仍编辑，无推荐覆盖弹层。该query未作网页成功声明。
- 当前目录EditorSession→输入EditorMachine→左上Back实际回EditorSession/files。
- 搜索不存在文件→Nothing here；首次blank click关闭搜索且原files恢复，Escape同样清过滤/退搜索。
- 未验：网页真正导航/首帧、目录建议材质、消息普通/Cmd点击、真实终端软折行。result23bc0db1…a4185，UI observations79f6233e…1fd0e。

早期受控/定向绿（Core44421、CLI59004、Node85431原8＋2、Recovery13057真实IDB17、H28700/Find32/Reader28/ACP4theme等）的范围/原false/源码映射在归档及总账，不因新test-only/UI初始化改动重全仓矩阵。

## 剩余原34项，不缩小成功标准

- S：当前host死亡而provider仍活的真实资格/typedUnknown、新叶实际及必要最小修；完整App恢复组合按总账补。63373已证resident重启不能代替host-crash。
- C/E：消息路径/URL普通与Cmd实际意图；真实终端softwrap；Root路径/网址拖选已单列观察，Mac中文系统IME/真人操作仍pending，不能typeText冒IME。
- W/G/B：新Wstartup/Back实际；sticky/menu剩余代表主题与建议材质；真实WK完整progress/light/失败重试与列表删除；手机同页viewport/左滑、跟手与键行。
- 手机：最近实际inventory iPhone15ProMax/iOS27 paired+developer但tunnel unavailable/DDIfalse；本地SDK26.5与ios/sim Rusttarget已装，不当SDK唯一原因。现iOS Rust改动仍需当前cfg编译；可按既有同qualitytarget模拟器compile-only计划，无下载/新cache/boot/install，不替手机。
- R2-001旧卡死原栈缺失，R2-016原历史拒绝唯一因果仍unknown；保限定调查，不以别的绿填修复。若无法闭合，须用户明确接受延期再改范围。
- 最终34逐项完成审计/非作者CleanCode增量、再按用户新授权决定main/ship。当前没有发布授权。

非作者[CleanCode增量0f606](reviews/R2-CLEAN-CODE-0f606.md) SHA0f041a3d…a060b：5源路径按唯一owner、IO/规则、错误清理/迟到身份、删除重复与必要原因注释再核；不以零注释/格式数量当目标。新861行私有host test有阅读成本，但现helpers不能保本例真实CLI/孤儿/HTTP/原生birth/冷part义务，未发现可安全删的无用重复；一场私有Mac cfgtest，不开新框架/通用killer。最新35829实际编译通过、业务红与清理事实已独审，最小生产修复待完成。

## 存储、恢复与防重复动作

遵[STORAGE](STORAGE.md)：每批fresh外20GiB/内10GiB保留＋显式峰值，5GiB仅紧急线；唯一quality target/jobs1/incr0/offline，无新target/worktree/完整binary gzip或内盘镜像。最近df外约107–109GiB/系统13GiB；quality约41–43GiB，50GiB先复核；内recovery约2.2GiB，5GiB先复核。

旧17f9014三个canonical＋旧App2hardlink已按UID/dev/inode/fullSHA/lsof/ownerfree严格退役，外实际+1,095,593,984B；source/userdata/releases/proofs/logs不动。receipt `/Volumes/disk0/Khor/maintenance/storage-20261030/retired-round2-9014-20261031.json`。当前0f606三canonical/KhorQuitLab用于已证实际，874旧产物到期可后续严格淘汰，不能扫未知目录。

OUT两次历史单目录可逆留存各有独立合同/真实结果；最近Quit前283/6,602,979B retain ba288244…7a11b2，新GUI37775已真实生成新OUT36359960。**source_absent只留存时事实，不重复move或造mtime/prior**；55254实际无GUI/OUT动作。首次Root stale appRUNNING记录挡在mutation前的准备失败另件保，没有双move。

当前source bundle `workspace-host-test-43ff9baa.bundle` 相对0f606，SHA 06f8d87880ceb71f437a1fde5a3d6d377c8bcc7aad1cb720c43fc214216ee8f5，14148B；已git verify0/实际外内双盘同SHA。前一nativeQuit bundle相对874仍在。内盘入口 `/Users/zgh/Library/Caches/khor-code-quality/recovery/{STATE.md,RESUME.md,current-runtime.json,latest-source-bundle.json}`，只紧凑状态/源码/索引；原始日志与截图唯一外盘，不复制payload。

CUA特别规则：mac getApp、已绑定App的AX/screenshot/动作都可能ensure-launch。**每次绑定先外部实际PID/birth/fullpath活性；预期Quit后只查PID/原handle，不再CUA观察dead App**。70472后误重开90081已独立正常Quit关闭/parent还原，旧false保；后续三owner没有重启干扰。所有fixture权限当前已恢复0755，无用户源码/会话/模型API操作。
