# Q-010 / Q-011 独立审查

状态：第四版复审完成 / 无已证实阻断；正式基线红绿与集成验收由root登记
审查者：/root/gui_audit（不是生产 fix 或回归脚本作者）
权威目录：/Volumes/disk0/khor-code-quality-plan/docs/code-quality
只读工作树：/Volumes/disk0/khor-code-quality-wave1
基线：8704a503a27b36ad58f922690de50e766f8bd485
范围：use-chat-scroll.ts、CommitPage.tsx、check-scroll-lifecycle.mjs、check-ai-elements selector 修改；定向核对调用方生命周期。

## 已执行

- 读 STATE/PLAN/任务卡/AGENTS；使用 review-changes 技能。get_minimal_context、detect_changes、get_review_context、affected_flows、tests_for、impact 图工具返回空图/未找到，故定向读源代码与 diff；不能把图返回零当作零风险。
- 比较真实输入事件、ResizeObserver、RAF 与 onScroll 的状态所有权；核对菜单关闭焦点处理与最终采样是否会自行修正状态。
- 经 root 分配独立端口 23826，运行生产 ChatView + 现有 fixture + Chrome headless 真实 wheel 探针。Vite cache 独立 `/tmp/khor-q010-review-vite`；执行后 browser/server 已正常关闭，没有重置 23820、没有改生产源码。

## R1 · P2 · 真实手势与 viewport layout 同帧仍被撤销（已运行确认）

证据版本：`use-chat-scroll.ts` SHA-256 `04456fe4f889f583318dcdfabdfcebbedfc9ccac77e081211784920a6cdd4a2c`，探针前后相同。

- 位置：hook 21–24 的 releaseLayout 只取消已有 RAF；64–70 的 onScroll 遇尺寸变化仍无条件 sync；36–40 沿旧 following/anchor 写回。
- 触发：读者真实 wheel 与新 viewport 高度变更在同一事件 turn。探针在一次 passive wheel listener 中仅设置 scroller 的 `flex:none` 和 `height=原clientHeight−80px`，模拟布局并发；**没有赋值 scrollTop，也没有调用生产 hook 方法**。wheel 由 Playwright mouse.wheel(0,-400) 发出，浏览器 `isTrusted:true`。
- 控制：top3421、height4084、view663 → wheel top3021；最终 gap400，away=true。
- 尾部并发：top3421 → wheel已到3021 → view583 → scroll回包/最终 top3501，gap0，away=false。用户离开尾部的手势失效。
- 历史并发：先真实 wheel(-800) 稳定到2621；第二次 wheel(-400) 已到2221 → view583 → 最终回2621。用户的新历史位置被旧 anchor 撤销。
- 影响：键盘动画或流式/布局重排和读者手势交错时会拉回尾部或旧阅读点。仅取消“当前已排队”的 RAF 不足以表达用户接管；之后新产生的 layout 分支仍覆盖输入。
- 建议：输入应参与滚动所有权/位置快照判断；保证同 turn 的新尺寸不会掩盖已发生的真实位移，同时保留迟到 caret/layout scroll 纠正。具体实现由 root 选择，不以任意时间窗吞事件。
- 验证缺口：现脚本 history 先 wheel、等350ms稳定采样，再模拟键盘；没有真实输入和 layout 同帧用例。已通知 root 与 validation_audit 补回归。

最小探针核心（生产页面已有足够长的消息，停在尾部或已稳定的历史位置）：

```js
await page.locator('[data-chat-scroll]').evaluate(box => {
  box.addEventListener('wheel', event => {
    console.log({ trusted: event.isTrusted, top: box.scrollTop });
    const h = box.clientHeight;
    box.style.flex = 'none';
    box.style.height = `${h - 80}px`;
  }, { passive: true, once: true });
});
await page.mouse.wheel(0, -400);
// 500ms 后只读 top/gap；并发组错误回到旧 following/anchor。
```

## R2 · 测试缺口 · commit 未强制验证焦点在选中文件

`check-scroll-lifecycle.mjs` 97–98 已记录 focusInTarget，但原160–162只断言非trigger且focus稳定。稳定在body也通过，不能证明键盘位置已交给目标section。生产fix 89/196确实赋焦点且使section可聚焦；这是回归强度缺口。已发给 validation_audit；作者表示当前 WebKit 完成后补每帧 focusInTarget 断言，避免执行中改指纹。

复核更新：作者已补 `evidence.samples.every(s => s.focusInTarget)`，R2源码缺口已关闭，完整新版本运行结果由validation/root登记。

## 暂定结论与继续项

- samples() 的目标动作后采样只读几何/焦点，没有焦点或scrollTop自修正。初始 toBottom 点击发生在被测键盘动作前，属于初始条件，不会掩盖动作后的失败。
- 继续核对会话切换/隐藏、effect/RAF清理、Escape与重复菜单交接；R1未解决前不建议将Q010标为完成。

## 后续核对记录

- 生命周期静态检查：DetailPane 525/540–542/571 与 App 2990 的 ChatView/MachineChat 使用会话 ID key；新会话重建 hook，不会继承旧 following/anchor。hook 45–58 cleanup 配对 disconnect RO、cancel RAF、移除4类 listener。`clientHeight===0` 保持原阅读快照，重新可见由观察器恢复；本次没有发现新增跨会话/隐藏资源泄漏。真实后台恢复仍未执行，不能称原生验收。
- 菜单：selectedFile 只在 item select 写入，close auto focus 读取后立即清空；Escape无selection时保留Radix默认trigger焦点。section 的 `tabIndex=-1`、`aria-label=path` 与 preventScroll focus 对键盘位置交接是合理的；没有新增Tab序列中的八个停靠点。
- 现有实际证据：`scroll-script-chrome-commit/results.json` 的mouse/touch与`scroll-script-webkit/results.json`的mouse/touch每帧focus均为src/module-6.ts，focusInTarget均true；Chrome Escape持续trigger。旧键盘用例分别因Home/方向键驱动时序、WebKit Tab配置未到达目标而失败，不能计为生产键盘失败或验收通过，作者正在修测试驱动。
- root补充并已核对现有JSON：WebKit khor-tail在约682ms后无尺寸变化突然gap100（焦点TEXTAREA），证明“一帧修正”还不覆盖迟到caret/layout scroll；Chrome四chat场景通过不能替代该边界。生产文件指纹与R1相同。
- 已向root建议：真实滚动意图/已发生位移需先于RO纠正被接纳；明确外部composer焦点下的迟到caret复位可保留旧tail/anchor，而非吞掉所有无input scroll。若引入持续到scrollend的手势所有权，需同时处理无位移wheel/keydown、嵌套滚动、scrollend不可用的退场，不能永久挂起自动跟随。
- check-ai-elements selector改动只将已失效的deck路径替换为session-bar内具名Back按钮，语义范围明确，没有放宽原断言；现阶段无独立发现。
- 新增 `inputDuringResize` 正式用例已静态复核：真实trusted wheel内只注入几何、200–650ms持续只读检查最终位置，没有自修正；tail与history分别独立初始状态。尚未将作者后续生产修复计为通过。
- 非阻断测试建议：keyboard()记录每步等2RAF的动画gap，但tail场景原只断言动画结束后稳定samples，可能漏掉动画内脱尾后末尾恢复；已建议作者对已有12步记录补tail断言或明确仅最终稳定状态的验证边界。

## 本次审查结论

Q011生产焦点交接与资源边界未发现阻断问题；键盘/Escape新驱动仍需真实执行结果。Q010有已运行确认R1，并有同指纹WebKit晚到caret scroll失败证据，**请求修正后复审**。测试动作后的观察未发现写回scroll/focus掩盖。此结论仅对应上述记录指纹；后续root变更需重新审查，不应沿用本结论宣称通过。

## 第三版复审（静态，2026-10-25）

root要求此时不启动浏览器，以免干扰即将进行的正式baseline红；本轮仅只读源代码/调用方。`use-chat-scroll.ts` SHA-256：`07a0f84baaa6cc076e182f4303400f55445403473c8fac571bb9fb872d3dea1a`。

- R1控制流修正已核对：`sync` 与 `onScroll` 在尺寸分支前调用 `acceptReaderScroll`，真实输入后实际top变化优先更新following/anchor。`readerUntil`是惰性200ms deadline而非setTimeout；无位移输入不延长归因，惯性位移更新deadline。没有“等不到scrollend就永久持有”的新资源风险。
- 可读性：把读取用户位置与布局写回分成readPosition/sync，减少重复状态更新；原有near40语义保持。Disclosure/toBottom清deadline与RAF后显式确定owner；cleanup仍disconnect、cancel、remove成对。keyup无需对应资源，deadline自然过期。
- 归因限制需要诚实保留：200ms内无法仅凭top变化证明变化来自手势，因此这是有界启发式，不等于完整输入因果证明。无位移wheel紧邻浏览器caret移动、按住原生scrollbar超过200ms后才拖动等边界尚未独立运行，未作为已证缺陷。
- 新具体待核实边界：`Markdown.tsx:369`目录/脚注按钮使用scrollIntoView。该button的pointerdown被readerInput排除，captureDisclosure也不识别它；如果WebKit点击按钮后activeElement仍为composer，onScroll的editingComposer会把明确导航写回旧tail/anchor。已发root待正式baseline红结束后决定运行一个WebKit探针；现在没有实证，不列阻断发现。
- 低风险建议：96行`focus.closest('[data-chat-box]')`覆盖整个胶囊（包含Plus/发送/模式按钮），并非只编辑文本；建议把caret例外收窄到`[data-composer-text]`或实际editable焦点，避免将胶囊按钮焦点也解释为caret恢复。
- root报告Chrome/WebKit第三版16项chat正在完成；这些是作者运行证据，审查者尚未核对最终JSON。没有以旧第二版失败继续断言第三版必失败，也没有在未核对版本时宣称整体已通过。

## 第四版复审与WebKit脚注探针

审查版本：hook SHA-256 `ec3ef32636a718d6518b2be78e8f97b4ccfd003495de0ba678afd97a42d76360`，独立探针前后相同。

### 静态结论

- composer例外已限定实际`[data-composer-text]`；`composerHadFocus`只在document失焦时作为后备，focusin到其他控件会更新为false。document focusin监听有对应cleanup。BODY失去textarea焦点且document仍聚焦时，不会被旧标记无限解释为caret复位。
- pointerdown现在允许消息内导航按钮建立输入归因；pointermove仅buttons非0续期，不将普通悬停变成滚动意图。仍需实际scrollTop差异才接管，200ms deadline无新增计时器；Disclosure/toBottom显式清空，unmount资源配对完整。
- 原R1的输入优先接纳路径保留；未发现第四版新增已证实的行为或资源阻断。200ms归因仍是有界启发式，不能将浏览器测试等同于所有原生输入/AT验证。

### 实际探针

root批准独立23826。使用现有quality-preview、生产ChatView与Markdown、WebKit26.5 headless；mouse为窄viewport桌面上下文，touch为390×844 mobile上下文。只使用fixture会话；未改源码，未写scrollTop或调用hook方法。每个上下文先构造长消息，包含开头`开头脚注引用[^review]`、60段正文、末尾`[^review]: 注脚正文`。定位`[data-md-link*="fnref"]`真实脚注回链，先聚焦`[data-composer-text]`，确认回链已在消息视口内，再分别click/tap。之后每RAF只读目标offset，持续1000ms。

| WebKit方式 | 起始top / view / gap | 导航后top | 连续样本 | 目标offset范围 | 结果 |
| --- | --- | --- | --- | --- | --- |
| mouse | 3403 / 663 / 0 | 605 | 100帧，末1001ms | 0.390625–0.390625 px | PASS |
| touch | 3513 / 663 / 0 | 715 | 99帧，末1007ms | 0.390625–0.390625 px | PASS |

两组事件均为trusted；pointerdown时activeElement=TEXTAREA，click时变BODY，随后的scroll留在目标，没有被caret例外拉回。目标实际ID为`user-content-fnref-review`。

最初用普通聊天heading作目标的probe因fixture没有slug而在前置检查失败；Markdown仅doc模式生成heading slug，故换用生产已有的GFM脚注回链。该准备失败不计产品回归，也没有被算作通过。

进程与隔离：两次probe都在finally关闭browser/server，23826已释放；cache仅`/tmp/khor-q010-review-vite`。没有触碰root的23820，也没有运行headed浏览器争抢焦点。已通知root可以开始正式baseline红。

### 当前结论

第四版在本次独立静态范围与上述WebKit鼠标/触摸真实脚注导航探针中**无已证实阻断**；第三版的Markdown焦点候选已由具体路径验证消除。Q011焦点方案结论保持。root仍需将冻结脚本的正式baseline红、第四版绿、既有门禁与集成版本指纹登记为任务关闭证据；本卡不把作者尚在执行的验证提前标成完成。
