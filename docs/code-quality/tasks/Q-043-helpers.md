# Q-043 helpers：剩余原生事件的资源归属

状态：review；两提交均已独立源审，作者固定源回归完成；尚未真实 native 验收或集成。

## 任务契约

- 目标：onPageWord / onFullscreen / onQuitRequested 与两个 Webview drag/drop 入口共用 Q043 owned adapter。关闭 owner 立即释放业务 callback；迟到 ACK/eval 不复活；四条拖放 listener 部分失败整体回滚；拒绝明确诊断。
- 用户与 root 授权：继续全仓治理、多智能体分工；root 在本轮批准生产 helper 适配，新增 window-drop.ts、ChatView.tsx 两接线和既有 check-terminal-drop.mjs 测试 seam 跟进。main 不改、不合并、不发布。
- 负责人 native_helpers；独立源审 root（已完成，最终测试待）。中央卡路径即本文件，STATE 只由 root 维护。
- 基线：d9ef30de33d5f6762236540d711a66268fd3f9c2。
- worktree `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-overwrite-recovery`；新分支 `fix/quality-native-helper-ownership`。先核旧树 tracked clean，原 `fix/quality-native-event-ownership` 保留 d4757136。
- 路径：apps/gui/src/api.ts、src/lib/native-events.ts、src/lib/window-drop.ts、src/views/ChatView.tsx；native-events fixture/专用 runner 与 check-terminal-drop.mjs 必要测试 seam；本任务卡。无布局/token/坐标处理、全局 package/CI、权限、Cargo/锁、上游补丁。
- 已读 khor-design / design-system README；仅订阅所有权变化。graph-first 已执行，图谱 build=9fd/head 不匹配，且 App/BrowseLanding/onWindowDrop 等漏召回；以下基于精确 d9 现场与锁定 SDK，不以图谱代替现场。

## 调查与原行为

- `onPageWord`→event.listen(Any)，payload.tab 过滤、onReady 在 native 接受且 owner 存活时触发；BrowserPane / BrowseLanding / PageControls / use-sheet-pan 调用。
- App→`onFullscreen`→Window.onResized(Window label)，首次与每次 resize 查询 isFullscreen；关闭后在途查询结果应丢弃，旧 owner 不再发新查询。
- App→`onQuitRequested`→Window.onCloseRequested(Window label)：真实 SDK 构造 CloseRequestedEvent，await handler，未 prevent 才 destroy。不能简单改 raw event 遗失默认关闭语义；旧 owner 在 await 后必须重新核活跃。
- TerminalPane→onWindowDrop / ChatView 自己的 effect→Webview.onDragDropEvent(Webview label)，SDK 串行建立 enter/over/drop/leave 四条。旧部分失败 catch 全吞且已入场 listener 不撤；关闭 wrapper 仍保留业务 callback。两处各自坐标/路径/上传业务原样保留。
- 旧 Node terminal-drop 门 mock 的是 SDK onDragDropEvent 成功组合 cleanup；没有真实四条入场、late eval 或第三条拒绝，因此漏本故障。旧 hear adapter 不覆盖这些入口。

## 结果与证据

证据根 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q043`。

- **真红** `helpers-red/report.json`：固定旧生产 d9，Chrome/WebKit 26 例，15 红 / 11 行为对照绿，source_stable=true。两端 page/fullscreen/quit/window-drop late 均 owner 关闭后 JS callback 仍存在；drag 第三 listen 拒绝留下 3 callback / 2 native listener；quit handler 中排队 microtask close 后 SDK 仍 destroy；destroy 拒绝产生 unhandled。Chrome import 前关闭仍入场，WebKit 同例因交付顺序绿，未把单引擎随机绿解释成已安全。
- 候选 `helpers-green-first/report.json`：30/34；剩余四个断言漏真实 PhysicalPosition 的 `type:'Physical'` 属性，实际 `instanceof` 已为真。仅改测试期待，保留报告，不当产品故障或最终绿。
- **候选固定** `helpers-fixed/report.json`：Chrome/WebKit 34/34，source_stable=true。精确 Any/Window/Webview target、page tab/onReady/ready 内关闭、fullscreen 首次/resize/关闭在途 query、quit prevent/正常 destroy/await 后 owner 关闭/拒绝、onWindowDrop partial rollback、四种真实 PhysicalPosition payload、drag 在途 admission 关闭及 unlisten 拒绝计数均通过。拒绝退订时 callback=0，native=4 且有诊断，没有声称释放。
- 最终 **`b8ffbc440f3426a5cb61e17b656637db4ef8f354`**：`helpers-final/report.json` Chrome/WebKit **38/38**，新增真实 SDK drag payload control 与 callback 抛错 peer 隔离两例均通过；SDK window/webview/dpi SHA contract 纳入固定源，sourceBefore/After 匹配。
- 同一 b8 固定源：`helpers-hear-final/report.json` 既有 hear **22/22**；`helpers-poll-final/report.json` 真实 ChatView 卸载在途/切 owner 双引擎 **4/4**；`helpers-final-checks/results.json` 完整 terminal-drop 原业务断言、types **exit0**，source_stable=true。unknown/empty/duplicate selector 三次 exit1，前后 fixture 数量不变且无输出目录。diffcheck 过。完整 cancel 套未重跑，root 依据仅换 drop 订阅批准最小相邻组。
- 实现：listenNativeEvent 接受显式 SDK EventTarget、onReady、event envelope 和活跃查询；ready 是无拒绝 boolean Promise，供组合逐个入场。关闭仍同步置 inactive/清业务函数/精确 unregister，独立 native unlisten。业务 handler 同步/异步失败由 owner 诊断，不截断其他 owner dispatch。onReady 抛错也回收已入场资源。
- quit 使用真实 SDK CloseRequestedEvent；保持 await listener 再按 preventDefault 条件 destroy，并加 owner 复查。fullscreen 保留初始查询和每 resize 查询，在导入后/发查询前/结果时核 owner。drag 四条独立 owned listener 串行入场，任一失败关闭整个集合，保留四种 payload 与 PhysicalPosition。
- 没有 Cargo/Xcode/下载依赖或权限扩大，没有启动/覆盖真实 App 或模拟器。HTTP 数据夹具不作 native 证明。

运行命令（worktree 根，已有 browser cache，不下载）：

```
PLAYWRIGHT_BROWSERS_PATH=/Volumes/disk0/Khor/caches/dependencies/khor-playwright NATIVE_HELPERS_CHECK_DIR=.../Q043/helpers-fixed node apps/gui/scripts/check-native-helpers.mjs
node apps/gui/node_modules/typescript/bin/tsc --noEmit -p apps/gui/tsconfig.json
```

## 限制与后续

- Rust 原生资源边界沿用 Q043：无 native id 的丢 ACK 不能猜测清理；unlisten 拒绝只释放 JS callback 并报告；迟到 registry 数值 metadata 不私自删除。不把 callback Map 清空声称为全部 Rust heap 清空。
- 真 App 等 root 分配 native/23840 槽；必须实际冷启动/事件、两端回收/关闭 owner 不复投/零 unhandled。OS drag/desktop window-close 物理交互尚未验。
- 集成需 root 非作者源审、固定头验证、分别声明实际平台边界；未完成不标全 Q043 done。

## 独立审查

root 已独立审冻结四生产文件差异，暂未发现阻断：同步 owner 失活和清 handler、onReady 关闭/异常两端回滚、真实 CloseRequestedEvent 的 await/prevent/owner-check destroy、四 drag 的 target/PhysicalPosition/失败回滚和原样坐标业务均核对。源审接收不替代固定源最终回归或真 App 证明。

固定生产 SHA256：

- api.ts `477e50591996a6795fc173b81369494c270382ab2e2eb2252c62ae1b51658c43`
- native-events.ts `51536082501313fa6b6cdd1de67ba3e1d4ddeb427358e77dfc599b5ad893182e`
- window-drop.ts `d8e803d126accd013f83dd930a0a107897715efb66dffedfa4b18c1abe9d5cd6`
- ChatView.tsx `c04f0ff0a8dd486d431b65a1c3f9ccedddaeac57a9c6db85a6df43a9974dcd69`

## 独立后续：共享 drop consumer 异常隔离（未混入冻结候选）

只读发现 `onWindowDrop` 的共享 handlers Set 直接循环调用：同一物理 drop 的首个 consumer 同步 throw，会中断后续 consumer。当前 owned adapter 能隔离独立 native listener 的异常，但共享 Set 是一个 listener 内的多 consumer，尚不隔离。root 已授权在第一批 helpers 冻结/提交后另做独立小提交：保留原 Set 迭代和业务 source/coords，逐 consumer try/catch，console 明确诊断但不含路径/payload；准备两个 consumer 的 scratch 真红→绿。已取得并保存真红/绿，且没有混入第一批 b8 提交：

- `helpers-fanout-red`：旧 b8 生产抛错逃出 shared native callback；保留初始红报告。
- `helpers-fanout-peer-red`：同一旧生产，首 consumer 抛错后下一 consumer **actual=[]**；test/source 固定，source_stable=true。这是实际运行红，不是源码断言。
- 候选 `helpers-fanout-green`：完整原 terminal-drop 业务断言与新增两个 consumer、固定一次诊断、诊断无 payload/throw path 文本均 **exit0**，source_stable=true。
- `helpers-fanout-native-contract/report.json`：同一候选真实 runtime 的 window-drop late/partial 双引擎 **4/4**，source_stable=true；types 无诊断、diffcheck 过。没有无故重跑与此改动无关的全38/22。
- root 独立审追加8行，核保留原 Set 迭代/同 drop 对象，只逐 consumer catch 且静态诊断无 payload/error 泄露，未发现阻断。
- 独立提交 **`adb868e016322d9b2541e1858801e47673855eba`**，仅 window-drop.ts 与 check-terminal-drop.mjs 两文件。业务变化为同步异常不再使后续活 consumer 丢事件，失败仍明确报告。

## 最新交接

- worktree `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-overwrite-recovery` / `fix/quality-native-helper-ownership` / **adb868e016322d9b2541e1858801e47673855eba**，tracked clean。首批 **b8ffbc44** 与 shared fanout followup **adb868e0** 分别可审、可集成。旧 d475 分支保留。
- 两提交 root 已各自完成非作者源审；最终源如上述固定38/22/poll4/terminal/types与followup4/terminal。真实 App 冷启动、OS drag/window-close 和原生两端资源仍待 root 原生槽，不声称浏览器等于 native；无权限/Cargo/Xcode/依赖下载/用户数据写入。
- 无本任务 server/browser/Cargo/native 进程，23843/23844 已退出；不占 23840 或模拟器。最终两盘约6.2/7.5GiB，高于5GiB阈值。
- root 正持协调/集成与 STATE，唯一 Cargo/native 槽由其统一分配。首批 source/tests 和 followup 不再修改，等待独立集成头回归和真正平台验收；全 Q043 尚不标 done。
- 本卡为作者中央记录，STATE 未修改。磁盘中断期间没有运行，root 清理独立旧 generator 缓存恢复6.7GiB后才按批恢复验证；未删原始日志或质量 App/原生缓存。
