# R3-01 终端内容打开

基线：main27a9ce5c。Root单独实施；代码与自动验证完成，浏览器／真实App验收待完成。未合并main、未发布。

## 问题与改变

发布版本在TUI开启鼠标跟踪时，普通链接点击被guard拒绝；Cmd/Ctrl路径可打开，但xterm的mousedown仍可能先发给程序。旧测试把此拒绝作为正确行为，遗漏产品契约。

在已识别链接的主按钮按下处保留点击归属，抬起只调用同一xterm provider结果一次；小范围移动仍算点击。超过原TAP_SLOP则把原按下交回终端，不再激活链接。右键、强制选区和普通文字不拦截；有选区、取消、离开或链接来源变化不打开。触摸tap沿已区分滚动／选择的入口打开，不再因程序鼠标模式被拒绝。

URL、OSC8和已验证路径继续使用已有provider／opener，不新增网络可达推断或重写识别器。唯一xterm6内部适配隔离在term-link-gesture，读取core.linkifier.currentLink，以保持原provider优先级。状态与文档级监听由TerminalPane创建、同次卸载释放，弱事件标记不保留会话正文。

## 验证

证据：`/Volumes/disk0/Khor/evidence/round3/R3-01`。

- `intent-red.log`：完成的touch tap在tracked TUI中被旧guard拒绝，21/22。
- `gesture-red.log`：发布基线guard且未接入点击协调时，普通点击没有内部目的地，退出1。
- `terminal-link-intent.log`：生产回调22项与事件harness23项通过；Node EventTarget用于验证局部事件所有权，不冒DOM传播或真实PTY报告证据。
- `links.log`：真实xterm缓冲中软折行路径／URL、Unicode、带空格引用路径及OSC8识别通过。
- `unit.log`、`types.log`：GUI `test:unit`、`test:types`实际退出0；新检查已登记npm命名入口并纳入test:unit。

## 尚待验收

真实Codex/TUI开启鼠标模式时的URL、文件、OSC8、软折行：普通内部、Cmd/Ctrl系统，且程序不收到同次点击；拖动、选区、右键、普通shell与手机触摸不回归。真实App、浏览器和自动harness分别记录，不用旧发布或Lab截图填补。

## Clean Code自审

打开目的地仍由linkIntent决定；是否可激活与手势归属独立，不把程序模式当用户意图。手势的link/program/cancelled状态集中在一个资源owner，私有SDK访问只出现一处，TerminalPane只挂接／释放。注释只保留SDK与事件时序原因；不顺带整理巨大TerminalPane或引入通用手势框架。

Final combination adds actual Chrome/xterm DOM evidence: `/Volumes/disk0/Khor/evidence/round3/terminal-dom-verified`10/10 (URL/path/OSC8 plain+Cmd, actual program mouse report observation, drag handoff, secondary click and Option forced selection). No production change needed. This uses production providers/claim adapter with no PTY or external app execution. Initial fixture failures (escaped payload, testing Mac Ctrl as a primary click, using Shift without the production Mac Option setting) were corrected and retained; none are counted as product regressions. Exact source hashes are recorded in the final rerun below. Real native TUI/iPhone remains to accept.
