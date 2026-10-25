# Q-021b 非作者审查

审查者：/root/store_owner。作者/root 的工作副本：`/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-recovery`，基于 `75e1afc3` 的未提交差异。2026-10-25。

## 结论

本轮未发现确定阻断。先独立审查生产 diff 和完整 useLock/LockScreen/App 锁定返回路径，再检查回归覆盖，未以作者的通过报告代替判断。

- 初次配置失败不再 setLocked(false)，settled 后 App 仍返回独立锁屏，未知时不挂载 PIN/系统认证表单或会话正文。
- 首次成功读决定初始锁定；后续读与失败保留已知配置和该窗口当前锁定状态，不撤销 PIN/系统认证的成功结果。
- reread 同步递增 epoch，旧成功/失败在 React effect 清理前即失去归属；alive 与 cleanup 进一步覆盖 StrictMode/unmount。
- 失败后串行重试；间隔 1/2/4/8/16/30 秒，30 秒封顶，成功后重置。手动重读清理旧定时器；请求实际取消未增加，但旧响应不能改变当前状态。这里有界的是重试间隔，恢复前会持续重试。
- PIN 每次尝试先清除两类旧错误，false 显示错误 PIN，throw 显示暂时无法验证；两者均不解锁。
- 重试按钮沿用已有 Button 样式，不引入新 variant。

## 独立验证

在上述工作副本执行 `node apps/gui/scripts/check-lock-state.mjs`，真实 React/Chrome headless 的 20/20 场景通过，检查期间四个源/测试文件 SHA-256 未变。证据：`/Volumes/disk0/Khor/evidence/runs/khor-quality-Q021b-review/{lock-state.log,results.json}`。

图谱先调用 detect_changes，但服务仍索引 main，未覆盖此 recovery 未提交源码；因此风险分数未用于结论，审查基于现场 diff/源码。

## 边界

未重复作者正在执行的完整 App Chrome/WebKit 交互，也不声称原生 App 验收通过。网络请求永久不返回的超时语义及系统认证 Promise 的错误处理是既有边界，本次改变只覆盖明确失败后的恢复；未发现此次修改引入的新阻断。

源/测试指纹：
- `apps/gui/src/hooks/use-lock.ts`：`0cd565f4f28ac76916cff4a0aadb19e0a80fa735c79323bc6a21da3a660ba1b1`
- `apps/gui/src/views/LockScreen.tsx`：`b5ced742d0e7fc8ddeeba1fea9dea55c5adb2bcd3c3ce23bdac258892a0d9ee6`
- `apps/gui/src/App.tsx`：`edf8c2dfbd02a6da19e13b4891852a8ece4420d45c4dd1f85c76d17ece295f58`
- `apps/gui/scripts/check-lock-state.mjs`：`d04ff8e8a4ab01e3975b5cb125017037b521a2fb3b1c028901d91576270d7be9`
