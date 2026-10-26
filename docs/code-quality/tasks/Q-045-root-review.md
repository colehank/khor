# Q045 本地SSH生命周期：主智能体独立审查

状态：review通过，可进入集成；精确集成运行待。作者树fix/quality-ssh-lifecycle，固定8a22cfdb+a1625c7f（HEADa1625c7fc475b28e14d23600ccd9e0d95b2add0d）。root非作者。

先用图谱detect/query，图谱HEAD旧且0flows不能证明覆盖；随后实际读取common Session/reaped receipt、完整join_process、finite SSH builder、调用方与测试/fixture；再核Q045/delivery.json源SHA、真实红与回归日志。源冻结clean。

结论：在声明的本地自有进程/管道边界内无新增阻断。退出结果cap1、仅reap后发送且poll消费一次，不因自然退出提前取消未读尾部；stdin/stdout/stderr并行，绝对期限包含输入，失败和超限回收并join。stdout1MiB失败、stderr256KiB保尾带标记，EOF单独不算进程成功。有限SSH不新建持久master或后台fork，保持既有ControlPath；显式jump同约束。IgnoreUnknown只针对较旧平台SSH缺少ForkAfterAuthentication，属于实际平台适配，不是旧Khor兼容。

验证：作者先实测旧200ms期限被stdin拖到3.036s、双pipe有限输出超时两条真红；候选blocked226.6ms、双pipe72.8ms、取消122.9ms，独立sentinel存活。135不同相关用例及正常Node库check通过（最后IgnoreUnknown后join20/satellite35/check精确重跑），36fixture晚检查ESRCH。root核交付13源指纹和命令/退出/资源回执。作者树绿不是新集成组合绿，集成后继续实际重编及适用组验证。

限制：未实跑Windows、真实SSH/master、GUI→remote-add取消API或远端执行lease；相邻gzip/fetch不在此deadline。OS spawn syscall不可抢占；底层既有Unix Drop在异常终止权限失败时的wait不具可证明绝对期限，用户ProxyCommand自行脱离/保管道不在本批闭环，不能声称远端回收。同步MCP/远端取消另卡继续。无main合并/ship。
