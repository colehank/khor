# 下一轮候选

本页只记录本轮开始后发现的独立问题，不加入当前34项实现或验证。

## 诊断日志轮转的UTF-8边界

2026-10-30 Root在R2-D只读定位现有日志时看到：Root5c的 `crates/node/src/doctor.rs::note_face` 超256KiB后，以字节计算keep，随后直接 `text[keep..]`。包含中文的行可能让keep落在UTF-8字符中间而触发字符串切片panic。此为源码风险，尚未运行复现；不把它推断为旧GUI卡死根因。下一轮用私有多字节日志定向复现，再按字符边界处理轮转；当前不改代码、不追加测试。

## 非WK导航事件关联能力

Wry0.55.1 Windows/GTK暴露的Started/Finished没有真实导航身份，且Finished不能证明成功。当前B只保有view/logical stream观察，Settled不显示成功100；旧A终结事件仍可能落入逻辑B。此SDK能力限制已明确，不以URL/FIFO猜配对。下一轮若需要与WK同等精度，再单独评估实际平台SDK适配并真机验收。


## 初始Load期间的交互暴露

2026-10-30，在退役无现产品caller的旧Replay API时，经当前ACP opening调用链和原私有extended fixture核到既有独立限制：`start_with` 等待初始session/load后才把Handle/EventReceiver交给消费者；若加载期间的agent请求Elicit并等待用户响应，UI尚不能消费该问题。当前是源码确认的能力缺口，未用真实vendor复现，不推断为此前GUI卡死或数据丢失的唯一根因。

按“新独立问题只记下一轮”，本轮不为它新增opening API或第二套重放；现有Prompt/Control环境、MCP和互动能力的真实断言继续迁保，不能以移除旧Handle.replay就宣称初始Load互动已支持。后续用专用ACP peer在Load内提问并等待回答，确定事件流与ready future的最小分离；保持同一writer/同一Load、完整canonical前不Ready、Stop/Close有界及真实进程退出事实，再决定产品适配。


## GUI生成缓存的冷重建准备

2026-10-30，GUI实际Cargo成功但原严格来源校验发现Tauri hash资产沿用旧时间：5个第一方script marker均current，唯一GUI OUT有349个旧输出，279被本次dep-info实际引用。当前不改变时间/任意接纳旧文件；最终构建前对精确自有OUT可逆保留再由真实producer生成。后续单独改进准备阶段对这类生成缓存的隔离，让既定来源门仍严格且减少重复编译；不扩成新构建系统。

## 构建失效范围与无关测试源

现有native source_frame包含整个受控源码集，修改仅测试fixture也会使旧生产proof的全frame检查失效。本轮按原helper守规则，先收完消费脚本再做最终构建，不修改旧proof或放宽require。下一轮可按真实生产输入/生成声明与测试目标分别研究失效范围，仍纳入实际使用的Git元数据/环境/依赖；不能仅按扩展名排除，也不能把当前未实现的精确失效当已有能力。
