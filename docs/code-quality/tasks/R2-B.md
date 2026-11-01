# R2-B 浏览器体验（014–018）

Owner：/root/round2_input。用户“开始吧”允许本轮实施；Root 2026-10-30从已固定M025转交B。没有main/ship权限。历史[BROWSER-EXPERIENCE](../BROWSER-EXPERIENCE.md)中的proposed状态是调查时点，不覆盖本卡最新授权。

> 当前状态见文末“B1–4 最终源码交接”；前文“待授权/未开始”均为历史调查时点。

## 恢复与唯一写权

复用khor-round2-input，保fix/round2-input和fix/round2-session-follow refs；新fix/round2-browser从Root aaf91b82d9b8a1e9b7c829579646b97a66deeb78开始。B graph minimal→semantic无相关节点，精准fallback。已读design skill/README、STATE/PLAN/ENGINEERING、BROWSER-EXPERIENCE及R2-REVIEW B交接。W由Root，S由history；不写它们的树。

当前仅B5授权：crates/node/src/tabs.rs、apps/gui/src/browse/BrowseLanding.tsx及定向script。共享core/api.rs、core/lib.rs、gen_bindings、Tauri lib.rs、src/api.ts、catalog仍history单writer；B1–4仅下面合同/路径只读，必须协调后再动。无native/browser/App执行授权，定向Node3准入已交独审，等Root唯一槽。

## B5源检查点

2ba5e9838c1eef25af716efd887c6f0b8b509c43（三源，tracked clean）。Node page pinned允许同change/root ledger事务删除，code等其它kind保原拒绝；原settle、flock、atomic rename、右邻/左邻/empty规则不另写。UI page菜单同原SideList pin+destructive delete，bare页去单close override，nonpage bare保旧close。沿answered失败note/rethrow，只有一次close，没有unpin→close窗口。

资源仍原useBrowseTabs.close→closeTab成功take→BrowseLanding.useHeld→PagesKind.useBorrowsHeld→releasePages/releaseBorrows owner并集。没有清cookie/history/site-data动作，不从单tab关闭强退仍有人使用的borrow。既有owner-union回归真实模块+受控native promises验证只退无owner页；实际OS/WebKit资源清理尚未实测。

证据evidence/round2/browser/B5-source。实际生产menu+SideList表达式旧源red（full只有pin）→新7 callback绿（bare/full×pinned/unpinned、pin切换、拒绝、nonpage）。Node3新增/迁移尚未编译：nonpage保护、page删除保邻居/异kind/失败不落盘、lastpage empty。独立Node准入contract/run.py沿既有guard/actualartifactcopy，300s/cmd、5GiB、jobs1/incr0/sharedtarget/外盘TMP，只3exact、不Core/CLI/gen/App，不切源；JS已证明业务red，不冒Rust红。

类型使用Wshared1不可变169managed+catalog4b4b，55相关输入SHA同aaf；原M165完整可逆留retained-M025-gen。首次误把producer basenames放gen根，types2/缺bindings模块已保原日志；原字节移到正确gen/bindings后types17444真实terminal0。没有改生成字节或运行producer。rustfmt初次忘edition默认无法解析旧let-chain；2024仅新测试换行、最终check0。callback首失败是跨VM Error realm fixture，注入同Error后7绿；原日志均保留。

## B1/B3共同导航合同（待共享owner协调，未写产品）

现wry0.55.1源码实核：PageLoadEvent.Started源自didCommitNavigation，非provisional start；did_finish才Finished。当前page_fail忽略真实WKNavigation并把失败发finished，iOS只loading KVO；不能由这两者捏造真实百分比/成功。

采用同一native page生命周期owner：tab + 唯一view实例 + WKNavigation身份，URL只目标/最终位置；reload同URL也新导航。以单owner revision排序快照，JS只projection，跨view/迟到callback拒绝。进度取真实estimatedProgress KVO，同导航max单调，成功finish才1；失败/cancel独立终态、不能借30s ceiling冒成功。重定向同WKNavigation保progress；主动新navigation重开。Web没有native页时只表示原Khor errands，不生成假站点进度。已有present/hide/lateborrow/retention owner不改。

拟公开PageNavigationState（在core/web.rs定义并由history统一export）字段：tab/view/navigation/revision标识、phase、requested_url/final_url、estimate可空、error可空；标识采用decimal字符串不丢JS整数精度。phase opening/provisional/committed/finished/failed/cancelled明确区分；estimatedProgress只native真实值。拟page_state(tab)->Option<PageNavigationState>供重新挂载/复用快照，事件page:navigation传同DTO，避免新listener漏原start后假加载。具体共享签名先Root/history裁定后接，不手造gen或旧兼容协议。

错误拟PageNavigationError{domain,code,description,failing_url,underlying[]}，链最多4级、每字符串UTF8安全4KiB、显式truncated，实际exit与proxy存在性取已有view配置，不含代理凭据。-999/102可表cancel不发用户失败；-1004 cannot-connect与-1005 lost分别正确文案，证书trust/download原道路保留。真正lan.cibol失败必须在受控真实WK复现原NSError才能归因，curl200/截图“拒绝”不证明来源，不自动改URL/出口/ATS/权限或加可达性门。

候选叶：新私有src-tauri/page_navigation.rs（纯状态合同及platform adapter唯一owner）、page_fail.rs（现delegate adapter/NSError）、phone_page.rs（原iOS delegate/KVO/原生deck ratio）；browse/loading-rule.ts/use-loading.ts/page-fail.ts及BrowseLanding/BrowserPane、SessionBar/native-deck、app.css限定progress。Mac hook须保wry原didCommit注入scripts/didFinish回调原IMP，新增provisional/redirect和progress KVO不能吞原delegate/auth/download逻辑。不是另造第二page registry，现失败/URL/book事实应向共同owner收敛。

## B2默认底色（待授权）

创建原生view时沿当前主题提供默认容器/WK scroll/under-page背景，theme变化同步所有retained页面；不注CSS强盖站点显式背景。Mac desktop builder/repaint入口在history独占lib.rs，iOS phone_page由本owner候选负责。wry background接口底层会设置drawsBackground，实施前按真实WK白页/有色站点做像素区分，不能只set容器后称修好。

## B4手机几何（待授权）

现BrowserPane.place把底沿延伸到native SessionBar.bottom；phone inset只改UIScrollView inset，不能改变window.innerHeight/media布局。最小修将实际WebView frame限制在可用内容rect，沿现PageBox/lift、showing revision、present队列和overlay hide/restore；safe-area/deck/键盘/旋转统一实际bounds，不能再把inset当frame。DOM/原生甲板复用同ratio，禁止再无限CABasicAnimation。真实站点布局和native touch/keyboard需设备验收，纯模型只验证rect变换/owner和迟到事件。

## 分片/未验边界

1. 固定B5源＋Node3actual→独审/集成；正式列表swipe/资源OS另验。
2. Root协调history释放/接一次共享快照注册/export，再B1/B3纯owner+两个platform adapter，先真实导航身份/失败材料，后消费进度。不猜截图根因。
3. B2默认色与B4frame按同view owner接，限定真实空白/显式白站点、full/mobile/折叠/键盘/旋转/主题验证；不动W/G布局。
4. 实际Mac WK进度/错误/主题、手机viewport和M025真手势都单列，不能据source/types/Node回归销账。本卡当前仅B5实现，其余未开始。

## B5 actual Node3终态

Root准入后仅一次Node libtest actual compile→3exact，exec66876真实terminal0/83.581s，compile0及三个1/0；driver48858/Cargo48906/test51081,51089,51097均已不在（精确PID以terminal-cleanup.json为准）。前后全source/environment同，remaining owners[]、无forced/timeout、专用TMPabsent；唯一native槽已明确归还Root。私有copy actual artifact SHA356e6ca95187d8fb835656c673aa23001bb01a20f8c58e8c81f15734b2ab8a59；result.json SHA510a352c19672d3c64f729b051f79e2db6d6db664eb92e18df0648145f52818a。只称Cargo actual artifact+sourceframe，不冒未调用的V prepare/verify完整来源链。原准入runner/contract保.admitted；Root明确offline加--offline、reviewer建议dont_write_bytecode，实际driver1d87f397…32aa7/contract7cc70055…95b4d。

B5实际Rust、callback、types片可审交接；真实列表手势、WK资源回收仍未验。B1–4等待正式叶/共享slot授权，未写原生或公共DTO。

### B1事件消费者闭包补核

当前真实page:load/page:fail代码消费只在BrowseLanding、use-loading、PageControls（finish后更新导航按钮）及api onPageWord；BrowserPane/loading-rule/page-fail其它命中含说明/解析。后续更换typed事件须同片接PageControls，不留失败finish冒成功的旧信号；它也属于待Root授权的具体源。Session内页面通过BrowseLanding/BrowserPane共同道路，实际top/底部甲板依SessionBar/native-deck传ratio，不另造一套导航模型。

## B1–4 最终源码交接（2026-10-30）

Root 后续正式授权私有叶和前端；现在全部源码固定、tracked clean。基线 aaf91b82；B5 2ba5e983，Pages 0b828692，平台叶 09dc4d64，显式 WK controls 4eb36d47，前端17源 27220c91，实际导航错误回传 d5b3f5e1，甲板组合 df92d4e4e080b9999c518847ef4b6da8ba2fce03。共享8源仍由 history 独占，独立差异 e5fb1b9b；其父 S WIP 不要求集成。没有手造 DTO/generated 或自行改共享文件。

一个 Pages owner 记录 view/request/native navigation/revision；原生 callbacks 同步生成 immutable Changed，前端拒迟到快照。KVO 实际估计只在真实成功 didFinish 到 1，失败/取消独立；非 WK Wry 的 Finished 不提供成功事实，按 Root 决定仅 Settled + None、结束静态 pending。原 Wry IMP/scripts/auth/download 道路保留。公开 NSError 根+3 underlying、每串 UTF8 ≤4KiB；完整 retry target 私有且只匹配当前失败导航，不用裁界诊断作动作目标。Mac/iOS 显式 load/reload/back/forward 同步 prepare→SDK return→expect；nil history/SPA 不伪造新导航，stop 后 cancelled。Mac async navigate 通过既有 oneshot 返回真实 Result，不吞 load 失败。

手机 WK frame 裁到实际 deck.top，inset=0。随后独审抓到 native deck 仍按裁后 WK bottom−height 上移的问题；df92 已让 JS 传实际 DOM x/y/w/h，native create/move 共用 main-view origin+DOM rectangle。组合反例实际旧算式 red476 vs660，新3组加原6组共9纯检查绿。没有把 CGRect 算术冒成 UIKit/WK 真机。创建与 retained theme 更新用原 ground，不强盖站点 CSS。

分层证据 navigation-source：projection-settled-final 11、loading-settled-final 10、failure-final 10，前三源未受最后 geometry delta 改动；viewport-deck-final 9 对应 df92，共40受控检查（非同源全量 native）。旧 error、旧 JS frame、旧 native deck composition 业务红与 fixture 最初错误均保留。B5 row callback7 保留。最终 rustfmt --edition2024 check0/diffcheck0。Pages12个 Rust 测试仅源码，未编译；B5 actual Node3 是既有独立批次，不冒 B1–4 Rust 绿。最后 fulltypes 仅 pending-shared exit2（4DTO/pageState/2labels 衍生8错），等 actual producer 后一次收口，不伪造生成或反复缺类型检查。

最终 source-receipt.json 与 internal-mirror.json 存外盘 navigation-source，逐22源 SHA/git-show、commit链、所有小证据指纹；内盘 recovery 镜像保完整 patch/format-patch/逐文件全文/卡/合同与小证据。当前无 native/browser/App owned 进程或执行授权。尚未验：实际 WK 进度/原 NSError 拒绝根因/主题空白及显式白站点、手机 keyboard/safe-area/旋转触摸、真实资源回收。不能以40纯检查关闭这些条目。B源码到此停止扩展，交 history 完成共享组合；下一任务 N 先只读定位与单私有并发夹具计划。


### Physical iOS pipeline/signing read-only clarification (2026-10-30)

The project already has a real physical-iOS archive/sign/upload route; it is not limited to the old run11 simulator Lab. `docs/SHIP.md` and release `20261028-code-quality/preflight-ios/COMMANDS.sh.txt` use current verified dist, Tauri `ios build --ci --archive-only --export-method app-store-connect`, then system-PATH `xcodebuild -exportArchive`. That release actually uploaded0.1.14(60) with teamQR8DMW493W and recorded fresh physical arm64 library/App/codesign; this remains old320a source, not current R2 proof.

Current read-only `security find-identity` returned one valid AppleDevelopment identity. The local Xcode wildcard provisioning profile QR8DMW493W.* expires2027-10-02, contains the listed physical iPhone and its DeveloperCertificates digest matches that current valid identity. Thus existing development-signing foundations are present for a distinct Lab namespace such as `io.github.colehank.khor.quality.round2`; no new key export/signing/provisioning operation was performed, and a successful future build/install is not assumed. The last formal archive's actual TeamIdentifier remainsQR8DMW493W; the identity display-name suffixD87L86M53Y is not treated as a team identifier.

Still missing: a strictly current R2 `aarch64-apple-ios` library and independently signed physical Lab App with current179gen/278assets/native code/own bundle container; reachable/unlocked trusted device channel; fresh human touch/IME/keyboard/gesture scene. Current devicectl lists a paired developer-mode iPhone15ProMax/iOS27Beta, but tunnel unavailable/DDI servicesfalse, so it is not presently an installation-ready channel. No claim is made about why unavailable or SDK compatibility. The real existing quality simulator is available but Shutdown and contains first-round c371 native code; neither it, desktop Safari simulation nor visualViewport mocks substitutes for this current physical task.

Report `mobile/formal-ios-readonly-1/report.json` **31fafab54b6a7dc7e490a7b6b5a48f8bf0ed296fc55fdc61b2fbd1992a631bf7** pins actual file/previous-result/signing metadata and the minimal existing command shape, with full small evidence mirrored. No build/install/boot/restart/user-body access occurred. Physical installation is a later specifically authorized action; main and installed production app stay untouched.


## 2026-11-01 发布决定与历史记录边界

用户最新明确授权“先直接commit到main并ship”。本卡以上实施、失败及阶段权限记录原样保留；本次授权及发布状态以[本轮发布记录](../ROUND-2-RELEASE.md)为准。原 Mac 真人中文 IME 与真实 iPhone 操作仍未验，按用户决定移到发布后，不登记为通过；本卡记录提交不代表已合并 main、完成任一 SHIP 目标或已发布。
