# R2-M 自定义终端键行（R2-026 / R2-028）

Owner：/root/round2_history，作者树khor-round2-history，基线7246b665。Root已授权前端最小闭包；没有main/ship授权。用户R2-028已明确为Codex输入区缺组合键配置，不再调查Mac物理Shift拦截。

权威需求：[TERM-KEY-ROW](../TERM-KEY-ROW.md)。必须遵守：Ctrl/Alt/Shift；custom不叠加/不消耗既有CtrlAlt锁；每次点击一次、自定义长按不连发；组合键和纯按键序列；编辑草稿取消不应用；试按区零真实终端写入；按当前观看设备保存，旧预置显示/顺序保留；Mac设置入口可见，软键行本身的显示政策独立不强制改。

## 调查与依赖

先读设计skill/README、区域/浮层、TERMINAL与原M方案。code-review graph minimal/query先行，但TermKeys/useKeyRow/pressKey相关0命中，之后定点读源码。reviewer key-row-source-plan.json六路径SHA与本作者基线全部相同。CodeGraph目录不存在，按用户规则不索引/不运行CodeGraph CLI。

现term-keys.ts同时持RowSeat(name/on)、KEYS定义、device localStorage/session fallback/跨窗heardFrom；解析只允许预置名。term-kitty.ts保存实际协商flags/modifyOtherKeys。TermKeys::pressKey→Terminal.input→TerminalPane.onData→S lease-bound send；onData仍会对单字节再次应用当前锁，所以custom必须有per-Terminal try/finally preencoded作用域，不能只把明文序列交进去。该作用域不影响物理键、粘贴或S归属队列。

编辑入口复用KeyRowEditor/List；Mac AppearanceRows旧coarse gate只对该设置入口移除。KeyPanel blanket onMouseDown preventDefault用于保持终端焦点，但新增名称/输入编辑控件必须允许其自身输入焦点；不把“按live键帽保持软键盘”误用成“编辑器不能输入”。

## 已批准最小领域合同

- v1 {version,entries}；entry稳定id/on，builtin(name) / chord(label,stroke) / sequence(label,steps)。stroke只有受支持语义基础键+ctrl/alt/shift布尔，不保存函数/ESC/任意命令。
- 第一版基础键为标准ASCII字母数字/标点/空格与命名终端键（方向/Home/End/Pg/Insert/Delete/Esc/Tab/Enter/Backspace/F1–12）；用KeyA/Digit1等语义标识固定字母/符号映射，不改实体键盘布局。无Meta/global hotkey配置。
- 预算：128条、每序列1–16步、名称≤32 codepoints且≤128UTF8B、持久JSON≤64KiB、单次编码≤16KiB。未知版本/字段/键、重复id、无效序列整项拒绝；不部分发送。
- 旧RowSeat[]只做读入迁移为唯一v1模型，不保旧执行fallback。builtin id等于其原name，保原DOM身份/排序；缺失新预置追加关闭。custom id为custom:稳定随机ID。
- 配置在现owner严格校验后才保存/广播；存储失败维持本窗口fallback；坏持久配置提供安全默认＋编辑器明确信息，用户未显式重置/保存前不覆写原始值。
- 编码在激活时采实际终端协商状态，kitty优先、modifyOtherKeys/legacy沿现规则；UI不写ESC。序列预校验/全编码后一次Terminal.input；固定mods不读latches，private origin guard阻止onData二次改写/消耗。

## 精确写权与步骤

Root已授新lib/term-key-model.ts、lib/term-key-codec.ts，现term-keys.ts/term-kitty.ts，views/TermKeys.tsx、新KeyRowEntryEditor.tsx、more/AppearanceRows.tsx、TerminalPane.tsx仅origin guard，catalog zh/en必要词条、docs/TERMINAL.md，新check-key-row.mjs/check-key-row-ui.mjs与现check-keys.mjs必要适配；原断言保留。use-coarse.ts/other-windows.ts只读不改。

先pure model/codec/storage实际消费checkpoint，再UI/real xterm按压checkpoint。共享Core/API/Tauri/gen仍本owner集中接W稳定DTO；当前M不改native。任何catalog producer/native/browser都先领Root唯一槽，禁止隐式npm pretest/gen。

## 当前状态

Pure model/codec/storage已按下方两固定提交独审通过；当前实现UI第二片段，未生成新词条/未实际浏览器验收，不能称用户功能已完成。前置S 94131+7246已交付/52 native分批＋5浏览器证据见R2-S；M之后仍返回S journal/startup/L01/L02，不因调度变更移出本轮。

验收：模型预算/坏配置/迁移/同跨窗保存；实际xterm codec ShiftLeft、CtrlC、ShiftTab、CtrlShiftArrow、CtrlB→n；锁不叠加、试按0发送、custom长按不连发、原builtin锁/连发保持；真实表单CRUD/排序隐藏/取消与Mac设置入口；序列A→B归属不串。actual Codex选择行为必须独立设备/owned Lab验，raw字节不冒Codex UI已验。


## Pure checkpoint已固定；UI待下一片段

固定首`9f0349737afc137e4f3691cc47bbfe0348798cec`七源，再窄修`fb4db8bc871c6225b656ad7f1f92e9ac6c065f52`四源，当前tree clean。新model/codec/实际device存储owner已消费，TermKeys最小runtime resolver与id排序消费者同步；未新增CRUD表单/Settings入口，当前不能称用户已能配置。原cursor/tilde/SS3/typed低层移入共享codec供builtin/custom使用，ctrlOf下沉Kitty打断原循环依赖；原物理key/现builtin路径保持。custom全序列一次输入并带per-Terminal try/finally来源，现onData不叠/不消耗locks，不更改S send owner。

`pure-1`实际新11、旧keys33×9状态、types全部0/输入前后同。最初新suite sequence fixture残留chord stroke字段被严格schema拒，原工具结果来源+script快照保`model-first-failure`，仅修fixture未放宽生产。随后独审发现两原合同阻断，先在9f产品上实际独立红：badraw ordinary set未拒、custom合法ESC[I被真实focus classifier筛掉，各Node1/AssertionError，非编译或超时。修后统一focusReport=!preencoded&&isFocusReport复用3处；storage所有set在problem时拒，唯一显式resetKeyRow可替换；列表坏态toggle/move/grip禁用。`pure-review-green-1`13输入before/after同，新12/原33×9/types均实际0。独审精确4SHA=green/fixed，已解除阻断可集成完整pure。模拟storage事件不是实际多窗口。

证据`evidence/round2/mobile/key-row-check/`四目录已mirror；两commit小bundle已verify并双盘SHA同。尚未native/browser/App/实际Codex，没有手写gen。Root native-editor ce69构建与file-onlyE在另树，本作者不改其源或复制运行中producer。

下一UI：显式坏配置提示+仅重置按钮解锁；Mac Settings显示入口但不改软键行可见性；可视组合/sequence/name CRUD、预览/隔离试按0发送、草稿Cancel；runtime编码失败须明确反馈，不将不能表示的legacy组合降级成错误文字。自定义cap不repeat，既有cap repeat/锁保持。只读准备完成后Root分配继续；共享W DTO稳定接线由本owner短串行，不并写另一个Core/API/Tauri。


## UI 第二片段进行中

Root已续授UI闭包，纯9f+fb4已集成。7源候选全文/manifest在`mobile/key-row-check/ui-candidate-1`，包含独立KeyRowEntryEditor、TermKeys/Settings入口、zh/en各22key-row词条、TERMINAL说明和新真实组件浏览器脚本。无Terminal/会话prop进入试按：每次临时xterm仅encode后dispose，真实按键经原TermKeys与S发送owner。旧builtin重复/锁不改。初次tsc仅新catalog key缺失，raw日志ui-initial-types.log保留，不是绿；尚未实际producer/browser/App。

源码自查发现手机键盘生命周期必要修正：key-panel.ts::raise监听系统keyboardShown即release；在键盘替代面板内新增名称Input会因系统键盘升起卸载表单。已报Root，拟仅TermKeys将“编辑这一行”交现KeyRowEditor TaskOverlay，释放原live-key面板占位；Settings同编辑器、单scrim，不向viewport/key-panel另加抑制状态。Chrome可验证输入focus但不能冒真手机键盘已验。候选1是待修/待审历史，后续固定新manifest。

R2-025长按移动候选参考验收属于另闭环；R2-026配置和R2-028缺组合机制由本M共同实现，实际Codex文本选择/触屏软键盘仍单列，byte输出不冒设备语义。


### UI来源/生成进行检查点

Root/独审确认手机键盘风险后，现入口先释放普通key panel，再打开唯一KeyRowEditor TaskOverlay，删被替代in-place分支与!up强制关闭effect；不改viewport/key-panel的owner。候选3七源manifest `99fe6dbc…781c`，独审源码/strict runner准入无阻断。

`ui-catalog-1`已实际终态0/8.422s：V严格metadata→prepare/actual catalog bin＋gate compile-only→verify/require_provenance→选中真实exe输出，再stage/atomic publish。2156source前后同、env/tools稳、owners[]、TMPabsent；真实catalog staged=published `4cbc1e9ff6ac35faa2246bc764cb2d49d6949b66e17450f4d5beec6ff14c92a1`，binary `59adb6aa17e005b2c483ae6abd75babb6b95b8bb4c9233e11a8226678c626e24`；原普通runner未运行，留superseded文件。native槽已交回，无App。

随后`ui-js-1`新12/旧keys33×9/words/types全0，16相关输入前后同。已生成catalog词条，不再依赖缺字段；原ui-initial-types.log红保留。浏览器尚未运行，拟9业务组（Mac入口/CRUD/实际双窗/actualxterm sequence与锁/hold/unsupported/坏态reset/320窄宽与实际viewport owner通知）；浏览器候选额外仅script catch截图，prod6与candidate3同，脚本 `078ec80a…8d9e`，不冒它在旧native fullframe中。UI候选/JS证据已双盘SHA核对。


## UI checkpoint已固定 2423f2b2；设备验收仍待

`2423f2b2f94fea7dcc73c23e122d5775509c6345`七路径已commit，tree clean，精确git-show SHA=ui-fixed-2423f2b2/source-manifest。strict catalog/source与新12/旧33×9/words/types证据如上，consumer6SHA不变；唯一后加测试catch截图源码差异在validation-map明确保留，原2156source不回填。

`browser-1`真实Chrome9组全0/10.678s，2156tracked/untracked source/167actualgen tree前后同；关ownChrome/server/fixture后ownedremaining[]、23840free、fixture[]。覆盖Mac设置入口、自动命名/隔离试按、保存/取消/Unicode/sequence排序删步、隐藏/重排/重开、同origin实际双window同步、实际xterm一次whole sequence与Kitty协商、锁/焦点/custom长按0重复/原Tab连发、unsupported序列零prefix且明示、坏raw保留至显式reset，以及320CSS窄宽与原panel→TaskOverlay交接。真实viewport owner经受控vv resize和settled keyboardShown发通知后草稿保留/Save恰一条；试按时live xterm仍挂，input0。此不是物理系统软键盘或真实Codex选择行为验收。

browser原log保既有Vite宽入口扫描`lab/code.tsx`的editorIn缺export（本轮E已移除）错误；Vite按需fallback后本9组/pageerrors[]实际通过，不称所有旧Labs构建绿，不改无关Lab。本条只记下轮债。

source/validation/native strict/browser evidence已双盘SHA核；bundle m-ui-2423f2b2.bundle 11448B SHA3115e440eddd42375e5731fa0505a3f41f787bc1b5bc76c6d4d8437f12dfcbb9已verify。没有main/ship/App/真agent动作。最后独审精准七SHA/producer-JS-browser映射已尾核无阻断，Root可集成；下一按Root调度接W稳定公共DTO或回S未闭项，不把M设备pending/完整S移出本轮。


## M026 实际 Codex 输入链路：限定事实已核（2026-10-30）

Root以集成 `5c726b0a` 正式 `TerminalPane`＋`TermKeys`＋attachment facade，经透明白名单HTTP转发到独立真实Khor Core/PTY，再到私有Codex0.165实例。实际CLI为5ddc producer的私有clone，SHA `b83f147e…8584`；Codex私有clone SHA `a4509915…a85a`。八输入GUI/API源与Root5c逐SHA一致，Rust差异仅B5 tabs.rs，与终端无关。没有新编译、没有使用用户HOME/登录/会话，custom provider只指向本地拒绝器；完整范围与源码见 `mobile/codex-input-real-pty-1/preparation.json`。

实际四批原结果均保留：45685/2.363s在合法`tui/16hex`被fixture误拒时终止，浏览器未启动；79009/43.046s已呈现真实Codex，fixture误等不存在的context标签、无键入；34725/19.856s标准ShiftLeft已实际发送，但fixture错误假定该Codex必须移动/扩展选区；71277/4.859s已完成两个独立草稿和focus断言，比较逻辑只剔标准侧keyup，因实际键帽也发同keyup而失败。它们不是一次green suite，不回填原false。每批source/gen/fixture稳定、真实private open/close0、模型请求0、owned/private footprint空、四端口free、TMPabsent。

`mobile/codex-input-real-pty-4/browser-failure.json`保留实际两条完整wire：去各自最后普通字符X/Y后，均精确为 `ESC[1;2D ESC[1;2:3D`，包含相同press与keyup，无须任何规范化。当前独立草稿分别为“› abcdeX”和“› pqrstY”，cursorX均8；键帽点击前后focus断言实际已执行通过。记录中没有CR/LF输入、page error、意外API或外部网络请求。故这次受验Codex对ShiftLeft未产生选区/光标移动，两个输入路径结果相同；不能把Khor发送能力称为第三方选择语义支持。

Root补只读 `analyze.py`，对7个不可变原件SHA和上述实际值执行离线断言1项通过；独审又逐原SHA、实际raw/state/cleanup尾核通过，不再第五次重开运行。`derived-analysis.json`明确原outer和acceptance_passed仍false；成功截图及后续键盘CtrlU/CtrlC没有执行，真实CLIclose0已执行并严格清理。37小件含原失败/截图/源码/收据已逐SHA镜像到内盘 `recovery/m026-codex-pty-4/manifest.json`，不复制私有HOME、二进制或依赖链接。

此条关闭的是“正式Khor终端链到真实Codex的配置键帽/标准键盘原始字节及本次草稿等价”证据缺口；不是完整green测试套件，也不是人手实体键盘、系统软键盘、真手机或WK验证。手机软键盘/最终组合仍按本卡保留，E真实Mac中文IME另验。
