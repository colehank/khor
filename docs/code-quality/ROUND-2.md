# 发布后的测试与下一轮问题汇总

状态：本轮已合并main并ship，进入实际使用反馈；**未开始下一轮实现**。构建提交320a48d9，Mac已运行、iOS0.1.14（60）已上传待Apple处理、三台集群已核新版本。证据与哈希见SHIP和STATE。用户要求先测试、总结，再定下一轮范围。

## 当前真实反馈

| ID | 问题与事实 | 当前判断 | 下一轮先做什么 |
| --- | --- | --- | --- |
| R2-001 | 用户报告旧GUI卡死，再运行旧binary也识别不到session，随后强制kill | 无卡死瞬间主线程栈；曾观察旧宿主高CPU/系统换页，唯一根因未证，不冒新版本已修 | 补卡死时自动取样/IPC与资源时间线，固定复现后定位 |
| R2-002 | 终端被kill后Khor显示失败，但Codex后台仍执行 | 本机Codex0.165实际daemon→app-server→工具进程独立于Khor终端；失败描述混淆宿主/连接与agent任务 | 分开表达连接、终端宿主、agent执行、历史可恢复状态 |
| R2-003 | 原Codex可iTerm resume恢复；普通Khor import只有状态，无终端画面 | 普通import只写观察身份，不迁移iTerm PTY；相关导入/显示代码与8704相同。缺实际rowflags，未排除所有真实attach后空白的回归 | 明确“仅状态”能力，提供可理解的恢复入口；定义导入/恢复/接管副作用 |
| R2-004 | 集群发布后hinton首次--all、turing settled --all各一次document transfer state busy | 原失败保；各机其他成功post观察证明原10IDs保留，真实服务进程/SHA/UI稳定，不称同轮CLI全绿 | 固定只读查询与后台同步并发条件，定位状态锁/错误呈现 |

用户已确认9个旧会话失败来自其强制kill；原记录均保留，默认5条年龄折叠不是数据丢失。不要因此自动重跑旧命令。

安全恢复的既有路径：当前任务完成后，正常退出外部同一Codex界面实例，在Khor＋新建本机/同目录的受管Shell，再手动执行codex resume选择原对话。普通import不会接管PTY；“转GUI/接管”有结束原进程等副作用，不能偷偷代替import，不能自动启动同一会话的第二个writer。

进程关系只读核查与import源码分类见发布证据；Codex提供thread/read、thread/resume、turn事件等协议，[OpenAI App Server文档](https://learn.chatgpt.com/docs/app-server)可用于下一轮恢复能力设计，本轮未实施。

## 实际使用清单

关注键盘展开后的最新消息/历史锚点、commit连续跳转、编辑保存/退出保护、模型账户/权限、附件拖放、会话切换和跨设备同步。每条反馈记录设备/构建号、步骤、预期/实际、连续复现及证据位置；未操作项保持待测，构建成功不替代真实手感验收。

## 上轮已登记项

| 项目 | 已知事实 | 下一轮先做什么 |
| --- | --- | --- |
| 040取消deadline | 原组合失败保 | 固定复现并定位 |
| native关闭交接 | 两项channel closed，唯一race未证 | 区分窗口寿命与迟到查询 |
| 草稿退出覆盖 | 会话内/机器文件未报全局draft；全局WorkspacePane已验 | 明确全部编辑入口owner/保护规则 |
| GUI自动来源证明 | 漏TAURI_CONFIG/REMOVE_UNUSED_COMMANDS；实际构建/ACL另验 | 补证据覆盖，不回填旧失败 |
| OS验收退出契约 | 持久terminal跨GUI存活，harness要求所有后代消失 | 区分产品持久owner与fixture收尾 |

以上均待本轮反馈汇总后由用户确认优先级和实施范围，不自动扩入已完成发布。
