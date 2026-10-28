# Q034 非作者审查

结论：**源码与对应生命周期回归审查通过，无阻断，建议进入串行集成**。审查者native_acceptance，2026-10-25；未改生产或测试源码，未启动Cargo。

固定版本：`b732f4de5de5b8c952f213979e4fcbb94d53fd5d`，父`fb0c67001d003dda9d639f00cc10b86af0d50aec`；工作副本`/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-store-owner-recovery`，tracked clean。范围仅ChatView.tsx与两份专用回归脚本。

## 核对的行为

- **真实gone停止**：结果先完成frames处理/paint，再根据batch.gone将outcome标为ended；finally将finished置true且不设下一个timer，tick开头亦拒绝finished。pollNow/visibility不能复活；只有新attempt建立新effect可重开。核对core chat_poll的gone来自实际attachment结束标志，chat_wait在gone时立即返回；这里确实消除原0ms空转。
- **失败deadline**：默认failed，只在完整成功处理后变live/ended；retryDelay从100ms倍增至5000ms，成功归零。tick在调用API之前检查retryNotBefore，pollNow虽能撤掉已有timer，却只能按剩余失败deadline重新排队，不能提前请求。asking保证同owner只有一个in-flight。
- **后台与前台恢复**：finally同时读取document.hidden，避免visibility状态已变、React pace effect未commit的一拍竞态；hidden延迟下限10s。retryNotBefore只保存失败自己的100–5000ms，故foreground较短pace可取消背景timer、在失败deadline后恢复，而不是被旧10s节拍额外困住。HTTP路径保持chatPoll与当前pace下限；未把native正常long-wait成功变成额外等待。
- **成功/attempt重置**：完整成功后retryDelay/retryNotBefore归零；attempt重建effect使finished/asking/cursor/退避各自新建，并清掉旧timer。
- **迟到结果归属**：owner改变/unmount cleanup先stopped，清timer与pollNow；then在任何setState/cursor/replay之前检查stopped，finally也检查，所以旧success/error均不paint、不replay、不续调。新owner effect独立，不共享闭包的asking或timer。

## 证据与独立复验

作者最终`Q034/final/report.json`为Chrome/WebKit各11项，共22/22；三项source SHA与当前提交逐一匹配。红`red-actual`原source下immediate-failure/gone-empty实际失败；中间visibility两项红与后续修复记录均保留。selector-validation.json确认空/未知/重复选择均非零退出，不能用空跑充绿。类型检查由作者记录通过，本审查未冒称独立再跑types。

非作者使用**原提交脚本**重跑Chrome/WebKit的gone-empty、visibility-backoff、switch-owner，共6/6，进程exit0。结果`/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q034/review-native-acceptance/report.json`与同级`.log`；脚本前后source指纹一致、最终与固定提交再次匹配。命令环境：`CHAT_POLL_SCENARIOS=gone-empty,visibility-backoff,switch-owner CHAT_POLL_WEBKIT=1 CHAT_POLL_PORT=23849 CHAT_POLL_CHECK_DIR=.../Q034/review-native-acceptance PLAYWRIGHT_BROWSERS_PATH=/Volumes/disk0/Khor/caches/dependencies/khor-playwright node scripts/check-chat-poll.mjs`。

真实React组件和production api.ts在两引擎内运行，只替换Tauri invoke/fetch传输并以Playwright clock推进window timers/document.hidden事件；**不是原生App或真实网络断连验收**。有gone终止、退避上限、healthy cursor/立即续等、attempt、owner/unmount、visibility、HTTP分支覆盖；真实后台OS定时器限制和最终集成App仍由后续原生复验承担。

已先用graph minimal/detect/flows/tests_for/impact。图谱HEAD落后于固定提交且tests_for没有映射，故未把0 flow/risk当证据；实际源码/真实组件报告优先。

## 非阻断建议

后续扩展可为“gone且包含最后一批frames”加直接paint+停止断言（现有gone测试主用空frames，正确顺序本次已逐行核对），并在最终集成版本重跑原22项。本次不需为这点扩大生产改造。
