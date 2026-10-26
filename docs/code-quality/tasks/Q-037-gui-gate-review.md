# Q037-g GUI replay gate 注册非作者审查

结论：受审4文件无阻断，可提交/集成。本审查只读源码与已有真实运行证据，不Cargo/浏览器复跑/改作者卡/生产文件；不等于GitHub远程CI已运行，不main merge/ship。

## 固定源码

集成树 `/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`，HEAD `fe265dadcc18d59eca710e4a308bfb584f84dc9a`，仅以下4文件dirty。审查前后SHA一致：

| 文件 | SHA256 |
| --- | --- |
| .github/workflows/quality.yml | 225c38f9b4b156788273c545293480328be34ad935fbb5e90ccc45cc90ab3a40 |
| apps/gui/package.json | 0b72957dd717ff2943a460ddf64b076840c7878b83601030314838a260904bd4 |
| apps/gui/scripts/check-chat-replay.mjs | 1dae77de4e63655d923390e4cea90246e01c1f05767cfe36667b1786917411d1 |
| docs/design-system/guidelines/50-gates.md | b8e4373d59cfa106c7edb713128a094347f39e98a20954d599d3babe088c0320 |

按review-changes图优先：minimal context/detect_changes实际索引仍main8704，未覆盖作者集成树diff；不采其low risk作为候选结论，回退实际diff与完整相关调用/CI步骤。

## 独立源审

- npm新增chat-replay正式入口并加入test:interaction的&&链，失败保持非零。workflow原有npm test通过pretest执行gen，前置生成catalog/bindings；之后安装playwright-core对应的chromium/webkit及系统依赖，才执行interaction。PLAYWRIGHT_CHROMIUM_CHANNEL空字符串使runner选安装的bundled Chromium而非要求Google Chrome；CHAT_REPLAY_WEBKIT=1开启第二引擎。
- CHAT_REPLAY_CHECK_DIR设为runner.temp独立目录，if:always上传该目录。默认改OS tmpdir及path.basename，去除本机外盘路径和手拆POSIX路径。环境变量可指定本地Khor分类证据位置；本轮实际运行使用该位置。
- pageerror与所有console error分别收集并断言为空，无favicon特殊过滤或全局吞错误。HTML内联空favicon明确取消无业务意义的浏览器默认favicon请求；networkErrors仅记录HTTP>=400响应作诊断，未冒称完整网络失败探测。
- selector非空/未知/重复仍在mkdtemp前校验。fingerprint在前置文件校验后读取7源，运行后再核并写报告。每场景finally关闭page，正常引擎结束关闭browser；主finally覆盖浏览器/服务运行失败时的browser/server关闭及fixture/cache目录删除。证据目录有意保留。实际集成GUI目录已检查无.chat-replay-check-*残留。本轮未新增资源持有方或修改真实API调度。
- 文档准确区分受控Tauri poll与真实host/native验收，没有夸大执行平台或原生协议覆盖。

## 实际证据复核

读取 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q037-gui-replay/root-integrated-final.log` 与同名目录report.json：

- finished=true，26行全部ok，Chrome/WebKit各13，errors/consoleErrors/networkErrors全部空。
- sources==sourcesAfter，7源全部与当前集成树逐文件SHA相符；不只相信日志26/26文本。
- 对照root-integrated/report.json确实首次26行中25通过，Chrome failed-empty由于console404失败；root-favicon-diagnosis单项同样失败；root-favicon-fixed单项1/1通过；最终完整26/26另一次运行。原失败未删除或改成成功。
- 当前git diff --check通过，4受审源SHA审查前后稳定。只核已有运行，不额外启动browser/Cargo/native槽。

限制：本机运行结果与CI静态安装/环境契约已核；未实际触发远程GitHub Actions，也未执行整套interaction或重编真实原生App。这批只注册已独立审查的replay gate及其运行器诊断/可移植性。
