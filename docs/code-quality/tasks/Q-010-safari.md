# Q010 Safari长草稿测量与晚回跳

状态 working，所有者 root（2026-10-26接管）。原native_acceptance因额度中断，进程已结束；不依赖其尚未找到的组件报告。

## 固定基线与实际产品红

源码 c37151a1，Safari native=false，iOS模拟器FE04AC7C-B375-4F2E-AE7E-B6F8F5608947（实际UDID以run11 receipt为准），真正MobileSafari/UIKit。证据 Q010-native-20261025/run-11/safari-keyboard-reader-mailbox/acceptance.json 与附件：ACP长稿尾部/历史通过；Khor新独立visibleDocument收到长内容后，真实键盘收起前五次strict settled gap0，第六次gap164，后续恢复。不把首次 setup焦点/active格式/mailbox文档归属失败当产品红，不放宽阈值。Khor reader在红后未执行，不能声称通过。

同源c371质量App/App lib在验收后仍原SHA；一次Swift relink误动DerivedData App的完整恢复receipt见run11/safari/restore-original-app.json，原安装App一直未变。不将Safari替代native或真机。

## root受控复现

复用 /Volumes/disk0/Khor/worktrees/current/khor-quality-native-io，分支 fix/quality-safari-compose-range，HEAD c37151a1。图谱旧55d索引不能代表当前源，按实际hook/Composer/SessionBar/CSS定位。

生产useComposeLayout每次render暂时移除expanded并设height:auto以读scrollHeight，消息区被临时放大164px，浏览器clamp scrollTop。恢复尺寸后ResizeObserver可能无变化且稳定log引用不触发布局同步。

新增真实SessionScreen/BarSeat/Composer/useChatScroll夹具，父flushSync重绘；32组：Chrome/WebKit、有/无只读scrollHeight probe、长/短草稿、尾部/reader、稳定/新log引用。Q010-safari-controlled-red/results.json：固定前后source指纹，28绿/4红。四红恰为两个引擎各有/无probe的长草稿+尾部+稳定log，after gap164；reader/短稿/真实新log对照通过。

## 当前候选与待验

use-compose-layout.ts在紧凑测量时暂保留root当前minHeight，finally恢复原inline值及测量锁；保持原有input/ResizeObserver/render触发，不加timer、不删除触发、不改视觉布局。Q010-safari-controlled-green与Q010-safari-fixed两次固定32/32通过，source稳定；第一生产提交54aa0f3f。实际同款MobileSafari/UIKit driver无重建通过：ACP/Khor各六次收键盘settled gap0，reader锚点开合键盘保留；26份原始requested指标与34附件，Q010-native-20261025/run-11/safari-compose-fixed/acceptance.json。候选hook与测前保存内容一致，driver SHA与原真实红完全相同。root截图检查末段确实位于输入框上方。

补充草稿清空/缩短与宽度矩阵发现WebKit ResizeObserver warning；旧c371与minHeight候选均有（Q010-safari-transitions-baseline），不是新回归。原width reader gap>100断言不适用于文本重排变短后的几何（同一情况40仍在历史）；仅将宽度后断言改成不被拉到尾部>2，保持固定宽度时严格首段与offset。原失败报告保留，不当成产品红或绿。

第二独立候选将ResizeObserver触发的测量合并到单个可取消rAF，避免在observer交付周期内改被观察root尺寸；input和React layout仍同步，minHeight修复不靠延迟。四组补充矩阵已实际通过、无pageerror；默认32/已有scroll34/types复验中。该候选还需独立提交与最终实际Safari复验。没有main合并/ship授权。
