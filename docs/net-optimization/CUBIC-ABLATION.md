# 当前Noq拥塞控制的隔离诊断

本页由普通Release唤醒消融的真实反例触发，不把CUBIC预先选为产品默认。

W block5（turing-20261113T100245Z-0d77409d）有完整source/control/owner/资源及raw分析见证。low原功能门通过，但终端在bulk和idle_after累计排队，scheduled-to-end p95分别8.868761s与9.910280s；这只是该轮n120结果。high原门失败：720计划记录中600成功、2超时、118blocked；完整192MiB文件及PTY关闭均留证。一个终端超时发生在idle_before，也就是文件下载之前。blocked是前一失败后未发送的意图，不是额外118次网络超时。证据E/该run/copy-wake-original-failure-analysis-v1.json；E=/Volumes/disk0/Khor/evidence/net。

这一反例已足够拒绝把W当成已解决原超时；原固定9块仍完整执行，不删失败、不补绿。它没有单独证明传输、拥塞控制或应用排队中哪一层是根因。

准备的源码位于khor-net-cubic-ablation-v1，branch experiment/net-cubic-ablation-v1。以准确C共同fixture tree adbc7d6a为基线，runtime仅将endpoint::transport的Bbr3Config::default切换为当前Noq原生CubicConfig::default，另有明确的实验注释；无vendor、超时、调度、质量、0RTT或信任变化。不含W唤醒变化或R恢复补丁，保持分量隔离。源tree21f37a7a2fb96abcd601db486c26b687767cade2，snapshot74c762dfffb849b9ec4cb5e68f588dd9698df50d。

原始设计协议E/cubic-ablation-design-v1.json保存准备时状态；当前已在NAT全批闭证且调度器退出后派发普通Release compile-only，control44328e0e755fcec5eae0772e07fcbe59eb9eda48、run turing-20261113T110223Z-34e7344c。该producer已0/586.890585s结束并核20files/identity/原native凭据，实际编译8m54s。固定3 high runtime已派发，run turing-20261113T111336Z-865645d6、control f0975c8945327ae2e1513b4a5e0778446dc680a6；真实PID12911运行中，尚无结果。计划先用普通Release、无qlog/net-diagnostics、原high夹具固定3个新场景判断CC是否值得进一步调查，原2秒caller、720记录/场景与192MiB完整文件门不变，替代0。仅作分量诊断，不冒充当时交错的完整性能接受；真实producer哈希及控制已在E/cubic-release-runtime-{plan,control}-v1.json预先绑定。

旧源码中已有CUBIC在随机丢包道路吞吐很差的历史数据，不能为解决零丢包模型的终端尾延迟而忽略它。若CUBIC表现较好，仍需当前Noq的loss/idle/burst/短流与吞吐对照、完整原工程门和最终同源矩阵，才能讨论默认设计；若它仍失败，不能继续把BBR当唯一原因。本实验不删除任何BBR补丁。

性能批次的root串行调度器活着时，只允许源码准备，不抢Turing编译或修改其ledger/progress。NAT补丁38的固定对照现已全部闭证：保留臂3/3强属性过、移除臂0/3，故保留该补丁；详TUPLE-REFRESH-EXIT。两个调查都不替代完整N0–N5。


## 固定三轮的实际结果

runtime111336Z-865645d6结束1/504.032874s，15个artifact文件及全部来源、raw、CPU、owned清理、selected还原已核。2160条计划记录全部ok，三个192MiB文件完整、PTY均关闭；原native门2/3。首轮direct_after=false，因此仍记失败；不删除原门，也不把改走Relay当作同一profile下已通过。

原统计重算：pooled high bulk目录/终端scheduled-to-end p95为351845/334029us；完整文件吞吐中位36.734944Mbit/s。准确早先C参考为580146/350138us、24.307740Mbit/s。冻结早先B/C六个延迟格和吞吐界限均过，但不是同时交错比较，不构成默认设计或根因证明。证据E/cubic-release-diagnostic-verification-v1.json。

下一步复用当前源crates/net/tests/congestion.rs的双向独立FIFO校准、两种控制器、typing后screen和完整失败记录，补真正无应用请求的idle后短流测量；协议先固定实际Noq1.4、零/2%丢包、样本与来源再运行。该纯Noq层不替代真实Node/SSH/hop、公平/取消、手机或最终同源接受。历史Noq1.3丢包吞吐弱点仍保留。


## 当前Noq丢包与静默矩阵

预冻E/cc-matrix-design-v2.json：双向丢包顺序0/2/2/0/0/2%，其余固定300ms、20Mbit/方向、200ms队列和原seeds。每块原20秒bulk及typing后screen；新增仅测量的quiet case每控制器3轮，warm一字节→10秒无应用请求→一字节→6400字节screen。仍用原Noq ask 10秒期限，不能代替Node的2秒期限或宣称产品门已过。所有失败和实际分母全留。

仅隔离树khor-net-cc-matrix-v1增加该测量。source tree25fed870f2637708c85f5a758a583d3d9f44dc65/snapshotcd36eaf2a9227c6b01fc49bf23652dab29873e28，fixture SHA da7e5ca0b414af0a3b278902d8c1222044dafa38b1cc96d453643b76d126aeed。首编译准备112934Z-a0f72d29被格式门拦下，Rust编译/业务0，原失败保留；v2只用其导出格式修正。准确在途句柄见STATE。

分量诊断screen已在runtime前固定：全原调用成功，每loss下各控制器3轮完整bulk，CUBIC吞吐中位≥BBR的90%；各phase p95≤BBR×1.10+20ms。屏幕样本量很小，报告n，不作总体p99置信声明。测量不接纳默认控制器，也不覆盖真实Node/SSH/hop、共享/分连接、取消公平和能耗。


## 构建图修正与实际矩阵

原构建验证图错误地传播了依赖库的dev-dependency：当前net→catalog把catalog自身测试用core也列作本次网络测量的必编译包。Cargo官方明确dev依赖不向使用者传播，typed metadata的dep_kinds提供normal/build/dev分类：[开发依赖](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#development-dependencies)、[metadata格式](https://doc.rust-lang.org/cargo/commands/cargo-metadata.html)。

隔离树修正native_provenance的图计算，只纳入被请求test/example/bench自身的dev依赖，保留所有normal/build边；typed与legacy IDs不一致、缺typed字段或未知kind拒绝，原source/actual row/dep-info/hash门保持。原37项和9个新正反例在Turing先对旧代码精确5红，再对新代码46全绿。源码与全部准备/校验拒绝逐项见STATE和各原receipt，没有删除失败或将未准入产物执行。

最新测量源tree23b5d7e9d841e1411ba84a68db91ed1f95d7bcff/snapshotc8a1ce24babfad4a68af86b4c0d530596932c1be；Rust runtime和quiet夹具与前版相同，仅两份Python验证文件新增修正及回归。producer120149Z-9c128417成功0/74.067894s（Rust18.97s），22files闭证。runtime120549Z-13cd71dd已派发：control7b1b5f0b588470af70afc7d04e6e0361c932c880，绑定真实producer及19个固定场景，E/cc-matrix-runtime-{plan,control}-v1.json。该矩阵仍在途，无最终性能或产品接受结论。


## 当前Noq完整矩阵结论（已闭）

120549Z-13cd71dd完成0/1115.477027s，50files、来源/CPU/私有namespace/全部owned清理核验；固定19/19原native门，1101/1101请求ok，替代0。完整E/cc-matrix-comparison-verification-v1.json。该纯Noq层的10秒ask期限与Node原2秒期限分别记录。

| 双向随机丢包 | CUBIC bulk中位 Mbit/s | BBR bulk中位 Mbit/s | 结论 |
| --- | --- | --- | --- |
| 0% | 16.507651 | 11.686758 | CUBIC吞吐较高，但小burst并非全面更快 |
| 2% | 0.325742 | 10.067335 | CUBIC未过预冻90%吞吐界限 |

0%下typing后screen p95 CUBIC429.567ms/BBR307.491ms、quiet后screen430.954/307.321ms，均各n9，CUBIC还未过这两格原延迟界限。2% bulk echo p95为763.900/455.850ms。静默后小读约304ms，不能简单宣称某一算法全场景最优。模型全部观测中BBR最大1.283s、CUBIC1.407s；这些数不是原Node 2秒超时的重现，也不证明已定因。当前不采用全局CUBIC。

Rust libtest会把test-name前缀与第一条stderr指标写在同一行。原行首解析器会漏首CUBIC bulk/quiet；v1原件及SHA保持，E/cc-matrix-parser-correction-v2.json记录确定性修正为收集完整marker。原2个bulk/6个quiet计数、样本/quantile/期限/门保持，v2逐条重算所有summary，无修改raw。所有1101结果均保留。

测量代码及验证图修正已提交隔离分支experiment/net-cc-matrix-v1，commitd6b97c3d41d13fb1b9ef209adb89975bfdd727af、tree23b5d7e9d841e1411ba84a68db91ed1f95d7bcff，不合入main/canonical。

下一问题是在保留I117的原BBR runtime时，Relay候选配置是否改变真实Node长尾。新的khor-net-direct-isolation-v1只改实验夹具，并把endpoint.rs逐字还原I117。Main用于目录/文件、Tunnel用于终端本来已分开；这不预先决定再开连接。E/direct-isolation-design-v2.json预冻六场normal/direct/direct/normal/normal/direct，原720记录/场、192MiB/场、2秒caller及全部业务门保持。禁止把夹具关闭Relay推广为生产建议，候选的配置/发布也不独立证明同时活跃的Noq路径。准确在途与来源以STATE为准。


## Relay隔离前置失败与修订

原D producer124033Z-8cfa87a1已通过普通Release编译/来源核验。固定6场runtime125829Z-17b3241d完成1/195.119839s：第1normal场原门通过、720业务全部ok、192MiB完整；第2场在计时前发现两端仍发布内置Relay，前置失败，余4场未执行、替代0。对照无效，完整partial verification见该run目录，原raw/失败均保持。第1场bulk目录/TTY scheduled p95 655612/317657us、文件25.192292Mbit/s只属单场正常配置观测。

实际node::chosen_relays明确在名单为空时关闭only_mine，作为手改空文件的安全兜底；写入口本来就拒绝该组合。把空清单当禁用开关是实验假设错误，不能因此判产品违规，也不能新增禁用开关来配合测量。

新设计E/relay-availability-runtime-design-v1.json保持同一有效Lab Relay清单，两组均核两端实际发布地址。故障组只停止并wait实验Relay的原child，再从该private设备实际探测监听口必须ConnectionRefused；正常组必须真实TCP可连。两组都按原要求先Direct，再执行相同720记录/192MiB/2秒caller及全部原门，顺序available/unavailable/unavailable/available/available/unavailable。变化在节点启动前发生，不冒充在途断线恢复测试；死Relay的后台重试成本包括在内，可用也不独立证明存在同时活的Noq多路径。runtime与I117 BBR逐字一致。当前只构建，不接受优化结论。
