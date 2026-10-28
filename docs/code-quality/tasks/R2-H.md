# H 长历史有界加载

状态 working；2026-10-29用户“开始吧”授权下一轮实现。main/ship未授权。
owner: round2_history；工作树 /Volumes/disk0/Khor/worktrees/current/khor-round2-history，fix/round2-history，基线e7c335c8。
范围R2-006/S1，先固定读取/传输/渲染基线与预算，提出稳定分页/跟随/取消接口，再实现一个可独立验证的有界片段。不得先接Codex全量观看或扩成通用分页框架。
允许写：原生历史提供者/GUI历史消费与定向检查；禁止CodeEditor/FileView、App.tsx/TerminalPane和全局样式。图谱定位后先报root实际修改路径/接口再改共享协议和生成器；生成结果不得手改。
读SESSION-CONTINUITY、ROUND-2-EXECUTION、ENGINEERING。证据 /Volumes/disk0/Khor/evidence/round2/history。只本任务卡可写中央文档。
首次返回不能扫描整份历史；单条/批量有预算；原记录不截断。跨页顺序/工具关联/迟到响应与取消要明确。native/Cargo唯一槽须root协调，target按PLAN。不要构建第三套缓存或运行用户会话。

## 首片段检查点（2026-10-29）

已确认读取链：Claude/Codex transcript_since从from到EOF全读；gui-core history_follow整批转帧；ChatView首读from0且全量创建items。首次全量读取和巨型单行确实无预算，但未复现用户App冻结现场。

新增crates/node/src/adaptor/transcript_window.rs及mod登记；Claude/Codex的transcript_window调用共享读取原语。256KiB总读预算（含左边界1字节）、64KiB内联记录预算。before为exclusive、超EOF截到当前观察长度、到0无previous。每页绝对范围；未完成/切开的行返回Fragment引用，完整大行返回Oversized引用，原文件不改。并发变化按实际读量标range。

验证：root授权standalone rustc --test，1.95.0；纯std原语8项通过，包含尾读预算、0/EOF、短读、巨大行跨页前进、半行追加、UTF8边缘、并发截断/追加。legacy-red使用现有read_to_end IO序列的独立algorithm probe，3MiB读取超256KiB预算，exit101；不是链接原provider执行，不能冒完整产品red/green。证据 /Volumes/disk0/Khor/evidence/round2/history/window-check.json、legacy-red.log、window-green.log，产物约1.1MiB。编译已结束并向root释放唯一槽。

未完成：Node crate/vendor wrapper尚未完整编译；新入口尚无Node/IPC/UI实际调用，旧history/follow仍全量。文件身份（等长替换）、片段拼接/按需大记录、工具关联、取消、界面分页/有界渲染均属后续。当前仅基础片段，不计S1或用户卡死已修。

下一步：固定首片段提交供非作者审查；向root报告history_page稳定契约、Node/IPC/生成器/GUI最小修改清单，获共享路径所有权后继续实际消费。完整crate编译须另行协调槽，不能由standalone结果推断已编译。
