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

## 已发布GUI宿主退出后的ready诊断生命周期

2026-10-31，公开Load实际70988已完成New/Join/退出/Load/旧历史验证，但两个成功发布的自有ghost在ACP断开后留下`khor-gui-ready-<parent>-<nonce>.why`。源码`gui_host.rs`成功路径删除ready；结束路径以`!ready.exists()`判定尚未就绪，因而成功后也会写启动失败诊断。两文件各49B，内容为agent closed connection；真实provider/host均已退出，不是Load失败/进程泄漏证据。原actual result false保持；两exact诊断另经所有者消失、identity/SHA/lsof门可逆保留，postrun收口receipt31a4c020…9a9cd。

下一轮应明确区分已发布与未发布的启动握手状态，限定`.why`给仍在等待的启动者，确认Ready前拒绝仍可见、Ready后错误由既有持久状态与事件渠道呈现；不以临时文件存在性推断状态。本轮只记录，不追加产品改造或新测试矩阵。

## 跨会话的轻量阅读书签

2026-10-31实际64MiB原生历史场：HEAD搜索定位后切到别的会话再返回，会重建ObservedSession/ChatView/reader并从tail读取。原R2-H/CONTINUITY约定为同实例跨页锚点与切换取消，未约定跨mount/App重开书签；按收口规则只记下一轮，不在本轮增持久窗口/完整历史缓存。未来如实现，只保存每个view/home的有界轻量来源身份+锚点，由当前history source重新验证并按可用范围恢复；不得保存全正文。原黑截图与AXtail不同尚无唯一因果；focused raise/实际指针/滚轮场正文可读，不凭该观察做paint架构改造。诊断LONG-HISTORY-RETURN-DIAGNOSIS在当前evidence/platform准备目录。


## 正式发布发现的检查入口登记缺口

2026-11-01 main eca实际types／unit均0，额外`npm --prefix apps/gui run gates`报告35个check脚本未被npm入口或静态import可达，另chat-recovery已有入口但50-gates文档未列。原`gates.log`与失败结果保留。此为检查发现机制的真实缺口，不冒这35项被unit执行，也不是本次已审功能行为失败。下一轮统一命名入口、组合脚本与文档，并核实际可达性；不把旧发布记录改成所有门绿。

## 安装脚本成功分支末尾的空日志状态

2026-11-01单次onset实际退出1，日志已确认serve up；三节点实际listener/exe SHA与新UI另证均已采用。只读源码定位：serve.sh成功分支最后一条`[ -n "$said" ] && printf...|sed...`在无新日志时返回1，成为远端sh／SSH状态，本地set-e提前退出，缺最终done。下一轮让成功就绪与附加日志显示各自表达结果；本次不修脚本、不重复安装，不回填原onset1。

## lecun既有会话读取busy的调查记录

2026-11-01升级前后同一CLI `sessions --all`均返回1／`document transfer state busy`；实际5467服务已采用新SHA/UI，未观察到_khor持久host进程身份丢失。hinton/turing各2ID升级前后相同。lecun此读取结果保持未知，不宣称新增回归或会话验证通过；后续沿既有doc transfer／CLI read路径单独调查，不扩大本次发布。
