# 两端都换网后的真实业务恢复

测试树7bacfb18/treea2f7a04c只在产品集成I3 6f4c8c39上增加两个ignored场景与显式mod路径，产品代码不改。普通Release producer214246Z-2606e4e9已0/598.284484秒、22files/identity/strict native闭证，实际CLI fresh=false且SHA逐字F3/I3 a2a655f7，完整测试27153e02。原首次模块路径预检失败213848Z-491c518e保留，Cargo编译/产品/业务0；随后只加路径属性，不改门。

E/dual-replug-design-v1.json在写夹具前冻结，原生准入后再绑定dual-replug-runtime-plan-v1.json。固定V4/dual/dual/V4/V4/dual六场，每种三次，无替代/运行编译；每场2个setup真实读，再两方向各20个新marker读、2秒caller/500ms计划间隔、每方向首次有效响应≤2秒。两端实际eth0 replug都完成后才并发第一对业务，期间没有前台探针、诊断或手工重绑。

220320Z-44d72463 **0/218.669043秒**，18files/identity/native/CPU/owned完整，独立重算全部起止人口、两端内核旧地址消失/新地址存在、dual两端/64前缀变化、独立固定UDP反射地址变化、四router规则前后读回。**252/252真实请求成功，六场全部过原门**。

| 场次 | 两端变化 | 首次a→b μs | 首次b→a μs |
| --- | --- | --- | --- |
| 1 | both-v4 | 508542 | 507954 |
| 2 | both-dual | 1475417 | 1674252 |
| 3 | both-dual | 705948 | 706564 |
| 4 | both-v4 | 507315 | 507911 |
| 5 | both-v4 | 508075 | 507472 |
| 6 | both-dual | 707732 | 708339 |

V4场是两个Home IPv4 NAT各移到独立Strict NAT，稳定私有Relay可达；dual场是两个Campus各移到独立Cellular，双方IPv4/NAT及IPv6 /64一起变。反射器是独立稳定第三设备，其端口不是Khor QUIC端口，变更后的反射只在全部业务完成后执行。

这验证“两端都变化后自动恢复”的两个限定几何，不能说内核操作原子同时、成功业务必走新IPv6、IPv6-only/同prefix IID两端变化已过，或物理Wi-Fi蜂窝/休眠/能耗已过。没有同期baseline或足以估计总体可靠率/p99的人口。完整N4与手机仍见REQUIREMENTS。

完整E/dual-replug-summary-v1.json、原run/dual-replug-runtime-verification-v1.json及全部原日志保留。
