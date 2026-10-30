# 下一轮候选

本页只记录本轮开始后发现的独立问题，不加入当前34项实现或验证。

## 诊断日志轮转的UTF-8边界

2026-10-30 Root在R2-D只读定位现有日志时看到：Root5c的 `crates/node/src/doctor.rs::note_face` 超256KiB后，以字节计算keep，随后直接 `text[keep..]`。包含中文的行可能让keep落在UTF-8字符中间而触发字符串切片panic。此为源码风险，尚未运行复现；不把它推断为旧GUI卡死根因。下一轮用私有多字节日志定向复现，再按字符边界处理轮转；当前不改代码、不追加测试。

## 非WK导航事件关联能力

Wry0.55.1 Windows/GTK暴露的Started/Finished没有真实导航身份，且Finished不能证明成功。当前B只保有view/logical stream观察，Settled不显示成功100；旧A终结事件仍可能落入逻辑B。此SDK能力限制已明确，不以URL/FIFO猜配对。下一轮若需要与WK同等精度，再单独评估实际平台SDK适配并真机验收。
