# 当前集成版核心路径回归

准确R/I2 runtime、tree90a1d781、普通Release CLI7e8c52bc及完整netlab e4f2f52c沿用MOBILITY-RELEASE。原始源码、门槛、业务期限及几何不改，无运行编译。

原9项174412Z-b676ad99返回1/564.240656秒，19files/identity已收。前6项完整原native通过，全部423个已记录目录调用成功；四个速度优先场景各30，p2共33，p3前后共270。p6另有未纳入attempt记录的原双向准备读，不计入423。

| 场景 | 已核结果 |
| --- | --- |
| 仍有慢直连，选择快Relay | 两端报告Relay，30/30，p95 148218μs |
| 仅快Relay参考 | 30/30，p95 187227μs |
| 仍有慢Relay，选择快直连 | 两端报告Direct，30/30，p95 26947μs |
| 仅快直连参考 | 30/30，p95 27654μs |
| 当前路变慢 | 约819.7ms后两端在快路并保持；33/33，原门通过 |
| 当前路黑洞 | 故障后260次读取全成功，最慢603.2ms，零wire退役；加故障前10次共270次 |

每个p6保持原30秒caller及p95<200ms门；不是任意Internet体验上限。以上单场次各一次、没有同期baseline，不宣称可靠率、p99或相对优化幅度。报告路不是独立包级最优性证明。

第7项p4需要原600秒观察，加30秒settle及准备，控制器误设owned600秒容不下。发现后root向经PID/PPID/starttime/argv/环境核验的唯一测量Python发送TERM，保留中断与强制清理，未改原业务/观察门；后2项未执行。这不是产品失败，也不能作为完整9项通过。原case outcome中余5僵尸及group_absent=false保持；controller退出后对87个observed PID逐一独立核查，全部已不存在。

剩余原3项175639Z-b739a547单独固定补齐，p4只增加外层owned至1200秒，其余600秒，NETLAB_LONG_S不得缩短；前6不重跑。已完成并独立核验：0/813.552456秒，15files/identity/native/CPU/owned均闭。p4原600秒观察中两端0接口变化，alpha0punch/0selection、2797包/365783B；不等于后台成本接受。Relay c原30秒门通过，fresh TCP20.0秒、peer heard22.9秒；原case没有真实前台业务，不能推出23秒用户中断，另加真实请求测试。uplink e约919.0ms恢复、wire down=false/retired0。当前只可表述9原合同分6+3覆盖，不能把此前中断的批次改写为全过。

证据E=/Volumes/disk0/Khor/evidence/net：原run下current-core-paths-partial-verification-v1.json、cancelled-case-after-controller-exit-v1.json；根下current-core-paths-cap-cancel-intent-v1.json、current-core-paths-cap-cancel-signal-v1.json及current-core-remainder-runtime-plan-v1.json；补齐run下current-core-remainder-block-verification-v1.json。完整N1/N4/NET-16、真实手机、成本与最终验收仍未完成。
