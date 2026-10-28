# run11：精确集成版真正iOS App验收计划

所有者：native_acceptance。2026-10-25开始、10-26继续。root已完成定点缓存清理并授权执行，冻结runtime HEAD：`c37151a1ed90abc81ce7c119994912574e16163f`。以下计划保留，当前执行事实以本节为准：**同源Rust和隔离App已构建/安装，正在分项真正UIKit验收，不提前报全绿。** main/用户App/旧c3未改。

## 当前执行账

- Rust PID/PGID80685，58.40s compile、60.12s stage、exit0，source_changed=[]、git clean。新`libkhor_gui.a`与run11 APFS捕获lib同SHA`b8ff8ef85843c38531340dd664e795f773f37b9ee32ece47b1dc4468618719b6`，1,010,946,624B；与旧c3不同。
- APFS clone原隔离ios/DerivedData到run11，旧c3 lib/project/App的inode/bytes/mtime/nlink在clone后及Xcode后逐项未变。没有用硬链接或用户App。Xcode build-for-testing PID85950，15.05s exit0；真实link路径指向run11新捕获lib。当前App debug dylib SHA`b586aef63fe070a6875b5f8931e359ca2055bee784c494a557f2cc81381387fd`。
- 新App只安装到专用simulator quality bundle；`KHOR_HOME`为其自己data container下`Documents/quality-run11`，没有复用旧质量store或复制用户身份。fixture PID/PGID88689，pure-origin23840；完整构建槽已归还root，只运行test-without-building。再次重编需重新获槽。
- **Q015第一次严格boolean断言失败已保存**：testNativeStartupNotification exit65，实际native query返回null，已无notification ACL拒绝；不是产品权限红。锁定notification2.4.0的commands.rs实际`Result<Option<bool>>`：Prompt→None；init-iife也明确null→Notification.permission='default'。root承认此前boolean要求过窄并批准按原契约修harness，不request系统权限。原driver/metrics/server/xcresult及完整primary源码快照保留于该失败case。
- 经root短槽仅Swift driver重编PID93841，6.01s exit0；App三个hash与安装时完全相同，Rustlib未换。harness-v2保留raw bool/null，检查Notification.permission对应granted/denied/default；notification-contract分项已exit0：原始native query=null，Notification.permission=default，main窗口实际Tauri IPC成功、无notification ACL拒绝；该次9.28s testcase通过。driver SHA`18cabefcad8849bd62d163bb2d5f468c4c6e3b506c78e671d31e1b61a9845e99`。
- **Q010真长稿手势已通过**：testNativeLongDraftUserHide exit0，288字、六个requested settled gap严格0，真实reader drag gap188；268帧native trace，500ms后所有gap0。动作是XCUI downward composer touch，XCTest确认UIKit消失，无DOM blur helper。源/harness在该轮未变，case/acceptance.json与events.jsonl保留。
- **新独立Q043实际启动异常**：`listeners[eventId].handlerId` unhandledrejection，完整user-script9/tauri _unlisten/api.ts stack与触发顺序在原events/附件。root已派store_owner只读审查，不归到notification cap或mock。Q015单项可按合法契约验，但不能把完整原生启动称为无异常。
- **Q010/Q011全套真正App已通过**：`keyboard-commit-long-fixture` exit0/test86.58s；ACP与Khor各三次UIKit空稿开合、长稿、reader历史优先均通过。ACP历史gap188、Khor gap597；键盘开合保持同一anchor。commit第6与第8文件通过真实滚动菜单/实际触摸命中，四次settled分别top3348/focus=`src/module-6.ts`、top4511/focus=`src/module-8.ts`。未用DOM click/scroll纠正。
- **数据前置不足失败保留**：原`testNativeKeyboardAndCommit`在Khor reader因短fixture的scrollHeight=clientHeight=510而无法读历史，exit65；不计产品滚动红。其后只注入60段Khor数据，未改源/harness。初次ACP用history-after-history_end未渲染，故该次不宣称长ACP回复覆盖。
- **真正长ACP回复+长草稿手势已额外通过**：`acp-long-reply-user-hide`通过合法live agent_message_chunk数据注入，真实scrollHeight5578，288字草稿，六次settled gap0；271帧原生trace，500ms后gap始终0。实际XCUI向下composer手势收UIKit，无DOM blur。原失败、错误数据形状、后修正live数据均另存。test-without-building exit0/stage27.06s、harness/source无变化。
- **Q035 direct真正native WK 4/4通过**：冻结生产BrowserPane/held/native-page-open，测试运输白名单直连真正Tauri invoke，未替换internals。实际14次open_page：late open先真实page_steps Some，再退owner/释放真实结果后null；StrictMode同样；真实close返回门控期间新owner等待，释放后新WK持续Some；10逻辑tab最终8真实PAGES handle、最旧2null、最终全部0。完整`retention-acceptance.json`/原始逐tab快照/transform输出hash/调用结果保留。它证明已知原生WK registry句柄生命周期，不是全部WebKit heap/process内存证明。direct machine为空，实际session/addr为空，**未验真实borrow lease，不据此关闭Q035**。
- direct第一次外部TSX依赖解析失败未执行native page；`retention-setup-01`保留，随后仅Vite root dependency dedupe修harness，不改生产/App。StrictMode另复现3条Q043相同stack unhandledrejection，保留原始事件；4/4资源断言绿不覆盖启动/event异常。
- Q033真实GUI上传链尚未计过。Q043作者准备独立trace，direct server/App已正常停止并释放23840；新增Mobile Safari专用sim具体最小计划在run11/safari-plan.md，不以App结果替代。物理iPhone仍未验。

所有本轮证据在`Q010-native-20261025/run-11`，含source/harness/library/clone-protection/xcode-protection/install receipts、完整每阶段命令/PID/log/xcresult。起始外盘11,332,222,976B，Rust后约9.66GB、Xcode/安装后约9.19GB；双盘5GiB保护未触发。

## 已核对的基线与现场

- 已读`Q010-native-20261025/ACCEPTANCE.md`、run09/10来源/Swift driver/结果、Q015和Q035卡。旧卡仍有早期working/review历史，不把这些状态或旧绿移作最终集成证据。
- 专用模拟器`FE04AC7C-B375-4F2E-AE7E-B6F8F5608947`，Khor Quality Acceptance 20261025，iPhone17Pro/iOS26.5，当前Shutdown且available。保留物理iPhone/中文IME/辅助功能未验边界，UA中的18_7只是兼容值。
- bundle `io.github.colehank.khor.quality`；已有隔离Xcode工程`.../Q010-native-20261025/run-03/ios/khor-gui.xcodeproj`，scheme `khor-gui_iOS`，UITest target `KhorQualityUITests`。
- run03 c3基线捕获lib位于`ios/Externals/arm64/debug/libapp.a`，1,005,391,152B，记录SHA`430bc43b9dac78192d19ac4b4f152e5f889544d3fbdd09b78c71f2afeeb8fa3b`；原工程、DerivedData中的KhorQuality.app及所有xcresult/trace都保留。此次仅复核metadata，没有重新读1GB求hash。
- run09明确DOM blur收真正UIKit键盘，六个settled gap0。run10明确XCUI真实向下composer手势、production field-swipe收键盘，无blur helper；六个settled gap0，随后真实reader drag gap460。run08原gap164红仍保留。
- 当前run03 driver与run10/driver.swift同SHA`5e66cfdd862fc43d6b0554991ebdd67287493231175a43e03503e0e263de2b5b`；原project.pbxproj SHA`12a3b3858cdc9b5adabc606c74056b6bc56727a50b3ca04c09f6804ed12bf33f`。run09 driver SHA`ed76ae9d84c37f026ef9cebbbcbf0a06b21338bebaea19fc5de738c33d4187c6`是原红绿同driver另存，不能与后加真实hide的driver混为一个版本。

## 磁盘与准备策略

本轮预检（root清理前）：外盘APFS，df可用7,507,756KiB≈7.16GiB；内盘10,990,040KiB≈10.48GiB。外盘距5GiB停止线仅2.16GiB，**当前数字不足以承诺Rust重链峰值一定可容纳**。root已通知将先定点回收约3GiB；我未删任何缓存。

只读du范围限定已知三处：旧run03 DerivedData467,576KiB、ios982,448KiB、quality-target/aarch64-apple-ios-sim7,816,628KiB。旧App主要debug dylib381,888,888B；主入口37,704B，不能仅看主入口大小误算App。

待root授槽后用APFS clone复用**已有隔离工程**到证据run11（不是新worktree），同样clone旧DerivedData或仅用新路径复用编译缓存。clone保留旧inode内容的CoW快照；不把旧lib作为新成果，不使用会随后被Cargo覆写的硬链接或不受控symlink。只在run11的Externals替换为本轮新lib，原run03完全不写。clone失败则先评估完整副本成本并回报，不静默复制1GB再撞停止线。

每个Rust/Xcode阶段都记录自己的PID/PGID/命令/完整日志/开始与最低free；jobs2、禁止并行test。掉盘、free<5GiB或本日志连续3次I/O错误只停自己PGID并保存失败。每阶段开始前重新df；root未再授槽时不继续下一次完整build。

## 同源构建、安装与证据链

执行顺序固定如下，生产源在整个build/UI窗口冻结；CI/docs若另提交，也必须证明runtime树未变。

1. 记录完整c37151a1、tracked/untracked状态、Cargo.lock、native capability、Rust/前端相关文件SHA及fixture/driver SHA；生成run11/source.json与harness-source.json。确认源树没有旧test注入、无生成器/生产build漂移。记录xcode/rust版本与simctl runtime。
2. clone旧隔离工程到`Q010-native-20261025/run-11/ios`；保留无Cargo shell phase的现有捕获工程（已有project不含xcode_backend重复编译）。准备run11独立DerivedData、logs、xcresult，不写run03。
3. run11配置保持纯origin：

```json
{"identifier":"io.github.colehank.khor.quality","productName":"Khor Quality","build":{"devUrl":"http://127.0.0.1:23840"},"bundle":{"active":false}}
```

4. 唯一槽内首先从冻结wave1执行确切Rust命令（TAURI_CONFIG传以上文件的JSON内容）：

```sh
CARGO_TARGET_DIR=/Volumes/disk0/Khor/caches/rust/khor-quality-target \
CARGO_BUILD_JOBS=2 TAURI_CONFIG='<run11 JSON contents>' \
cargo build --locked -p khor-gui --no-default-features --features devtools \
  --target aarch64-apple-ios-sim --lib
```

5. 只在这次成功后，把`quality-target/aarch64-apple-ios-sim/debug/libkhor_gui.a`以APFS clone捕获为run11 `ios/Externals/arm64/debug/libapp.a`。记录新lib来源、bytes/hash、构建日志与冻结源对应；不能复用旧c3 archive或主仓release。记录link search path仅指run11捕获路径，并保存Xcode实际link日志。
6. 同槽串行Xcode build-for-testing（项目为run11工程）：

```sh
xcodebuild -project <run11>/ios/khor-gui.xcodeproj -scheme khor-gui_iOS \
  -configuration debug -sdk iphonesimulator \
  -destination 'platform=iOS Simulator,id=FE04AC7C-B375-4F2E-AE7E-B6F8F5608947' \
  -derivedDataPath <run11>/derived-data -jobs 2 -parallel-testing-enabled NO \
  CODE_SIGNING_ALLOWED=NO build-for-testing
```

7. 核对最终.app Info.plist bundle、Mach-O/dylib hash与新lib link来源；只安装到专用simulator的quality bundle。允许替换该测试bundle的已安装旧副本，**原c3文件artifact仍保留**；不碰生产bundle/用户窗口/用户store。将本次`KHOR_HOME`指向quality App自己data container里的run11独立子目录，旧quality测试store也不复用为新状态。若使用自定义root先验证sandbox/owner边界，不复制用户身份。
8. Fixture就绪后分项`test-without-building`，每项独立resultBundle/日志，沿用同一新App/Rust lib；只改Swift driver时也记录新hash，不能悄悄重链接旧lib。继续设置`KHOR_NO_MDNS=1`,`OS_ACTIVITY_MODE=disable`,`OS_ACTIVITY_DT_MODE=NO`，避免run03已证明的XCTest日志递归启动阻塞。

## Fixture边界与顺序

复用run10纯origin23840 + early history.replaceState bridge query + observer enforce:pre + tauri://localhost CORS。devUrl绝不再带query（旧run02已证明Tauri mobile proxy path问题）。冷启动先给DOMContentLoaded/真正composer最多45s，再做断言；12s仅底色的setup失败不计滚动红。

**关键运输边界：** production api.ts的bridge分支优先于inApp，故旧bridge23840所有业务调用都走HTTP fixture；它可以供固定消息/commit数据，但不能证明page/borrow或上传经过Rust。run11要在Vite的**测试专用transform**中只对明确命令白名单路由实际`@tauri-apps/api/core.invoke`，其他数据继续fixture。保存transform差异与返回模块hash，不改生产api.ts，也不覆盖`window.__TAURI_INTERNALS__`、viewport、UIKit行为或scroll逻辑。

Q015显式query与Q035 page_steps等观察也直接invoke；不得借HTTP返回boolean/计数冒充原生。

## 分项验收与严格断言

| 项目 | 操作与必须证据 | 不据此宣称 |
| --- | --- | --- |
| Q015冷启动 | capture bootstrap/error/unhandledrejection真实window/webview label、原始reason/stack；DOM就绪后直接`plugin:notification\|is_permission_granted`按锁定官方Option<bool>契约返回bool或null，并保留raw值和Notification.permission映射；启动+稳定窗口不再有ACL拒绝；固定源码cap只允许main/menu/settings的readonly query | 不调用request/send制造boolean；不称物理设备/其他窗口所有通知行为全过；首次过窄harness红保留 |
| Q010 ACP/Khor空稿 | 各三轮XCUI点击真实composer、XCTest确认UIKit keyboard存在、关闭后尾部稳定；保留visualViewport+scroll/composer bounds实测 | 浏览器synthetic viewport、DOM焦点不代替系统键盘 |
| Q010长稿真实hide | 真正typeText扩展288字草稿，真实向下composer触摸调用生产field-swipe；XCTest确认keyboard消失；原六次settled尾部断言不提高gap阈值，持续rAF trace至少覆盖旧266ms位置恢复窗口 | 不拿显式DOM blur说成用户手势；原run09可作有标签的补充，不替代run10路线 |
| Q010 reader优先 | 真正scroller drag读历史，再开合键盘；保留anchor，不能自动拉回尾部；长稿hide后也继续真实读历史 | 不能只看到gap0就说历史阅读正常 |
| Q011 commit6/8 | 真实滚动clipped menu使目标row可见，elementFromPoint确认命中后实际tap；多次settled位置+focus仍是目标文件，截图/事件/frames/xcresult保留 | 不直接DOM click/scroll到文件，不把旧driver误点当产品红 |

采样只取`event=requested`与对应label的明确断言包。连续scroll包会暂保留前label，不能取最后同label包当settled结果。全套结束前不提前报全绿。

## Q035真正native页与borrow

已核对：iOS不是Tauri子webview表，而是`phone_page.rs`主线程PAGES持有`Retained<WKWebView>`。现有`page_steps` → `phone_page::steps(tab)`直接查真实PAGES：Some为该native handle存在，null为无该handle。因此无需新增debug permission/生产count命令，逐个测试tab轮询即可证明**本次已知标签的native registry句柄**。这不等于证明所有WebKit heap对象/进程立即释放。

先跑direct机器空串的真WK路径：BrowserPane/held/native-page-open均用冻结生产模块，所有open/move/hide/close/show/page_steps路由实际Tauri IPC，内容为本地轻量HTML。保持标签desired集合与真实点击/切换，在晚到open、StrictMode、快速关闭重开和10逻辑tab/8活页cap各阶段，对完整已知tab集合取原生Some/null快照；所有关闭后必须全null。XCUI webViews数量或React ledger size只作补充。

晚到场景可在fixture运输层**先等待真正native open完成、确认page_steps为Some，再延迟交还该真实结果**；在此期间关闭owner，释放promise后生产late-hold必须最终关闭实际WK。仅延迟真实完成结果，不造一个page/borrow对象。快速重开也只门控真实close的返回，随后验证新URL事件/实际WK仍活，不以close调用次数代替结果。

borrow必须另接真实resident：`open_page(machine非空)`调用Rust borrow_web→Node::borrow_unlisted；该API**要求驻留serve，没有bind-own fallback**。iOS质量App用独立store和独立Mac出口peer（同冻结源构建），通过真实invite/pair/endpoint连线；复用`tunnel_wire.rs`的隔离配对方法，不用用户网络、账号或已安装旧release冒充新版本。

具体资源证明：捕获真实WebBorrow session/127.0.0.1 proxy addr；两个tab同出口必须同session/addr；通过代理发送一条本地echo/HTTP请求证明端口真工作，随后关闭探针连接。关一个owner后代理仍接受连接；最后owner关闭后真实`close_web`，轮询新TCP连接必须拒绝，重新打开应得新的borrow session。不能用“session列表里没borrow行”证明释放——unlisted本来无行；也不能要求共享QUIC wire都关闭。若无真实resident/peer准备，direct WK可独立验收，**borrow明确缺项，不关Q035**。

前置缺口：旧Swift/HTTP fixture没有用于驻留Mac出口的同源worker控制面。执行阶段需要先确认冻结源的CLI/独立测试worker可运行、IPC store和配对均隔离；若需另编Mac helper，先报确切命令向root协调串行槽。此计划不授权现在启动它或写新生产观测接口。

## Q033真实GUI上传取消的夹具评估

现成`check-chat-cancel`/`chat-cancel-wire.mjs`会安装假的`__TAURI_INTERNALS__`，其HTTP/null与frame注入只证明React恢复语义，不能用于此次“真实App→Rust→host”。

可以复用`apps/gui/core/tests/chat_upload_cancel.rs`中的真实gui_host_main+fake ACP Python+TCP first-Part gate，但当前该fixture随单测试收尾，没有可直接驻留给iOS使用的一键worker。需受控常驻worker（同冻结Rust源）导出私有addr/cookie/ready与release控制，iOS quality独立store中仅seed此fixture的session记录，或建立隔离mesh far session；不得拷用户store、把Mac owner stamp冒充iOS owner。能否用local session metadata接loopback host要先验证attach/far_machine实际分支，不能只因127.0.0.1可连就称链路成立。

真正验收必须让ChatView的chat_open/poll/wait/say/stop/leave走native IPC，真实大上传卡在writer/first Part，然后用户点Stop；Rust host/fakeACP日志必须证明session/prompt=0、没有SendCommitted，UI显示可恢复草稿；下一新token prompt可正常完成。legacy路径需真正握手降级与原attachment退役/显式重连。若只有HTTP mock或synthetic send_cancelled，这一项仅标UI证据/待真实链，不混入Q010/Q015完成状态。

## 收尾与交付

逐项独立PASS/FAIL/SETUP-BLOCKED，不以整体XCTest0覆盖未执行项。保留new lib/App/hash/source、完整launch/build/test命令、PIDs、xcresult/attachments、native telemetry/frames、fixture差异与所有setup失败。完成停止本次servers/worker、Shutdown专用模拟器、释放槽；不删c3或失败现场。

下一动作：Q015/Q010/Q011真实App与Q035 direct WK分项已完成。fixture88689与direct server25380均已核对自有PGID后正常停止，quality App已terminate；专用sim暂保留给Q043独立trace。真实borrow/Q033链仍缺同源resident worker，不用mock顶替。Safari新driver若需重编先向root申请槽；原c3与所有run11失败/绿证据保留。

## 2026-10-26后续独立平台分项

Mobile Safari计划：仅专用sim的com.apple.mobilesafari打开本机23840同源fixture，native=false；通过XCUI真实点击composer/输入288字/下拉收键盘、长消息reader drag及保持anchor，记录Safari动态工具栏与visualViewport实际值。只选ACP/Khor各一轮严格空稿+长草稿+reader，不重复App全套commit。沿用只读metrics、数据fixture，不注入viewport或内部bridge。需新Swift method和小幅driver重编时先向root申请唯一槽；当前仅准备，不称已验。

## 本轮交付与当前资源

自包含分项报告：`/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q010-native-20261025/run-11/ACCEPTANCE.md`，最终`handoff.json`确认冻结HEAD仍c37151a1且git clean。本人无活build/XCTest/server；quality App已在Q043额外3次5s coldlaunch后停止，专用sim暂为协调保留booted。Q043作者独占其trace server，我不擅停。Q043独立证据在`.../khor-code-quality-evidence/Q043/trace-01`：native-launch.json为首10s冷启动，native-relaunches.json为额外3次各5s；所有同source/App/store，均无编译。作者已初步实测listen ack先于WK JS registry注册，完整throw根因与产品修复仍由Q043单列，不据此提前关闭。

## Mobile Safari独立执行（进行中）

root已授短Swift-only槽：只增独立Safari method，PID38870/j2，build-for-testing6.03s exit0后立即归还。safari/app-library-unchanged.json全文hash复核Info.plist/入口/debug dylib/捕获Rustlib四项保持，冻结production clean。独立server39137/port23841，未占Q043端口、不改原server/metrics。

首轮safari-keyboard-reader exit65为**输入前harness命中失败，未证明产品滚动红**：实际Safari native=false、长reply5578、visualViewport714、tail0，真实XCUI TextView.tap合成点(63,731)，AX框(58,726,270,36)角落+5；仅pointerdown/up、active始终BODY且无focus，故没有UIKit。完整xcresult/15附件/合成事件plist与finding.json保留。拟仅Safari三处tap改normalize中心真实触摸，未调断言/容差，待root重新授短Swift槽；原App已过场景不重跑。

### Safari后续harness与产物检查更正

第二轮中心实际trusted pointerdown命中TEXTAREA(193,682)、focusin到composer、UIKit真实存在，visualViewport714→376且tail0；新增Swift断言误把metrics.active的属性值字符串`true`当作tagName，已保留第二个harness红而未计产品红。一行契约修正经root授权，PID45180 Swift build6.02s exit0后归还槽；第三轮尚未启动。

**产物检查时序错误已上报root**：safari-center构建PID43162实际自动重链接了run11 DerivedData中的App debug dylib（同捕获Rustlib，无Cargo），变为33c5f6a5…，此前并发hash在链接完前采样而误报unchanged。第三轮前串行fullhash及时拦下。原专用sim已安装App仍严格b586aef6…、原UUID4464B507…，已安装验收App/旧c3/用户App/生产与捕获Rustlib均未变。日志/UUID/新旧fullhash在safari/*hash*.json及unexpected-relink-uuids.json；不能用此前pre-link检查声称post-link不变。准备保留意外输出并从已安装独立质量App原件恢复DerivedData，再只test-without-building；等待root判断，不追加编译。

## Safari最终可信红与收尾

**不能将全GUI键盘验收标全绿。** `safari-keyboard-reader-mailbox/acceptance.json`明确单列真实功能红，57附件与完整raw/Swift/observer/sourcehash保留。ACP同轮真实全部通过：native=false/scrollHeight5578、viewport714↔376、空稿开合0、288字长稿真实手势hide六次gap0、reader376/anchor保持。Khor已确认正确wide/all前台新documentId、scrollHeight5255/draft0；空稿开合与长稿打开gap0，真实向下hide后前5次0、第6次gap164。偏移在trace起4866ms（4.5s trace结束后362ms），约138ms后自行回0；非永久卡住，不放过严格断言。bad时BODY、visual714/offset0、total5255/client426/top4665（tail应4829），reader输入尚未开始。Khorreader未执行。共享hook入口为MachineChat.tsx76，尚未归因/改源，交root安排独立最小复现与修复。

第三次旧Safari tab污染共享action队列的前置失败已保留；只修测试documentId/visibility mailbox，旧未携id轮询不drain、后台raw全部留存但不作active requested/latest。此前两个tap/metrics字段错误也完整保留，不计三个新产品缺陷。Swift最终SHA9c9bb50a…，最后一轮不编译，只test-without-building。

按root授权，先APFS clone完整意外重链.app+linklog，再ditto专用sim原安装完整.app回DerivedData。`safari/restore-original-app.json`逐文件manifest全等、4hash原值；UITest Runner不回退。原bundle本无CodeResources，原/恢复/意外输出codesign strict均同资源错误，未伪造签名。最后一轮**进程结束后**串行`post-process-integrity.json`再核4hash严格原值、harness不变、c371 production clean，纠正早先并发pre-link检查误报。

本人所有build/XCTest/23841 server均停，专用Safari已terminate、quality App仍停；sim仅booted待协调。stop.json含PID51447与双盘free（仍>5GiB）。原c3/用户App/main未动。完整最终报告run11/ACCEPTANCE.md已同步Safari红。真实borrow lease/Q033链/物理设备/全Safari工具栏折叠模式仍未验，不用现有UI数据替代。
