# D 旧GUI卡死：限定调查

更新2026-10-30。范围R2-001。状态：已完成当前可得材料的限定调查，原故障仍未解决；不把H长历史有界化当作原卡死唯一根因。未改产品，未取代真实重现验收。

原反馈缺卡死瞬间的主线程/WK栈、IPC等待和内存压力时间线，旧宿主高CPU/换页观察不能证明唯一根因。当前只读进程快照和Root5c实际源码核查已保存在 `evidence/round2/diagnostics/freeze-current-20261030/observation.json`，内盘逐SHA镜像 `recovery/r2-d-limited-diagnosis-20261030/manifest.json`。当前瞬时CPU/RSS不是UI响应度测量，且不含WK/GPU总量，不据此说已修或当前必不挂。

现有 `.khor/face-notes.log`按256KiB轮转、单行400字符，主要是终端attach/touch观察，没有通用主线程或IPC心跳。相同`khor-gui`二进制既运行GUI也运行持久宿主，不能凭进程名批量kill。调查没有结束/重启用户App、修改session或读取用户消息/日志正文。

复现时的最小采集：记录UI操作、时刻、会话类型和历史大小（ID匿名化），确认GUI PID、可执行路径及进程出生身份；在实际卡顿期对确切GUI和对应WK渲染器各取两次5秒sample（每次15秒外部时限），前后核PID身份。同期保CPU/RSS、系统memory pressure及有界pending IPC时间线，原始栈留本地、分享前去命令参数。不得先kill再采集，也不主动制造用户会话卡死。

没有原现场或新的复现就保留unknown；不追加常驻watchdog、全局日志框架或新性能矩阵。下一次真实组合验收若重现，以该窄采集包继续定位；若无重现，交付必须如实列出原问题未证修复。
