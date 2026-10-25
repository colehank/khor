# Q-010 / Q-011：真实原生 App 验收

## 验收完成与候选提交（2026-10-25，优先于下方历史记录）

- 提交 `093d411539e9f2c69ccca1a592eed6ff1258d5a1` 已在wave1完成，只有hook与对应scroll回归脚本两文件，tracked clean；没有main合并或发布。
- 最终 Chrome/WebKit **34/34**通过，源码运行中无变化；证据 `browser-candidate-final-02/results.json`。hook SHA与原生run-09/10完全一致，脚本最终SHA `0811cc3e84316b058f84146eed3cb06c51028b30d1b00809439b9fdae0d00288`。
- 最终WebKit raw观测仍gap40；明确保存这个未稳定瞬时读数，3帧后原350ms严格tail采样全部0。不宣称瞬时gap消失，不放宽2px阈值；两次33/34历史失败与旧hook对照均保留。
- 真正原生系统键盘：run-08基线红 → run-09相同driver候选绿 → run-10真实用户下滑hide和reader历史拖动绿。Q011 commit真正原生run-07绿。物理iPhone/中文IME/硬件Escape/辅助技术未验，通知ACL另归Q015。
- `tsc --noEmit`、`git diff --check`通过。证据总索引 `ACCEPTANCE.md`，固定源码/提交映射 `candidate-final-source.json`。
- 完整Cargo槽早已释放；原生/浏览器测试均结束。自有localhost23840/23844 fixture已核对PID后停止；专用模拟器已Shutdown。源码冻结窗口已交还root，后续集成不需等本任务。

## 候选过程（历史，2026-10-25 20:18）

仅 root 授权的 `use-chat-scroll.ts` 与对应浏览器回归在 wave1 有未提交修改，HEAD仍 c3fe828b。唯一生产变化是 composer 退焦到 BODY 后保留布局归属；真实 reader 输入仍先接管，聚焦其他控件后恢复普通 scroll 判定。没有改阈值、键盘布局或权限。候选 hook SHA256 `3504f9e5e09dce1b80f3c54b4a4384b9dfcc96078abf1b258cb5e37ae659b558`。

- **run-09 原生绿**：复用 run-08 同一实际 Tauri App / Rust 库 / 完全相同 Swift driver，加载候选前端。XCTest exit0。显式 DOM blur 引发真实 UIKit 关闭，6次 settled tail gap全0；固定红基线 run-08同脚本全164。证据 `run-09/long-draft-green.json`，原红证据不覆盖。
- **run-10 用户真实隐藏键盘绿**：XCUI原生在textarea快速向下拖动，trace记录 trusted pointerdown/touchstart/touchmove→生产field-swipe的blur/focusout→UIKit消失，driver不调用blur helper。XCTest exit0，6次 settled gap全0。随后真实消息scroller下拖读历史，gap460，证明reader touch优先。证据 `run-10/user-hide-result.json`，全部requested样本、原始events和xcresult保留。
- 新增浏览器4场景 Chrome/WebKit×ACP/Khor均通过：长草稿自然blur关闭、显式late旧top恢复、trusted wheel读取历史、其他控件聚焦后普通scroll均有断言。`browser-blur-red`原4/4失败，`browser-blur-green`4/4通过。
- 完整34场景两轮均33/34，唯一WebKit `lab-acp-blur`填短草稿后2–6ms首样本gap40、下一帧15ms起全0。候选单跑该Lab通过；独立Vite仅覆盖c3旧hook跑相同17场景顺序，Lab也通过、新blur回归如预期失败。未证明瞬时差异归因候选，原证据不覆盖。root批准Lab在fill后保存未等待rAF的原始布局，再await既有frames helper，然后开始原350ms严格settled采样；不改2px阈值/原生断言。最终34场景重新运行中。首个同步尝试在review时发现raw误用了等待rAF的samples helper，已只终止自己的整组进程并保留aborted.json，再纠正为直接evaluate观测；未混作完成结果。
- `npm run test:types` / `tsc --noEmit`通过。不需要Rust重编，完整Cargo槽归root/Q032；仅Swift UITest增量编译。
- Q011原生commit验收run-07仍通过（第6/8项实际菜单滚动后真实点击，位置/焦点稳定）。通知ACL拒绝另归Q015，未改权限。物理iPhone/中文IME/硬件Escape/辅助技术仍未验。
- 资源现场：专用fixture server run-10（localhost23840），专用模拟器FE04AC7C-B375-4F2E-AE7E-B6F8F5608947；两个原生测试均终止。浏览器诊断Vite localhost23844仅覆盖hook为c3旧源码，不改候选文件。


## 原生验收当前结论（run-08，最新）

- Q011 的真正原生模拟器流程已通过：run-07实际菜单上滑命中第6/8项，位置/焦点持续稳定，XCTest exit0（1 test）。
- Q010 的空稿开合、长稿打开和历史锚点已有真实原生执行通过证据，但长稿关闭存在已确认原生红，不能关闭该任务。
- run-08最小复现无前置开合：0字→真实输入288字，打开时gap0；明确blur关闭后6次稳定采样gap均164，XCTest exit65。`long-draft-red.json` 保存完整结构化报告。
- 256个原生rAF帧：blur后+82ms visualViewport538→874、clientHeight218→514，滚动owner先正确调整top5014/gap0；+266ms尺寸与内容不变、activeElement=BODY、没有任何pointer/touch输入时，WebKit把top改回4850（恰为打开键盘前旧top），gap164持续。观测期间没有focus/scroll纠正；只是显式blur触发真实UIKit键盘关闭。
- 实际事件顺序为blur/focusout→visual-resize→textarea/chat scroll；晚到旧top恢复不能当作用户阅读历史。生产仍冻结c3fe828b，等待root在非冻结候选中实际修复并做同脚本native红绿。
- 当前不启动新编译；完整Cargo槽归root。专用fixture server当前run-08/session93658；run-08测试session76747已经完成，待收尾确认；模拟器是本任务专用FE04AC7C-B375-4F2E-AE7E-B6F8F5608947。root可据后续分配决定继续原生候选验收或停止这些自有资源。


## 已取得的真正原生证据与当前 run-08（最新）

冻结生产仍为 c3fe828b。完整 Cargo槽已释放给root；当前只有独立 Swift UITest driver 与专用模拟器，不覆盖用户 App/store。

- run-05 实际 Tauri App（native=true，UIKit系统键盘）完成 ACP/Khor 各3轮空草稿开合，gap=0；真实输入288字符后键盘打开仍tail；真实拖动历史后，键盘开合保留首可见段落（ACP48、Khor50）。这些断言均执行通过，但整个单case随后在commit操作失败，不能宣称整个run绿。
- run-05 ACP长草稿关闭后、历史拖动前观测gap164；Khor相同步骤0。该点原driver未assert，明确未验收，不以其余键盘结果掩盖。
- run-06 commit定向只读trace确认之前坐标tap命中HTML/底下CodeMirror，并没有menu.itemSelect。实际菜单可滚区域上限240px，row6中心已在裁剪区域外，是driver的可见性前提错误，不是生产回归。
- run-07 driver改为先真正在菜单内部上滑，直到elementFromPoint确认目标item可见，再真实tap，保留全部位置/焦点断言。XCTest exit0，1 test/0 failures；第6项连续4次top3348/focus src/module-6.ts，第8项top4511/focus src/module-8.ts。真正menu.itemSelect/帧trace/截图保留；物理iPhone、硬件Escape及辅助技术仍未验。
- run-08 当前只做最小ACP长草稿关闭：新文档空稿、原生点输入框/键入288字符、开键盘tail、明确blur收键盘，然后6次稳定采样全部收集后严格assert tail。4.5秒只读帧与事件环记录 scroller clientHeight/scrollHeight/top、visualViewport height/offset、activeElement/composer bounds、触摸/输入/focus/blur/scroll时序，不调用scroll/focus纠正。PID27235，session76747；结果待收。

## 实际通知拒绝发现

run-06 原生实测 windowLabel/webviewLabel都是main；bootstrap后约27ms出现字符串rejection `notification.is_permission_granted not allowed`，reasonType=string，实际没有Error.stack（stack=null），未伪造调用堆栈。

冻结Cargo.lock为tauri-plugin-notification2.4.0。其lib.rs:233注入init-iife.js，脚本启动即invoke(permission-read).then，未catch；项目前端没有显式isPermissionGranted调用。当前default.json只授予core事件/window，scan.json只有barcode权限。完整依赖初始化脚本、lib.rs、两份冻结capability及原始事件都复制到run-06/notification。没有擅自增加notification:default或改冻结生产。由root后续区分最小授权与插件错误处理。


## run-03 / run-04 启动诊断（最新）

run-02 实际 App 已安装启动，但首轮 XCTest exit65：仅初始底色、未出现 composer，因此不是产品滚动失败证据。实际 Tauri2.11.5 mobile dev proxy 会把 devUrl 与资源路径直接拼接，带 query 的 devUrl 导致 main.tsx 请求返回 HTML；`run-02/proxy-url-check.json` 已验证 malformed URL=text/html、正确资源=text/javascript。

run-03 只修 harness：纯 origin devUrl；fixture early history.replaceState 提供已有 bridge/session query；localhost CORS/测试 WebContent HTTP；增加早期 native/location/JS错误与请求记录。没有改生产或模拟 viewport。真正 Rust lib 再编译 exit0约21秒。

run-03 原生 App 仍在请求 Vite 前停住。对实际 App PID93971 的2秒 sample直接显示 main thread 在 XCTest idle notifier→os_log→mirror_to_stderr→writev，Tauri Swift StdoutRedirector 回读stderr又 Logger.info→os_log→writev；本地 Tauri Logger.swift 的管道重定向及 OSLOG- 前缀过滤相符。`startup.sample.txt` 保留。已仅终止自己的测试 App，XCTest exit65，旧报告保留。

root同意测试环境修正。run-04 复用完全相同 run-03 工程/App/staticlib，不再做 Cargo或App重链接，仅在 XCTest driver 的 App launchEnvironment 添加 `OS_ACTIVITY_MODE=disable` 和 `OS_ACTIVITY_DT_MODE=NO`，避免测试 OS 日志镜像递归。观测JS仍保存 native/href/错误，不改 production/viewport/键盘断言。

当前 run-04 XCTest PID/PGID97702，session10952；专用模拟器仍 FE04AC7C-B375-4F2E-AE7E-B6F8F5608947。观察server/日志在 run-03，run-04/harness-source.json 明确记录引用关系、driver变更与哈希。唯一完整 Cargo槽已释放给root；这轮只增量编译 Swift UITest driver 并运行专用模拟器，尚无原生通过结论。


## run-02 恢复执行（最新）

用户再次恢复目标，root 重新分配任务与唯一 native/Cargo 槽。旧 PID/PGID 29446 和 fixture38906 已核对终止。内盘辅助工作树已迁移到 `/Users/zgh/Developer/Khor/worktrees`；本任务外盘路径保持不变。

root 冻结 wave1 HEAD `c3fe828b0ae0a2b35b8fc65ed43f2982e45acb6f`，tracked clean。新证据放原证据目录 `run-02/`，旧暂停日志保留。重新记录生产源码、Cargo.lock、当前 catalog 与 fixture 指纹。

- 真正 iOS-sim Rust lib 已完成：Cargo exit0，约45秒。新 lib 约1.005GB，sha256 `26b79654fb6d79e1270d4e7cc79dcc5cad3366101e275e769d38a2a7c3a8dd65`，同盘 APFS clone 到派生 Xcode 工程。没有使用主仓发布库。
- 派生 Xcode App 已链接成功：xcodebuild build exit0，约6秒。保留生产 Tauri/Rust入口、main.mm、framework与资源配置；独立 bundle id `io.github.colehank.khor.quality`，只省略已显式完成的 Rust prebuild、指定实际只读 source navigator 路径、允许 fixture localhost，以及增加 XCTest UI driver。
- 当前 XCTest UI run：PID/PGID86430，session77782。专用模拟器 `FE04AC7C-B375-4F2E-AE7E-B6F8F5608947`，不覆盖用户模拟器 App 或 store。fixture 仅 localhost23840。
- UI driver 会点击实际 App textarea 唤起真实 UIKit 键盘、原生输入长草稿与拖动历史、点 commit 菜单第6/8项；fixture仅提供虚构数据与只读尺寸采样。关闭键盘的 blur 和场景初始 scroll 是显式 harness 操作，不冒充用户触摸。物理iPhone/中文IME/硬件Escape与辅助技术仍另记。
- 本节记录构建与启动事实，尚未有原生运行通过结论。最终以 xcode-test-result.json、test-results.xcresult、metrics.jsonl 和冻结源码指纹为准。


状态：working（用户恢复目标；冻结 c3fe828b 的隔离原生 App 验收进行中）。
所有者：/root/native_acceptance；恢复时 root 重新分配。
基线：`9b175b42be7e4bce3d42f08ac6778f80a62f222b`。
源码只读：`/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`。
证据目录：`/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q010-native-20261025`。

## 本轮核对

- STATE/PLAN 与 Q010/Q011/Q025 已读。当前 main 不改，不合并、不发布，不覆盖用户 App、store 或会话。
- Xcode 可用；iOS26.5 模拟器可用；`aarch64-apple-ios-sim` target 已安装。可用模拟器检查时均 Shutdown。
- 已有 native-menu/editor-history harness 是实际 WK + 生产适配器 probe，但其文档明确不是完整 Tauri App，不能代替此次验收。
- 生产 `api.ts` 已有 `?bridge=<port>` 优先接口，可用它把实际 Tauri App 的数据访问送入隔离 fixture，保留生产 App、viewport、scroll owner 和 CommitPage。
- Tauri2.11.5 的 WebviewUrl::App 默认 index.html 保留 devUrl 的 query；本地 tauri-build2.6.3 读取并合并 TAURI_CONFIG，支持隔离 identifier/devUrl，无需改生产源码。

## 已准备的隔离环境

- `tauri-config.json`：identifier `io.github.colehank.khor.quality`，devUrl 指向 127.0.0.1:23840，bridge 指向同一 fixture。没有使用主仓既有静态库/发布 App 冒充新版本。
- `source.json` 固定 HEAD 与 Cargo.lock、生产滚动/viewport/CommitPage/原生入口及 fixture 的 SHA-256。
- `server.mjs` 只在 127.0.0.1:23840 开启，复用项目现有 quality-preview；已通过 HTTP 注入 60 段虚构 ACP 长回复。
- `metrics.js` 是 fixture-only 观测与操作入口，记录原生标记、visualViewport、scroller、composer 和目标文件位置。其程序化 scroll 只能用于场景准备，不能当成真实触摸/键盘证据。
- 尚未生成或安装任何隔离 App，未启动任何模拟器；没有新的原生运行验收结果。

## 本次构建与停止

root 授予唯一 Cargo/native 槽后，执行：

```sh
CARGO_TARGET_DIR=/Volumes/disk0/Khor/caches/rust/khor-quality-target \
CARGO_BUILD_JOBS=2 \
TAURI_CONFIG="$(cat <证据目录>/tauri-config.json)" \
cargo build --locked -p khor-gui --no-default-features --features devtools \
  --target aarch64-apple-ios-sim --lib
```

实际调用由 `build.py` 用结构化 env 实现；上例仅描述参数，不含凭据。PID/PGID 29446；监测器针对掉盘、<5GiB 可用空间或重复 I/O 错误仅中止本组。约210秒时收到 root 转达的用户暂停要求，向自己的 PGID 发送 SIGTERM，构建退出 -15（launcher143），未完成，不能计绿。ps 确认 PGID 无剩余进程。日志保留 `cargo.log`、`build-process.json`、`build-result.json`、`pause-processes.json`。

fixture PID38906 已 SIGTERM，session退出143。构建槽已归还 root。没有删除缓存，也没有补跑构建。

## Q025 只读复核

原 Q025 工作树在 `/Volumes/disk0/Khor/worktrees/current/khor-quality-native-io`；仍有未提交 queue/connection、chat/term 与 native_io dispatcher 代码，任务卡原先“尚未实现”已落后于源码。未修改或覆盖它。

轻量执行 `node scripts/check-native-io.mjs`：7/7 std-only 用例通过，包含 socket shutdown、保序、关闭/重开隔离、turn片间控制、回包唤醒与队列背压。fixture panic 是被捕获的预期分支，其用例通过。没有执行真实 core/Tauri Cargo tests，dispatcher ACL/参数适配和真实 native 编译仍待验证。

## 恢复后的下一步

1. root 明确重新分配唯一构建槽；核对现场 HEAD/diff、source.json 和进程，不据旧句柄重启。
2. 续建本次真正生产 Tauri iOS lib；记录新产物哈希，不拿主仓旧 Externals 复用作证明。
3. 在证据目录复制生成 Xcode 工程，使用此新静态库和独立 bundle id；派生工程省略已由 Cargo 显式执行过的 Rust prebuild，完整保留生产 main/资源/framework 配置。安装至独立测试 App/专用模拟器，不覆盖用户 App。
4. 使用真实系统触摸唤起键盘，分别验 ACP/Khor 尾随、历史阅读、三次开合、多行草稿；记录 native 标记与真实 viewport/截图。程序化 visualViewport 或 Safari 不计通过。
5. 在真实 App 内加载生产 CommitPage fixture，真实点击菜单第6/8文件，等动画/懒加载稳定后检查位置与焦点；取消/Escape 与连续选择另验。
6. 模拟器 App、物理 iPhone 与辅助技术分开记。物理设备尚未验收，不冒充完成。Q025 在独立槽做 core/native invoke tests 后再集成，不把它混进首批验收源码。
