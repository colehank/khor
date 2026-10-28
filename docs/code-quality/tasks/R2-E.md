# E 输入与草稿完整性

状态 working；2026-10-29用户“开始吧”授权下一轮实现。main/ship未授权。
owner: round2_input；工作树 /Volumes/disk0/Khor/worktrees/current/khor-round2-input，fix/round2-input，基线e7c335c8。
条目 R2-011/012/027/028/L03；先完成R2-027迟到读取/组词最小复现与保护，不声称整包完成。
允许写：CodeEditor/FileView及其纯模型/定向测试；其他输入hook/路径/草稿入口需先向root报告新路径所有权。禁止改历史链、全局App/TerminalPane/样式/生成协议。
规则见EDITOR-INPUT、ROUND-2-EXECUTION、ENGINEERING。先图谱、失败证据、最小修复、职责整理；不要用定时延迟掩盖竞态。记录真实IME尚未验证。
证据 /Volumes/disk0/Khor/evidence/round2/input；更新本卡记录实际文件、接口、验证、下一步（仅本卡可以由owner写中央文档）。native/Cargo槽须root协调。保持用户App/会话。

## 首片段：后台文件读取拒收旧回包

作者树提交 `447419cf81a811a9bc7da446acb96b7eacdffd56`；状态review-ready，整包E未完成。

确认旧again闭包可在读取中发生输入、撤销或组词后覆盖新输入；旧源同8例1过7失败，新源8/8通过。检查执行真实again/onText/setEdit回调，输运回包顺序受控，不挂载React或启动浏览器。生命周期失效模拟、composition DOM事件与真实IME不冒通过。修后tsc、diff检查通过，证据refresh-receipt.json/refresh-red.log/refresh-green.log/types.log位于既定input目录。

实现：file-refresh纯协调器持有编辑代次、请求序列与composition活动状态；所有setEdit在调度React状态前同步失效旧读，machine/path layout清理失效旧请求；后台回包只有无中间编辑/组词且仍最新才能同时更新merge基线、缓冲与缓存。CodeEditor只增加组词状态通知，不更改任意外部text替换策略。

Clean Code：删除已不完整的长refresh历史说明，保留merge基线不变量；读取接纳规则由纯协调器唯一持有，组件保留输运/呈现副作用。

独审round2_reviewer未发现首片段阻断；按意见明确真实挂载/组词结束最终事务时序尚待验证。接下来R2-011/012路径/搜索退出先报告新增写路径并独立复现；R2-028、通用外部text组词策略和L03退出草稿未完成。无native/Cargo/App/真实用户文件操作。
