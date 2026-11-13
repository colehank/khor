# 网络验证运行账本

2026-11-09。证据根 `/Volumes/disk0/Khor/evidence/net/`；每个目录的 source.json / environment.json 才是准确源码和控制器回执。失败按产品、准备、格式或执行器分类。历史源码通过不覆盖后续修改。

| Run | 任务 / 源码 | 结果与适用边界 | 证据子目录 |
| --- | --- | --- | --- |
| 42505117705 | 旧策略 + 速度回归，net-unit | 产品红，60 pass / 1 fail / 1 ignored；90ms Direct 胜过5ms Carried | baseline-unit-42505117705 |
| 42505672250 | 首版候选 net-unit，tree b66d0343 | 61 pass / 1 ignored；不含后续所有 bind 注册 | candidate-unit-42505672250 |
| 42506109052 | 首版最终 net-unit，tree d0fdb004 | 62 pass / 1 ignored，含统一 bind 与 settling | final-unit-42506109052 |
| 42505323213 | 初次 baseline smoke | p6r 产品红；p6d 反向 wire 未建立，是准备红；p2/p3 绿 | baseline-smoke-42505323213 |
| 42506330051 | 修正 fixture baseline smoke，tree e73903a3 | p6d p95 34.02ms绿；p6r p95 369.78ms、Direct311/309ms产品红；p2 3.1s、p3最坏846.1ms绿 | baseline-smoke-42506330051 |
| 42506345592 | 同 fixture candidate smoke，tree ee158fdb | 4 pass；p6d p95 64.96ms；p6r p95 152.12ms、Relay84/88ms；p2 1.2s、p3最坏546.6ms | candidate-smoke-42506345592 |
| 42506805985 | hop 看守新增回归，net-unit | 产品红：relay30ms/hop22ms，固定10ms看守错误释放有价值hop | hop-unit-red-42506805985 |
| 42507205671 | hop 候选 net-unit | 仅格式失败，patch已应用 | hop-unit-42507205671 |
| 42507239186 | 同轮 net-node | 仅格式失败，patch已应用 | hop-node-format-42507239186 |
| 42507643289 | hop/p7候选 net-unit | 仅 p7 assert 格式失败，patch已应用 | hop-unit-42507643289 |
| 42507629732 | 同轮 net-node | 仅p7 assert格式失败，patch已应用 | hop-node-format-42507629732 |
| 42507391687 | p7动态，tree 9ffe76e4 | 产品红：交互与闲置两场景均45s未回Direct；0请求错误；尚未含新采样修复 | dynamic-red-42507391687 |
| 42508095178 | net-node，tree 067d62b5 | SUCCESS，hop9 / wires10全部通过；不含随后vendor采样改动 | hop-node-42508095178 |
| 42508069745 | net-vendor，tree 067d62b5 | 执行器准备失败：excluded依赖不能从根workspace跑dev-dependencies；控制器已改为独立manifest + committed lock | vendor-unit-42508069745 |
| 42507224815 | 完整旧策略baseline，tree e73903a3 | FAILURE：23 pass / 8 fail；6个测量断言红、2个配对阶段busy，另p2i报告red但未assert | baseline-full-42507224815 |

新采样候选：同tree `064925806d1c3962f715942d7f637a4b6257783f`（含全部新增文档，无排除untracked），base bc6cded2，control `16474c5`：

- net-unit **42509122554**，snapshot `3c88d2f67ff691431427cbd3153b76d67d1abf66`。
- netlab-dynamic **42509155432**，snapshot `41fdf5a93ef5fc717145abd87c6b218adce2b130`。
- net-vendor **42509174050**，snapshot `301ecbca4134be116d877ceac658f70545bc92c1`。
- unit SUCCESS：63 pass / 1 ignored，证据fresh-unit-42509122554。dynamic SUCCESS：p7a 2.6355秒、p7i 18.1609秒、零请求错误，证据fresh-dynamic-42509155432。vendor SUCCESS：noq431/iroh路径21通过，证据fresh-vendor-42509174050。source-only脚本轻量测试16项通过。

p6网络配置中的 relay40ms 是单腿配置；完整 A→relay→B 往返约80ms，实际84/88ms。已修正新日志和协议标签，未改变拓扑或200ms目标。前述A/B仅一次，四场景并行、不同托管VM；只能作为首轮证据，不能声称统计显著或替代重复/固定好路径参照。


固定好路径参照/重复与完整候选（control419b5f0）：

- baseline compare **42509612220**，tree `f95f6e54876971b5e8555896196b85fa457a3bca`，snapshot `e1088b13e0391c85998c92fa5338bb533cc58aab`。fixture/脚本与候选一致；生产代码6485cb83。
- candidate compare **42509632238**，tree `4549630fc7df95885de570640c764ca1d60e2735`，snapshot `776925e4e8ecaf7caf561dd38a4a7843c80a8c98`。
- candidate full **42509652259**，同tree4549630f，snapshot `0c2e962f57ed106c4944be399f02f288f1334825`。
- **42509569373 CANCELLED**：提交后发现baseline漏同步NETLAB_TEST_THREADS开关，已主动取消；准备输入错误，不纳入A/B，正式任务42509612220已补齐。


三次顺序动态 **42509803932**：tree `9075794453893d6bb17a54303cf358fc01c8d8e1`（clean30335781），snapshot `3d5b3955c61cd83e99f23a06424d9b661aca0278`，control `9d915b7`；SUCCESS，三轮全部零错误：p7a 2.5261 / 2.7413 / 2.7497秒；p7i 19.6758 / 19.6386 / 19.7252秒。证据fresh-dynamic-repeat-42509803932。fixture新建Lab/居民，每轮同一VM，不能把重复当作独立机器统计；闲置已贴近20秒门槛，需相位/负载覆盖。


完整旧策略baseline42507224815：31项、8并行、1908.33秒，23 pass / 8 fail。

- 进入测量后断言红6项：p6r（p95 429.97ms，仍Direct306/320ms）、p1s（alpha3次/最多1次）、p5（150ms阶段beta打洞2次、600ms阶段alpha1次）、f4s（未回carried）、Plain relay NAT d2/d4未直连。
- a2与f4i未进入相应测量：配对 `document transfer state busy` 导致unwrap；这是准备阶段观察，不证明其NAT/路径行为失败。busy归因尚未确定，不能无证据称执行器故障或已修。
- p2i报告red（34.3秒）而Rust test ok，当前原场景仅观测；完整pass计数不表示它达标。
- p1/p1j/p4均600秒，p1移动alpha0/beta1，p1j0/0；relay b3是1200秒。b1/b2/b3/b4、c、uplink e与其他转发通过。
- 此完整matrix包含显式p1s CPU压力和其余并行场景，性能数字不与单线程repeat混用；它也没有后加p7及固定参照。候选完整42509652259尚在途，暂不归因回归。


quality输入一致性预期红 **42510628203**：tree `fa664f056b152393d7c5e8b42737aaffcf8fa86b`，snapshot `c2c4fbe831fab5a6eaae24816c2768879c667362`，control9d915b7。新增PathQuality数据与2项行为测试，helper仍读旧rtt；待结果。随后新增vendor缓存/API尚未包含在本run，也未验证。


正式3次对照已结束并全部下载：baseline42509612220每轮5pass/1fail，仅p6r旧策略红；candidate42509632238每轮5/1、4/2、5/1，p6全绿但p2两红（13.0/16.4秒）、p3两红（9.9/15.3秒、后者1超时）。见REPORT，不能把candidate工作流失败都归作预期旧策略红。

共享quality输入：42510628203已取得产品红63pass/2fail/1ignored，证据quality-input-red-42510628203。修复tree `fdc41531d7785fb22ce45b83ff0ef537ef52099b`：net-unit **42511021381 SUCCESS 66pass/1ignored**（snapshot97786ea0，shared-quality-unit-42511021381）；net-vendor **42511084014 SUCCESS noq431/iroh22**（snapshotad1f0fb5，shared-quality-vendor-42511084014），control9d915b7。包含PathMeasurement缓存/同副本age/count、PathFact接线和亚毫秒质量区分。

后续API短矩阵 **42511301780**：tree `f8eba3f5cb00780bc3b95d159fe5f421374ce409`，snapshot `083053a77e25ea76db83ef59218149bbccc09c29`，controld7d1af1；加入f4/f4s/f4i、2并行。它仍在ACK-delay校正前，待结果。

最新ACK-delay候选同tree `05a5c7575d72b524a081a28b48b62afc9c4824b1`、basecbcc8abe、controld7d1af1：vendor **42511866017**，snapshot `e3c172fa2112d40b87a9a343cee69e4e541bca2e`；短矩阵 **42511895143**，snapshot `6e5479ace12e63a8671c23a3fb2dd4ef8c2aac1d`。尚待验证。该候选保存与平滑器一致的校正样本，未改变raw latest/PTO/拥塞数学；不能凭代码推断已修p2/p3。


42511866017已SUCCESS：noq432、iroh22；证据ack-sample-vendor-42511866017。42511301780已FAILURE，8pass/1fail，证据shared-quality-smoke-42511301780：p2 2.6秒、p3 worst536.9ms（0错/退役）、f4/f4s与4项p6全绿；f4i first started正确，但恢复后2次selected relay、回hop58.6秒，违反无回退断言。

同速黑洞在飞数据预期红 **42512359677**：tree `d35655265579a8477072a5831c7135c0bd9f8da2`，basecba1ea80，snapshot `edd23810981b29bc57e292fbb40e03df19835cdd`，controld7d1af1。仅新增真实虚拟PTO/永久移除旧路的多路径流交付回归，未修改重发逻辑；待结果。


42512359677已产品红：432pass/1fail，same-speed blackhole test在旧路数据立即重排断言失败；证据blackhole-backup-red-42512359677。42513039282（tree1ad7b266、snapshot7c71583a）仅编译失败，因path_stats引用/借用；证据blackhole-backup-candidate-42513039282，已结构性修正。

重排修复候选：**42513546687 net-vendor SUCCESS**，tree `6465d10558e05566d7aa621fcc911e432ee8019f`，snapshot `516e70a650e05ed2b40a085d42bec7a3d774a448`，basecba1ea80；control以environment为准。真实网络动态 **42513580645** 同tree6465d105，snapshot `e67ddf7b3a6d72b2545d1a17be014c0d8d6ffea9`，controlc6a20c5，p2/p3/p7单线程各3轮，在途。

42511895143已FAILURE 8pass/1fail，仅f4i前置未达，证据ack-sample-smoke-42511895143。ACK校正p2 973ms，p3最坏696ms、零错/退役，f4/f4s与p6全绿；仅一次，不能替代正式3次。

42509652259完整旧采样candidate已FAILURE：35项21pass/14fail，1968.35秒；证据candidate-full-42509652259。此源早于共享API/ACK校正/失败Backup重排。保留原p1/p1j/p1s/f4/f4s/f3/p2/p5等回归，不把旧源问题静默删掉。


42513546687已下载blackhole-backup-green-42513546687，**433Noq/22iroh全绿**，environment证实control `c6a20c5a733a0778985c3960bc6490820da60d41`。同速黑洞回归现在不仅重排计量通过，流数据在健康路交付也通过。

带f4失败诊断的短矩阵 **42513843436**：tree `6e61d014c82c25b24f2bf4cfaab7a4e5f92043b3`，snapshot `66dac4d7271a6cf336492d141253b8be299a2a32`，basecba1ea80，controlc6a20c5。在途。生产代码与tree6465d105相同，仅追加f4失败时最后50个路由journal到artifact；不改变拓扑/门槛/工作负载。


当前源接线/第一方格式 **42514304126 net-unit**：tree `cb25e12597100ca70f27a1c0cdfd0d06a94c5347`，snapshot `bc56f16f9f7b481ff80387d2acb3a3ca038737ce`，basecba1ea80，controlc6a20c5。在途。生产代码与433/22绿源相同，包含f4诊断与最新文档；若格式失败，保留formatter patch并应用后再验证。


42514304126仅格式失败：新f4诊断的matches宏需多行，未执行单元，不算产品红。证据backup-source-unit-42514304126/format.patch已检查并应用，无本机formatter调用；待新源复验。


格式修正41c8af78后的 **42514729034 net-unit**：clean tree `7ba783704556f3893cfb1bd1dc9fb6d1fc3d4b74`，snapshot `955b3e90dbefc3a7f8b4b8ade950a1ca793b0ea1`，controlc6a20c5，在途。42513843436短矩阵已FAILURE，待下载/分类诊断，不把观察超时当终止。

## 最新终态与p2协议纠错

以上“在途/待结果”为当时记录；以下是终态。所有证据已下载，原失败不删除。

- **42514729034 SUCCESS 66pass/1ignored**；证据backup-source-unit-42514729034。只覆盖当时源，未覆盖后加p2 v2与f4_stall诊断。
- **42513580645 FAILURE**：三轮分别3pass/1fail、3pass/1fail、4pass。p3最坏530.5/732.0/740.9ms，均零错误/退役；p7 active2.4233/2.6044/2.4252秒、idle19.5757/19.5113/18.7489秒。旧p2前两轮AlphaRelay/BetaLAN后约390/410次成功回显仍未bothLAN，第三轮1.4秒bothLAN；原门未测每次回显分布，不能回填体验通过。证据backup-dynamic-old-protocol-42513580645。
- **42513843436 FAILURE 7pass/2fail**：f4绿（腿应答后0.2秒回carried，无回退）；f4i本次绿（22.4秒回m，无回退）；f4s红（23.0秒carried、应答后14.0秒、最长Relay17.8秒、slowest echo10.1秒）。p2另一红为LAN-only类型门；p3最坏563.7ms零错/退役，p6四项绿（p95 64.59/148.30/87.07/115.82ms）。证据backup-hop-diagnostic-42513843436；没有path-diagnostic行，因为本轮f4i通过而当时f4s尚未挂失败诊断。后来fixture补到f4_stall，不改拓扑/门槛。

p2纠错协议在86ce3330/680dee60、任何v2实跑前冻结：原Relay未加延迟，必须LAN与速度优先冲突；改验明确快集合LAN/Relay并加逐次真实回显门，10秒/零错误/零退役保留。Link旧平滑RTT无年龄，不拿它冒充新鲜值设门。旧LAN-only数据全部保留，p3真实卡顿继续有效。

相同v2 fixture三轮顺序对照（controlc6a20c5）：

| Run | 生产代码 / source tree | snapshot | 状态 |
| --- | --- | --- | --- |
| 42516040900 | baseline6485cb83 / 02851ae57ad3df1c0a4e67db171f8148152fd82e | 6fc450c403b4d4022172a19f5ff0ed695f63781c | FAILURE，三轮4/2、4/2、3/3；证据speed-v2-baseline-42516040900 |
| 42516056972 | candidate680dee60 / e0f8ac95368e27248abe9db96296f949f1d85cbd | 1c86ed462d76d423c63e640e9ac1913de48f213d | SUCCESS，三轮6/0、18项全绿；证据speed-v2-candidate-42516056972 |

两版paths.rs相同；baseline生产代码保持648不动。候选包含新增f4失败输出但compare不运行f4。候选p2 1.3/2.9/2.9秒、n35/41/41、p95 38.6–47.4ms/max62.4–90.1ms；p3最坏431.1/539.7/463.8ms，全部零错/退役。旧p2 3.1/3.1/26.6秒，末轮超限；p6r三轮慢Direct红，p95 330.4–333.0ms，新版113.4–128.9ms三绿。旧固定Relay参照p95 110.9–113.9ms但三轮期末held路径断言红，不能称整项绿；详细各项见REPORT。

源码持久化：43个实验快照已建本地不可变`refs/khor/net-evidence/<run>`，清单证据根source-refs.json。每个ref的`:source`子树校验匹配source_tree，避免七天artifact和未引用git对象过期后丢失脏树实验源。未推外部、未覆盖任何工作树；新run终态后仍须补环境/模式/结果证据。

## 诊断与真实样本时刻

- **42517477095** net-unit，tree50bb9361/snapshot55ffa0e6/controlc6a20c5，仅p2 fixture格式失败，未执行单元；patch已应用。证据speed-v2-unit-42517477095。
- **42517864599** net-node，clean221b98fc/tree87f4555d/snapshot50775a6e/control781a907，仅新journal兼容测试字符串格式失败，未执行单元；patch已应用。证据path-measurement-node-42517864599。
- **42518102550** net-vendor，treed62cbf30/snapshota9a761f4/control781a907，同一journal格式失败，未执行Noq/iroh单元，不能记预期新鲜性产品红；证据ack-time-format-42518102550。

221b98fc在既有path事件附缓存质量/age/count，不新增timer、不改UI显示句。control781a907将journal单元加入net-node，新增netlab-hop以原fixture单场景顺序三轮。

| Run | 任务 / source tree | snapshot | 状态 |
| --- | --- | --- | --- |
| 42517915206 | netlab-hop / 87f4555d149816be3fbc74801aac98a7357c74de | fd10c974e5e2c032f5cef62627675e747b1bb26b | SUCCESS，f4/f4s/f4i三轮9/9；hop-isolated-42517915206 |
| 42517959505 | full netlab / 87f4555d149816be3fbc74801aac98a7357c74de | 6c42bd86d9193953f9fde0821346e343e55067b6 | FAILURE，23pass/12fail，35项1864.48秒8并行；ack-backup-full-42517959505 |
| 42518379039 | net-vendor / 915fec4f91a79525b6d2fac33ee728048004be93 | d24a4cc9f51c7cb580b170bf2e2171c2801fbb80 | 产品红，Noq434绿、iroh22pass/3fail；ack-time-red-42518379039 |

后两类源不同：前两run生产代码仍为ACK校正+Backup修复、Reading用actor观测时刻。42518379039只向Noq暴露采样包发送/ACK处理时刻，保持原RTT/PTO/拥塞数学，新增Noq元数据一致性与iroh三项回归（无ACK初始估计变化、延迟读stats、积压旧包不能确认新往返）；尚未修改Reading消费逻辑。全部control781a907。恢复先取真实红，不能把格式错误或静态判断当作回归已经复现。

真实时间窗修复 **42519038936 SUCCESS**：tree `5b137c71dd44ea439111d41b724acc7fb4871d00`，snapshot `997e58aca525718f24617443b2fb109425c5760e`，control781a907，434Noq/26iroh全绿；证据ack-time-green-42519038936。保留3项旧源真实红，第四项是后加静默覆盖。实现4fa3f1b3，p7相位先7cfa922d冻结、后6a45e488补5个case，共7相位；不改10/20秒目标。

同clean6a45e488/tree `2c18140bb83f52ca7c6ff0508980977667b1dbfc`，control0e5788c（给net-node补核心khor-net单元）：

| Run | 任务 / snapshot | 结果 |
| --- | --- | --- |
| 42519750741 | net-node / 641d2e83ad9250e2a634a5bcca51c64b07ad0bbe | 格式/核心66pass1ignored/hop9/wires10绿，journal27pass1fail；新诊断Entry兼容产品红，ack-time-node-42519750741 |
| 42519778236 | netlab-dynamic / 64a72fa8fd51a021c2517c0b889e277bd64d174a | 在途，p2/p3+7个p7相位，单场景顺序三轮27项 |
| 42519803507 | netlab-smoke / 053f6d67243e454ee874cc1e35a71b8bda74c364 | SUCCESS 9/9，2场景并行380.40秒；ack-time-smoke-42519803507 |

最新短矩阵：p2快路1.0秒、p95 33.7ms/max45.7ms、零错/退役；p3最坏609ms零错/退役；p6d18.68/p6r106.05/ref-d29.22/ref-r106.27ms p95全绿；f4腿应答后0.9秒/f4s1.0秒回carried且无回退，f4i9.3秒回m。仅一次，不能替代本源码三轮或长稳定性。f4/f4s的3/8秒断腿期间没有快路，最长回显3.7/8.8秒是记录值，不套p3快备用目标。

42519750741失败原因：新What::Path嵌PathFact含u128，serde内部tag缓冲不支持直接deserialize_u128；已取真实产品红，不算格式失败。Netlab::LabNode::journal直接读原始JSON Value，所以既有实验路径日志仍完整，正式journal Entry回读另外需要修。6b3644f5保留算术类型，读取JSON的u64毫秒数字再扩展，并验证实际持久化Entry及显示句；不改网络选择。

首次该修正 **42520842668**：tree `a7b1a498994cb2cef7544f44ebf75f73b987ba8f`，snapshot `775b8e61ca09161bc24460b868ee55f56625c71e`，control0e5788c；仅新assert格式失败，未执行单元。证据journal-number-node-42520842668；patch与当前源反向校验通过，已按其格式落盘。

35项full42517959505终态分类：

- 准备未达4项：f4/f4s/f4i在180秒内未经m，p1j beta没到Direct，未进入相应测量。
- 原断言红8项：f3没声明no_relay；Plain NAT d2/d4未选Direct（尚不能推断直连路径未打开）；p1移动5/3、p1s7/9；p2快集合13.5秒、max424ms、零错误/退役；p5短卡顿切换/打洞；固定Direct参照p95 1.5628秒、max2.6313秒。
- p2i报告red15.7秒而Rust未assert，不能计达标。p3最坏656ms零错/退役、p6自动两向及Relay固定参照、p7两项均绿。
- 18行f4准备失败诊断仅x/y的Relay/拨号/settled，无hop/Carried记录，缺M腿和道路书事实，不足以定因。6b3644f5把三个f4失败出口都扩到x/y/m六方向；完整8并行压力不能与独立9/9转发混为一条性能结论。

当前clean6b3644f5/tree `75ca1f10d591b5cc29f0189775847461475a0a4c`，control0e5788c：

| Run | 任务 / snapshot | 状态 |
| --- | --- | --- |
| 42521515341 | net-node / 853f56ed862f13915c14b6f924ac7f14f8e1e95d | 在途；格式与核心/hop/wires/journal，含Entry数字修正 |
| 42521539154 | full netlab / fc1903c89d37f711a93eb52877f3f14bd58ab7be | 在途；40项8并行，真实时间窗+新增相位+六方向失败诊断 |

恢复时先取三项活动任务（另有42519778236），不因轮询超时重启。各run实际source/environment和原日志为准；不能把这次full35项或前一版短矩阵当作未来源码完成证明。


## 2026-11-09 窗口过期修复与独立稳定性

以上三项等待记录已结束：42519778236为26pass/1fail，p2末轮16.2秒超10秒，所有21个p7相位绿（交互2.26–2.60秒、闲置4.39–18.56秒），p3最坏703.7/573.3/458.6ms，零错/退役；证据ack-time-phases-42519778236。42521515341仅hop_diagnostics签名格式失败，未执行单元（journal-number-node-42521515341），其formatter patch已应用。42521539154为28pass/12fail，40项8并行2001秒（ack-time-full-42521539154）。

full40红项：f3没声明no_relay；f4在180秒内未通过m；f4s腿应答后0.4秒回carried、但30.5秒后又Relay且期末Relay；d2/d4未选Direct；p1移动5/11、p1j准备未Direct、p1s移动5/5；p3最坏1.1秒且零错/退役；p5切换/打洞断言；p6r p95 223.21ms；p7i20.0543秒。其余6个p7相位绿，p2本次绿。p2i报告red79.8秒但Rust未assert，不能计达标。完整交叉负载不能全归CPU，亦不能拿它代替独立稳定性。

控制均为fdd1b3f80de655e8430c9ee00362da1109f06fd2：

| Run | 任务 / source tree | snapshot | 结果 / 证据目录 |
| --- | --- | --- | --- |
| 42524575725 | net-node / c4c0324c69ca3d55c3d031d1b9cfb2f8ddc552da | 2a7cb33abfb4b4a016ff217d600b7f4514a5e811 | SUCCESS，核心66pass/1ignored、hop9、wires10、journal28，journal-number-node-42524575725 |
| 42524600966 | netlab-recovery / 同c4c0324c | cdabfe166a39f07958b22cbe71755f8c5c9d3b42 | SUCCESS 6/6，recovery-diagnostic-42524600966 |
| 42524625685 | netlab-stability / 同c4c0324c | 093b405cb04e0483ea818c6cfa4172b928e20515 | FAILURE 2pass/2fail、1880.44秒，stability-isolated-42524625685 |
| 42525350514 | net-vendor / 7af2b013aef23546977fc07e545b63827b0b4e16 | 851e0be25cc924c3677b312013ac270143c47b61 | 预期产品红：Noq434绿、iroh26pass/1fail，quality-expiry-red-42525350514 |
| 42526508917 | net-vendor / f182946f57f13110a16433e1a1bdaf093e3aef2c | d709594aa36daf3313f5d65bf0e2f3cf4e70de0d | SUCCESS Noq434/iroh27，quality-expiry-green-42526508917 |
| 42527295249 | netlab-dynamic / 6f9a77e399e25e1c3b57eb8d52136fcd04e3d92a | dbcf6d949d8de4be03ce247d1b2bc29e3c59de2c | 在途，过期修复后，p2/p3+七个p7相位3轮27项 |
| 42527316922 | netlab-smoke / 同6f9a77e3 | e600c180ba602727678ed55dd1bd554a341fe599 | 在途，过期修复后9项2场景并行 |

独立恢复p2收敛750.2/727.6/753.1ms，max32.65/31.53/31.88ms；p3最坏640.6/705.8/578.4ms，零错/退役。先前16.2秒失败保留，不以不同VM三绿宣布消除偶发。

独立稳定性单场景顺序、每项一次，保留p1s自身忙CPU与20次300ms停顿：p1空闲600秒两端移动0/0、零退役；p5在150/300/600ms各4次停顿，均零Relay切换/零打洞，最慢回显266.9/469.3/973.7ms。p1j准备失败（alpha未Direct，日志见Relay约23–25ms、Direct约21–22ms；尚未进入600秒测量），不能把它记为600秒抖动红。p1s确实测量红：600秒移动14/2、零退役、最慢回显658.8ms。说明抖动不能全部用full交叉负载解释。部分selected事件快照质量已接近/反转，不能当作底层决策瞬间输入；需确定性复现确认流程，先不盲调门槛。

窗口过期红：近2秒窗口只剩1ms样本，派生quality却仍300ms。4ec6ab2e在窗口缩小时复用record的中位数计算，空窗保留最后已知值；样本时间、ACK计数、确认记录不刷新，不新增timer/探测。红绿源仅后者修正派生值，实际网络因果仍待新两项结果。实际Entry持久化回读已由42524575725验证，兼容修复不再等待旧格式任务。

实验源现已锚定57个本地不可变refs，清单source-refs.json；已校验每个snapshot:source与source_tree一致。活动源已锚定但仍须补终态environment/mode/结果。


42527316922终态 **SUCCESS 9/9**，373.32秒、2场景并行；证据quality-expiry-smoke-42527316922，准确source/environment见该目录。p2 1.0秒、n34/post30、p95 36.70/max40.55ms，零错/退役；p3最坏642.6ms，零错/退役；f4/f4s腿应答后0.3/0.4秒回carried且无回退，f4i19.6秒回m；p6d23.57/p6r123.02/ref-d24.11/ref-r106.38ms p95。早于下一项确认候选，不替代防抖验证。

新增确认回归取红 **42528150054 net-vendor**：base645363cd、treeb82736a93788817c0f8a2d8397ae4d5c6d352ebc、snapshotd760ea482545bf86350ebd555722dbe14c532ada、controlfdd1b3f；在途。仅新增两条行为回归，生产逻辑仍为4ec：决定前两个包不能触发快速迁移，等待快候选时应同时探测当前路。拟最小候选与恢复边界见NEXT-DESIGN，尚未修改生产逻辑。


42528150054终态预期产品红（decision-proof-red-42528150054）：Noq434全绿，iroh27pass/2fail，恰为新两条回归；一条旧值Go，另一条旧Hold不探测from。不是编译/格式红。候选只改确认取证起点与等待时双路复测，快分支仍不强制等慢当前路，不改黑洞leave/收益/固定确认时长。

确认候选source tree `86619b975d12358a8ee99aefee4407857efdbe6b`，base645363cd dirty，controlfdd1b3f：

| Run | 任务 | snapshot | 状态 |
| --- | --- | --- | --- |
| 42528714010 | net-vendor | b82b415a8bb99fcc905cc1799141c7d7f9de0677 | 在途，434Noq/29iroh目标 |
| 42528746575 | netlab-stability | bf2c40d5cd2c5fff34e30bc81038156300cc2cc5 | 在途，单场景长稳4项，原准备/忙CPU保留 |
| 42528783380 | netlab-hop | 97263e13d067b978bb73b8be3aa2b7921151140e | 在途，转发3场景×3轮 |
| 42528841695 | netlab-dynamic | 05d5a15898b11633f6356cd60d2761ac6f813137 | 在途，p2/p3+7相位×3轮 |

全部source已锚定；清单现62个refs，不依赖snapshot远端分支或artifact保留。确认期间可能比旧快分支多PING，受原4秒确认/30秒冷却边界约束，尚无后台开销结论。


42528714010终态 **SUCCESS 434Noq/29iroh**，证据decision-proof-green-42528714010。新增两回归先红后绿；原快路不等慢路/原黑洞离开测试仍绿。源码已提交6eabe635；提交前核当前vendor/Noq/策略文件与绿snapshot:source无diff。新确认可能多一个取证阶段，实际p1s/f4/p7目标与探测开销仍须网络证据；余三项新源任务和42527295249旧确认/过期修复相位任务均未结束。


## 过期修复后的正式三轮：黑洞仍有真实长卡顿

42527295249已 **FAILURE 25pass/2fail**，三轮9/0、8/1、8/1（362.79/371.43/367.23秒），证据quality-expiry-phases-42527295249，tree6f9a77e399e25e1c3b57eb8d52136fcd04e3d92a、snapshotdbcf6d949d8de4be03ce247d1b2bc29e3c59de2c、controlfdd1b3f。单场景顺序、三轮27项。

- p2收敛1.5/4.0/1.1秒，max58.15/62.52/50.23ms，零错/退役；全部通过。
- p7共21项全绿，交互2.26–3.10秒、闲置2.51–18.41秒；闲置phase0为18.356/18.408/18.358秒，仍有20秒裕量问题。
- p3最坏612.4ms/1.3秒/15.3秒。末轮还有10.3秒请求，1次15秒超时，零退役；alpha Relay@1s、beta Second@1s。既已选备用仍卡顿，不能只用路径类型收敛证明请求恢复，不能当执行器问题删除。

在此源之后的6eabe635只修快速确认，三个实际网络任务仍在途，不能借其未出结果覆盖旧失败。当前新增 **42529470019 net-vendor**：basee45cac03、tree65bdf14629fee79159010ae273c7582179c53d58、snapshot50542c58de1cdbb5f4e9d8758c090f8280071895、controlfdd1b3f。仅复用原真实虚拟计时黑洞fixture增加“先改status、后达到不应答”的回归，保留原先故障后移路及健康同速覆盖；未改生产Noq。预期检验只在status改变时检查是否漏掉后续PTO达到的不应答，尚未取得结果，不预认定为这次netlab的全部原因。


42529470019终态预期产品红（late-backup-red-42529470019）：**434pass/1fail**，仅新增先移路用例失败在“a failed backup must release data even if the move came first”。没有编译/格式错误；Noq失败后脚本未运行iroh，不能记iroh新红或新绿。

候选保持原判据，只在Backup既有LossDetection事件重新置位检查，由poll_transmit统一重排；不引入独立timer/逐次发送扫描。回归额外检查<300ms不重排，并在相同虚拟时序验证<1秒释放及健康路完整流交付。p3仅在失败后补两端路径事件，不加测量请求。

以下同tree `6cbec16a56889023db95e92f6a4dc657aa6a6805`、basee45cac03 dirty、controlfdd1b3f：

| Run | 任务 | snapshot | 状态 |
| --- | --- | --- | --- |
| 42529931673 | net-vendor | fb8718047f9584b6dcd3f239cea7ce1aef5bca02 | 在途，目标435/29 |
| 42530014989 | netlab-recovery | 544da02c21aff9aaad98a94bcdc3b0fdbbb72e2a | 在途，p2/p3三轮6项 |
| 42530060891 | netlab-smoke | 39c8968ba8ccc7e49c6df6e5e2d31fdedf0b53ca | 在途，9项2场景并行 |

6eabe635三项旧Noq网络任务仍独立在途，不把它们当作这份新Noq代码验证；它们用于分离确认修正对稳定性/转发/相位的影响。


42529931673终态 **SUCCESS 435Noq/29iroh**，证据late-backup-green-42529931673；Noq新增后移路故障用例先红后绿，另外<300ms不重排、<1秒原PTO释放及完整流交付均通过，既有434与iroh29全绿。代码已提交ff399658，提交前核全部vendor/策略/p3文件与绿snapshot:source一致。当前66份实验源ref已验证，终态收据已刷新，余5份网络任务仍在途；不能用单元代替实际p3/f4。


## 2026-11-09 确认版终态与准备v2

下列前3项同tree86619b975d12358a8ee99aefee4407857efdbe6b、controlfdd1b3f，早于ff399658：

- **42528841695 SUCCESS 27/27**，decision-proof-phases-42528841695。p2 976.7/752.8ms/3.0秒，max32.29/32.11/31.55ms；p3 461.2/613.8/493.3ms，零错/退役。21个p7全绿，交互2.38–2.79秒、闲置5.62–19.67秒；phase0为19.6105/19.5826/19.6735秒，裕量小。每轮359.64/355.58/359.18秒。
- **42528746575 FAILURE 1pass/3fail**，1311.33秒，decision-proof-stability-42528746575。p1空闲600秒0/0；p1j、p1s均Direct准备失败，未测各自600秒。p5零Relay切换，150ms组打洞0/1、300ms组2/3触发原短停顿红；600ms观察组打洞2/2、最慢回显1.1秒。不能称p1s防抖已经改善。
- **42528783380 FAILURE 8/9**，decision-proof-hop-42528783380。三轮3/0、3/0、2/1，554.53/485.29/514.06秒。f4应答后1.0/0.6/2.2秒，末轮超1.5秒；均无回退。f4s 0.5/1.0/0.8秒全绿，f4i22.7/29.8/18.8秒全绿。

以下ff399658源码tree6cbec16a/controlfdd：

- **42530014989 SUCCESS 6/6**，late-backup-recovery-42530014989。p2 2.9/3.1/1.3秒，max36.43/34.95/36.66ms；p3 591.9/428.7/602.2ms，零错/退役；每轮172.20/167.52/165.26秒。
- **42530060891 FAILURE 8/9**，305.58秒2并行，late-backup-smoke-42530060891。f4i准备阶段LabNode.with的Node::open_as报document transfer state busy，并未进入网络恢复测量。p2/p3、p6四项、f4/f4s绿；p3 594.5ms零错/退役，p6d35.16/p6r112.88/ref-d25.24/ref-r110.97ms p95。f4应答后0.2秒；f4s selected记录早于最后一腿heard，饱和差值0而非证明真实恢复耗时为零，均无回退。独立状态门测试明确Busy是实际非阻塞保护；当前不改生产存储，也不删除失败。

稳定性准备v2先309c1bbc冻结、后f8292b59实现。设备Relay100ms往返只用于one_speed准备，清除后再原30秒settle/600秒测量；p1s忙CPU/20次300ms、原最多一次移动/零退役均保留。只复制paths.rs/hop.rs/dev.py/netlab.sh至baseline；已逐文件相同，并核其全部diff只在这四文件，baseline生产仍648。

| Run | 任务 / tree | snapshot | 状态 |
| --- | --- | --- | --- |
| 42530871452 | net-unit / e4912fda2dcb2caf8aed2324495d48c8e1cff424 | b372912e515416278e81fe4b97e9a250b6e6d3e5 | SUCCESS 66pass/1ignored，stability-bootstrap-unit-42530871452 |
| 42531361022 | candidate netlab-stability / 同e4912fda | 591b73995f7e88fd6f629bedde75e99a046a1b46 | 在途，ff生产+准备v2 |
| 42531408799 | baseline netlab-stability / 0e9bf325d47377ac4c7b09b8d8ef68cd53b2e62c | 1b3f7a1bbadee364f746f3c9e8df449509e6f87d | 在途，648生产+相同fixture |
| 42531477192 | netlab-hop / 同e4912fda | a432378e2dbd779482bdffad75e5851a7c51b002 | 在途，新Noq版三轮9项 |
| 42531529275 | net-vendor / 80b4f2a6f891a9f9d96874ed206007cb8149523f | a17654f76db3e8eb80bdb616d31a8918d15a47c9 | 在途，Watch单次慢质量不能立即打洞的回归取红，生产未改 |

均controlfdd1b3f。Watch当前在首次慢quality直接打洞，未等既有2秒探测窗；新回归仅提出行为要求，未将静态分析记为复现成功。下一候选须保留真实故障1秒通道/30秒节流、要求决定后新往返，避免仅延长固定等待而让旧样本/CPU暂停通过。


42531529275终态预期产品红：Noq435绿，iroh29pass/1fail，单次150ms质量同时probe/punch为true；证据probe-before-punch-red-42531529275。

Watch候选 **42532303734 SUCCESS 435Noq/33iroh**：basef8292b59 dirty、tree63ddfeff21b634bbb79e0cf104f07a6117ca8b6e、snapshot5da5fad0b37e8a93d3c34f26e7f0e64ace4ab0ab、controlfdd，证据probe-before-punch-green-42532303734。代码519ad4aa；提交前核vendor/策略/fixture与绿快照一致。普通质量持续原2秒+两个开始后包的独立观测才打洞；原探测预算不变，前台样本仍可确认。极慢RTT超2秒不等放不进预算的往返；原故障1秒/打洞30秒节流保留。3个附加覆盖为新绿，不能声称都已旧源取红。

control **6af0ac328457a8c3aea9157e8b2ffcd65ae7e5c2**已push feat/net，新增netlab-stalls（原p1s/p5完整时长各1次/1线程），不替代完整A/B/full。产品9项、control准备1项Python及network.sh bash-n通过。当前3个Watch网络run同tree `5f40be2e96dae66a8d57cb82afe1e447014c194b`、base519ad4aa dirty（后续接口/文档变化），control6af：

| Run | 任务 | snapshot | 状态 |
| --- | --- | --- | --- |
| 42532770351 | netlab-stalls | b23eef27de00b6214e09e82416f764f108044a6f | 在途；实际prepare成功、Linux network运行、Mac跳过 |
| 42532849927 | netlab-dynamic | dc80527796731ec472955f5c593b5069f2e82d59 | 在途，27项三轮 |
| 42532877270 | netlab-hop | 43c7f6ee25e5dd488c1d3fdbb262dfbb06a0c711 | 在途，9项三轮 |

源码ref清单75份已验证，所有终态收据已刷新；其余3个旧Watch（新Noq）网络任务继续独立保留，不互相替代源码结论。

## 2026-11-09 六项网络任务全部终态

上一段“在途”现已结束，收据/source.git/原日志全部下载。75份immutable源码ref逐个校验，source-refs.json无pending。source tree/snapshot沿用前表：31361022/31477192为e4912fda、31408799为0e9bf325、其余三项为5f40be2e；前3项controlfdd，后3项control6af。没有以当前HEAD替换实验源。

- **42531361022 FAILURE 2pass/2fail，2451.63秒**，bootstrap-v2-stability-42531361022。ff生产、准备v2；p1/p1j各600秒0/0移动，alpha打洞1/12轮。p1s原600秒/20次300ms/忙CPU，移动7/0、零退役，最慢734.6ms，13轮打洞、4833包958738字节，有3次切Relay。p5 150ms打洞3/1、300ms0/3、600ms观察组1/2，零Relay切换，最慢243.9/497.2ms/1.0秒。原短停顿门红，早于Watch修正。
- **42531408799 FAILURE 3pass/1fail，2451.42秒**，baseline-v2-stability-42531408799。生产648、同4文件fixture/脚本；p1/p1j移动0/0，打洞0/10轮。p1s移动7/0、零退役，最慢802.1ms，2轮打洞、2991包436429字节。p5三组零Relay切换/打洞，最慢244.4/474.9/934.4ms。两VM仅一次，不能声称候选稳定性/空闲成本全面改善。
- **42531477192 FAILURE 8/9**，late-backup-hop-42531477192。三轮3/0、2/1、3/0，509.42/523.97/467.02秒。f4腿应答后0.8/0.1/0.2秒；f4i43.7/36.8/17.1秒；f4s0.5/1.7/0.1秒，中轮超1.5秒。均无Relay回退，不能用8绿删1红。
- **42532770351 FAILURE 1pass/1fail，1120.40秒**，probe-before-punch-stalls-42532770351。Watch版p1s原600秒/20次300ms/忙CPU，alpha移动3次（lan@31s、second@37s、lan@458s），beta1次（lan@48s）；零退役/打洞，最慢671.8ms，2609包323136字节。p5 150/300ms零Relay切换/打洞，最慢254.7/481.9ms；600ms观察组打洞0/1、最慢981.5ms，原门不扩展。不同源/VM/任务组合，不能从7或14降至3计算因果改善比例。
- **42532849927 SUCCESS 27/27**，probe-before-punch-phases-42532849927。三轮均9/0，363.89/368.46/372.58秒。p2 1.1秒/831.8ms/6.2秒，max50.38/52.60/75.08ms，零错/退役。p3最坏626.1/655.5/508.1ms，零错/退役。21个p7相位绿，交互2.29–2.62秒，闲置2.52–19.53秒；phase0为18.4154/19.5320/19.2032秒，20秒裕量仍小。
- **42532877270 FAILURE 8/9**，probe-before-punch-hop-42532877270。三轮3/0、2/1、3/0，477.90/467.09/461.77秒。f4腿应答后0.8/0.0/0.3秒；f4i34.3/15.1/12.6秒；f4s0.0/**1513ms**/0.2秒，原1500ms门红。日志一位小数显示1.5秒但assert原值1513ms，不可四舍五入为通过。均无回退；饱和0只说明selected记录不晚于最后腿heard记录。

目前无活动run。下一步可选诊断构建保留原场景与门槛，记录实际决策输入和ACK校正分量；日志改变开销，不得当作普通SLO成绩。p1s剩余移动、f4s超限尚未定因，不先放宽门槛或按缓存快照调策略。N1仍红，N2–N5完整范围不变。

随后调度诊断候选（只加观察，不改策略）：basedf5b0864 dirty、tree1113605daf5597c2663d5e6468c45dade34031c9、controlc3b117d。42536677230 netlab-diagnostic，snapshot72b0f7e13c0cb7142db033434cbd8182a1ba610a；42536711138 net-vendor，snapshot71a5b250ed41979a7e8d66d01991b06254b813d3。前者原p1s/f4s一次1线程，显式诊断模式，保存原serve.log而非尾部采样；不算普通SLO。后者根格式/Noq/Iroh回归。CLI用可选feature与环境开关启用已锁0.3.23 subscriber，默认构建不启用。9项产品Python、1项control准备及两脚本语法/diff检查通过，Rust待远端结果。

42536711138终态 **SUCCESS：根格式、435Noq、33iroh**，decision-diagnostics-unit-42536711138。已核5f69db3f与快照全部vendor/CLI/脚本无diff；未以此声明可选CLI feature已编译（诊断run仍在途）。77份源码ref，当前只有诊断pending。覆盖审计发现33过滤器遗漏net_report/socket的khor_回归，control322835b已加入额外`--lib khor_`与单独日志；仅控制分支push，追加覆盖尚未执行，不回填本次成绩。

追加覆盖 **42537392595 net-vendor** 已调度：based5e926bf clean、tree d7f8f38cdeaabefc398f5580a747c7e599209c49、snapshot a256f6708e9ad3920c1d01561b785e6e1425d1bd、control322835b。生产代码与5f69db3f相同，只增文档/控制执行覆盖。包含原435/33及khor_选家/QAD/网桥专项，结果待取；不当作重复旧33就证明遗漏覆盖。78份源码锚点，当前两项pending。

42537392595终态 **SUCCESS：根格式、435Noq、33路径、9选家/QAD/网桥**，vendor-home-coverage-42537392595已下载。9项逐个为fallback tier、家驻留/对等历史样本/hinton轨迹、接班频繁断开冷却、HTTPS可达、QAD复用/失败单族探测、网桥过滤；全部ok。不是iroh158项全量，更不替代对应故障域/真实设备集成。当前只42536677230诊断在途。此前STATE原文归档HISTORY-20261109-before-diagnostic-results.md，恢复以精简当前STATE为准，不按历史在途表重复调度。

42536677230终态 **FAILURE 0pass/2fail、714.13秒**，decision-diagnostics-42536677230。可选CLI编译成功，subscriber有效。f4s在准备观察LabNode.with/Node::open_as报document transfer state busy，未进入恢复测量；p1s原600秒/20次300ms/忙CPU，alpha移动2次（lan@244s、second@275s），beta0，零打洞/退役，最慢1.1秒、3187包343563字节。两份原serve.log共399888字节，另存analysis-decisions.json；日志模式不是SLO，不与普通3/1数字比较因果改善。

两次Go均failed=true。第一Go monotonic567.1171、since566.6073，当前Second仍有quality3.807ms/1近期观测，候选LAN2.576ms/1；Silence began566.2036、judged567.1169、answered None。原路在567.3554记answered（Go后238ms），568.6044才解除hold、quick=false。第二Go598.1150、since同刻，当前LAN最后ACK590.8951/窗口0，候选Second597.6363/1；Silence began597.2961、judged598.1148。由此定位实际快速故障通道，未证明全部来自ACK校正/普通quality；copy级PTO与last_received尚待补诊断。

下一候选先冻结observer client v1并在lab.rs复用同设备runtime的Arc<Node>观察客户端，避免每次观测反复本地初始化争用；生产状态保护/网络策略均不改。追加静默copy的PTO/等待/最近收包和道路聚合heard/wake_allowed日志，明确旧wake只在off_direct启用的历史原因（旧EWMA下曾放大p1s），不盲目删除。新增netlab-hop-diagnostic原f4s一次1线程，完整诊断模式仍保留p1s/f4s。未来A/B需匹配新的lab.rs，旧对照不回填。

observer v1实际调度同treebe7b5abef3cc3c1ef81744075f4436efdf668fd0、base82e0d3b5 dirty、controlc68b644：42538717126 netlab-hop-diagnostic / snapshotce0af01606b4368ff6c3a190d59afbb14a58ecba；42538759858 netlab-diagnostic / snapshot5bd8418703baccc5a71979a984e19b861447c6f8；42538808023 net-unit / snapshote62f6126eb1c9dbb2e17a3c43a33b71dedc64fa3。均在途，不是策略修复成绩。源锚点81份，产品9项Python/control1项准备/sh-n/bash-n通过，Rust/实际Arc观察客户端待远端。

42538808023终态 **SUCCESS 根格式、66pass/1ignored**，observer-client-unit-42538808023已下载。它不编译CLI netlab，Arc观察客户端实际启动仍待两诊断run，不用核心绿替代。

补Noq4专项 **42539256792 net-vendor** 在途：baseff5b4d36 dirty、treec0d5c1b516b5c6272f17f83e9bb6bbe597813bcd、snapshot09de7c0a3f15e534ccd06b298702290bf3ffcb19、controlc68b644；目标436/33/9。只新增a_backup_probe_is_acked_when_the_responders_data_road_is_blackholed：已验证Backup仍能回PING ACK，即使响应方的主数据路不可达。未改生产传输逻辑，也未宣称已对旧源取红或完成单向故障/ACK预算。82源ref。

42539256792终态 **SUCCESS 436Noq/33路径/9选家、根格式**，backup-ack-isolation-42539256792；新增备用ACK独立返回用例实际ok。其余Noq435及Iroh原覆盖均绿；没有旧源取红的宣称，单向故障/包数成本仍待覆盖。

42538717126终态 **SUCCESS 1/1、133.81秒**，observer-hop-diagnostics-42538717126。observer v1已成功初始化/配对与完整f4s测量，三节点trace245896+273640+176815=696351字节。Y Relay@5.0、carried@8.7秒，X Relay@8.2、carried@9.7秒，无回退、最慢9.7秒；末一腿Link heard14.9秒晚于选择，饱和0不证明物理恢复零耗时。日志模式非SLO，不用一次绿删旧1513ms红。

实际恢复输入：X Custom在monotonic324.7560记answered，quick=false等到325.7569解除hold再Go；Y Custom在324.7526记answered，quick=true立即解除、324.7530 Go。首次可用性仍依赖旧quality/类型的quick判据，造成一层额外1秒等待；不能单凭此断言所有f4红唯一根因。

新增确定性行为回归取红 **42539860644 net-vendor**：base1a084e79 dirty、tree06a93054b7a08919537f8ad8dd7e5a5907d59fc6、snapshot48ff937b1c7521fa2db4f71f1078bca0001223a7、controlc68b644。新用例首次决定后往返恢复可用性不能依赖quality；原hold测试先插近期恢复标记，仍验证重复故障1秒/REHOLD。只改测试，生产策略未改。目标验证436Noq绿/iroh33pass1fail，未出结果不预认定。83 refs。

42539860644终态预期产品红：**436Noq绿/iroh33pass1fail**，仅a_first_post_judgment_round_trip_restores_availability_without_a_quality_gate失败；availability-proof-red-42539860644。没有编译/格式错误，后续home过滤器未运行，不记为新红或新绿。

observer v1完整诊断 **42538759858 FAILURE 1pass/1fail、808.44秒**，observer-full-diagnostics-42538759858。f4s Y@9.6秒回carried，腿Link heard8.7秒、差0.9秒；X@15.1秒，最慢8.6秒、无回退。p1s alpha5次（Relay@31s、Second@93s、Relay@397s、Second@488s、Relay@610s），beta0；零退役、4轮打洞、最慢703.8ms、3619包521832字节。原20次停顿/名义600秒参数不变，末条610秒含原循环调度漂移；不以日志模式与普通模式计算性能改善。

p1s五个Go有4次failed=true。每次故障Go时，聚合heard在同一路上的另一副本都晚于坏副本最老unacked：454.9018>454.4014、820.8316>820.4875、912.0995>911.7594、1033.9612>1033.6502（monotonic秒）。前三/末次Direct门停wake，Relay一次允许wake仍在同tick Go；第一故障Hold只探测候选，另三故障用决定前备用样本直接Go。由此下一回归应验证故障确认的决定后样本与当前故障copy探测，而非盲调quality。原日志每副本PTO/last_received/unacked和heard完整保存。

35候选 **42540515220 SUCCESS 436Noq/34路径/9选家、根格式**，availability-proof-green-42540515220；同源普通模式 **42540582927 netlab-stalls在途**。
两者basec43370ff dirty、tree44fda196d1a9800c7e87e6f9175c3484661d5e51、controlc68b644；snapshot分别1f1cc72dcf35b79f43f96dceb34770455153b967与fd0c22ffcd844d33271d337ca879961a02e2387b。
代码只让首次新往返恢复可用性独立于quality/type，重复故障1秒/REHOLD、旧包和closed保护保留，删quick回调/两临时Vec、无额外探测/timer；copy-waking未变。
NAT helper仅失败后输出两端既有path诊断，门和注入不变。提交前策略/fixture与绿快照逐文件无diff。85 refs，当前只有stalls pending。

35已提交66e37760。新增故障确认回归取红 **42541459177 net-vendor**：base66e37760 dirty、tree32f638229d81beea7cd0dc40ccd16d67cb35282a、snapshot00e5b0a17f4364e83ba3a0c42d6a5a3361db7a0c、controlc68b644。
只新增a_fault_move_probes_the_current_road_and_requires_a_post_decision_backup_sample，备用包在故障后但决定前发出时，要求Hold并探测当前路/候选。
生产35未改；预期436Noq绿/34路径pass1fail，未出结果不预认定。拟修leave的决定后证据与当前最高PTO副本探测，仍需p3<1秒/防抖/转发共同复验，不引入应用请求重放。86 refs。

42541459177终态预期产品红：**436Noq绿/iroh34pass1fail**，仅新fault_move回归失败，实际Go、期望Hold {from:true,to:true}；fault-confirmation-red-42541459177。原4秒兜底等既有测试通过，home过滤器因路径红未执行，不回填成绩。

36候选已写未验证：leave用max(since,began)后的备用包，候选探测tick也夹在此后；Hold双向PING。当前路选择最高PTO live copy，候选最低PTO copy；可选日志记实际conn/path/pto/result。原4秒上限不延长、正常收益/确认/routine wake规则不变，没有应用请求重放。旧早期fault测试按新协议要求先探测，再用决定后答复Go；额外检查原normal move更早时不能用故障前样本。

同treea7f419472a9189c691a773535c70d223c2921ff9、base6c62a1bf dirty、controlc68b644两项在途：**42542123882 net-vendor**，snapshot2de64d8fa80f2fa067e82eddba2e2dfd7514ba76（目标436/35/9）；**42542228149 netlab-diagnostic**，snapshot44cde49b6afb8e561a5e8664253aa7ddc93d218e（原f4s/p1s完整时长、日志非SLO）。产品9项Python/diff检查通过。增加当前PING的代价与p3<1秒/稳定性/转发必须一起复验，不能靠单元完成N1。88源ref，另35普通stalls继续独立在途。

42542123882终态 **SUCCESS 根格式、436Noq/35路径/9选家**，fault-confirmation-green-42542123882。新fault回归34/1红→35绿；另在同用例补normal move更早时的故障前样本保护，只记为额外绿覆盖。提交前36策略文件与绿快照无diff；实际普通p3/防抖/转发与日志诊断仍待，不能以单元称5次抖动或<1秒恢复已收口。

35普通42540582927终态 **FAILURE 1pass/1fail、1119.21秒**，availability-stalls-42540582927。p1s alpha2（lan@213s、second@305s）、beta2（relay@316s、lan@418s）、零退役，最慢763.8ms，alpha零打洞、3027包380113字节；p5 150/300ms均零Relay切换/打洞，最慢231.7/450.2ms；600ms观察组打洞1/0、最慢1.1秒，原门不扩展。35只修首次availability，不能称解决故障确认；旧红保留。

36诊断42542228149终态 **SUCCESS 2/2、812.64秒**，suspect-probe-diagnostics-42542228149。p1s原20次300ms/忙CPU，alpha/beta0/0移动、零退役/打洞，最慢600.2ms，2629包276760字节；alpha44、beta24次confirmation probe记录，实际当前PTO3副本ping结果Ok且无Go。f4s Relay@4.9、carried@8.6秒，腿heard8.3秒、差0.3秒，无回退，最慢9.9秒含8秒断腿。日志非SLO，一次0/0不删旧失败，也不与普通模式算因果改善。

36已提交a087e1c5。以下普通三项同basea087e1c5 **clean**、treec807d328e78bdfda1dbebd7bdd4c14b0eaa8cd25、controlc68b644：42542824071 netlab-stalls / snapshot969f8e52fa52fe7b5043ff89f333bfc368c5724a；42542984111 netlab-dynamic / snapshot98d696e58102978d44fe15cfb1be0a439473fc30；42543254686 netlab-hop / snapshotbfd551912eebd65398b69d53467ca7366cf511f0。原时长/负载/门槛，日志关闭，均在途。

补Noq6专项 **42543928044 net-vendor** 在途：basea087e1c5 dirty、tree87bd60cccdce5eeb412774b9409f9a256dd722fe、snapshote4be1454ca3338aab9abc723a0218a2fa6063ad5、controlc68b644。只加a_path_keepalive_is_not_shortened_by_the_connection_timer，虚拟时间验证连接5秒不抢路径15秒计时器、路径自身仍保活、没有路径计时器的对照仍保活；禁用MTUD排除探测PING。目标437/35/9，无生产修改，不预认定通过或旧源红。92 refs。


36普通终态（同treec807d328、controlc68）：42542824071 **SUCCESS2/2**，p1s0/0移动、零退役/打洞，最慢865.4ms，3019包312293字节，p5原150/300门绿；suspect-probe-stalls-42542824071。
42542984111 **FAILURE25/27**，p3三轮1.1秒/1.2秒/818.4ms、零错误/退役；p2 2.8/3.0/4.8秒，max60.6551/46.0140/44.5722ms；21相位绿，idle最慢19.60910888秒；suspect-probe-phases-42542984111。不放宽原1秒门。
42543254686 **FAILURE8/9**，三轮2/1、3/0、3/0，用时479.01/483.09/461.18秒；f4s应答后0.7/0.4/0.6秒，但首轮回carried10.9秒后又退Relay，28.8秒才再回，最终Link仍Relay；其余无回退。f4三轮3.2/3.2/3.1秒最慢键击包含原3秒断腿，未离carried；f4i25.6/21.4/17.6秒绿。suspect-probe-hop-42543254686。物理恢复标记与选路标记不同，不以饱和0证明零恢复耗时。

42543928044终态 **SUCCESS 根格式、437Noq/35路径/9选家**；path-keepalive-coverage-42543928044。Noq6计时专项实际ok，无生产改动、没有旧源红宣称、不能替代当前闲置/手机成本。

新37边界用例提交 **42545673450 net-vendor**：basea087 dirty、tree7b61b7edc12ed5ef384b7054240e7b5202f1638b、snapshota21c1a978549979a8956639d35fdbd722cc6c8df；confirm_move传live当前copy收包提示，但leave明确忽略，保留36生产行为。新增无接收copy时接受故障后备用样本测试预期红，**未实际取红**。
基线匹配observer v1六fixture提交 **42545712302 netlab-stalls**：base6485 dirty、treebaca3064752631153c35547440da2114a36ca855、snapshot4565266b2619de01b16cbd62f88240d0551bf65f；生产基线原样。两项均controlc68b644fdd9494fec001132c40961ab634c423a2。
**两项在prepare开始前被GitHub账单/额度拒绝，steps为空、runner_id=0、network skipped，无测试或远端source artifact。** 不记产品失败、行为红或通过。本机回执、GitHub run/jobs/annotation、未执行分类分别保存在current-copy-hint-red-42545673450、baseline-observer-stalls-42545712302。
源ref增至94，无pending；拒绝快照remote_source_verified=false。异步询问用户恢复额度或已有Linux通道，未改账单、未采购、未改凭据/发行边界，既有Mac仍依赖托管prepare。


源码检查点33202c22已提交Noq6实测覆盖、未执行37边界用例及本批普通结果/账单分类。本轮没有实施37条件分支。
N2–N4只读复核已记NEXT-DESIGN，八文件blob收据design-source-audit-20261109/inputs.json；无新网络/性能执行。
94个source ref全部逐一验证REF:source等于tree；Noq6代码与42543928044绿快照一致，37边界源码与42545673450未执行快照一致，baseline/候选六fixture逐字一致。
本机test_remote_development.py九项通过，diff-check通过；先前错误指定test_dev.py只得到0 tests，不能算验证，之后已使用真实入口。没有运行本机Rust/Node/Xcode。


账单阻碍接续第2turn：复核42545673450/42545712302仍prepare前拒绝、无新任务，未重发。N5固定1.4.0 Noq1/2/5调用链审计与包checksum/源hash已保存upstream-20261109/noq-1-2-5-call-chains/；无上游执行或依赖升级。
新增Noq2 address_discovery_reporting_waits_for_one_rtt_on_resumption，维持reporting启用，覆盖发0-RTT时零地址报告与后续1-RTT报告/字节交付。尚未远端编译/执行，不增已通过计数、不宣称旧源红；生产无改动。旧accepted的禁用角色配置仍原样保留。


42545673450同源attempt2重新验证执行通道，prepare128936767186再次账单/额度拒绝（runner_id=0、零steps），network skipped；无编译/测试，不含本轮新增Noq2回归。完整attempt2原始状态/annotation和分类已存current-copy-hint-red-42545673450/attempt-2，原attempt1证据不覆盖。没有新source ref或在途任务。
N3底层congestion采样v2在改记录前冻结到ACCEPTANCE：保留每个echo/typing/screen的结果，成功/失败/超时与成功分布分开，默认几何/seed/工作量不变，实际Node/能耗与SLO仍独立。未改benchmark实现或运行该测量。


N3记录候选f17b8a31在f0be88d4协议冻结后实现；完整记录echo/typing/screen与bulk结果，失败/超时不筛走、空分布null。
control 6447289f4f9260df4f5887adc7c1e2f2270bbcce已push并核远端SHA一致，net-congestion三轮Noq模拟，未调度。
本机driver桩测试1项证明首轮失败后另两轮继续/保留且最终exit8；prepare1项、产品remote_development9项及bash-n/diff-check通过。
没有执行Cargo/Rust，格式/编译、Noq2新回归及真实拥塞数字仍缺；没有source run ref或SLO通过。baseline同步dev.py选项后仍只有六fixture修改，生产原样。


接续第3turn验证备用执行通道：SSH macserver经原iroh-ssh路由banner超时（exit255），未修改配置/信任/会话。既有自托管smoke42549824680 SUCCESS，控制6447289，Mac工具链/guard+C编译探针，非产品网络测试；mac-worker-smoke-42549824680已保存。
Mac便携分流control6b1616c8b816bdb72a8a031d0ff28cc46a3fb4c7在feat/net，沿注册development.yml、短期只读checkout、源码校验/去认证materialization和已有build guard，无托管prepare/retire、无空间策略修改。
初次dispatch0508ddc9f8834c6c9202b43452edfc1c被HTTP422工作流env上下文拒绝，无run；修正step env后同request创建 **42551075037 net-vendor**。
复用原snapshot而非改源：basea087 dirty、tree7b61b7edc12ed5ef384b7054240e7b5202f1638b、snapshota21c1a978549979a8956639d35fdbd722cc6c8df；Mac26.6.2 arm64、Rust1.95、jobs2、control6b1616c。
终态 **预期产品红：根格式/437Noq绿，Iroh35pass1fail**，仅a_fault_with_no_receiving_copy_can_use_a_backup_that_answered_after_the_fault实际Hold双PING、预期Go；home因失败未执行。mac-current-copy-hint-red-42551075037已核source/control并下载。
95不可变source refs，该run与原账单拒绝run共享快照但平台/任务不同；smoke没有产品source ref。不能把Mac单元记为Linux黑洞SLO。
37现已按红最小写候选，无接收提示用故障后已有备用ACK；有提示保持36规则。false旧包/4秒检查为新增待验证覆盖，Noq2/N3新源也仍待执行。11项Python开发入口测试通过。


37与Mac CLI已提交72b1d891c879cab63aaed032e201aee7baf1734c。**42552723913 net-vendor，executor=mac**正式调度：clean、treef6d0e84996723b8e9ad4659b6a2be8b625cb74b5、snapshotbafedc8a33390550c2bd0b10de0103e305500d31、controlb9b27116ad286fb51edc1e211949127681c5aa27。源码包括Noq2和N3未验证候选，不预认通过；host-capabilities只读，不启动VM或改资源门。96 refs，当前该run pending。


42552723913两attempt最终均**执行器源码传输失败**：attempt1 source checkout、attempt2 control checkout，Mac→github.com:443连接超时，未校验源/格式/编译/测试，无source artifact。两attempt raw/log/classification分开保存mac-fault-candidate-checkout-42552723913/及attempt-2；96 refs全部终态。原72b1/treef6d0/snapshotbafed/controlb9b271不改，不记产品红。
控制dd57be6c57090a811726983e700d0b39a40fd1e8已push：可选API Git bundle传输、准确GITHUB_SHA可信脚本、现prepare/guard，未远端验证。本机产品12项、bootstrap3项、prepare1项、driver1项以及YAML/16shell语法通过，无Rust/Node本机构建。STATE重写为当前状态，旧页原样归档HISTORY-20261109-before-api-transport.md；N0–N5范围不缩。


**42555870179 Mac net-vendor在途**：clean d2849d4732c7e9b6dd2e953616ce583f0e520391、tree4a9c4d89cc01e33a480375d50cc9f77b6d2d7a83、snapshotb6a4e9240786b1b178aee1e11284e6164aecc377、controldd57be6c57090a811726983e700d0b39a40fd1e8。API bundle17935476字节，blob91937c9b7d1330f2b1d8352dbe6d309c278e38b5，SHA256284904bec454a22f964fbb5c4f5e0307e3f76342815d95d641dd9f18c261cf0b。API bootstrap实际success、两Git checkout skipped、进入原build guard；测试结果未出。97 refs，本run唯一pending；mac-api-vendor-42555870179已存本机准确回执和raw run。


42555870179终态 **SUCCESS：根格式、438Noq/36路径/9选家**，回执tree4a9c4d89/snapshotb6a4e924/controldd57be6已逐项核对；mac-api-vendor-42555870179保存source/environment/bundle/原始log和classification。37快速Go回归由42551075037旧源红转绿；Noq2 reporting始终启用恢复实际通过。N3记录器仅格式通过，未编译/测量。host VZ=1、16GiB/10CPU、约76.2GB空闲、无已装VM工具，仅brew；没有启动VM。97 refs全终态，Linux原p3/CPU/hop与N2–N5仍缺。
新增N2真实dial锁deadline回归，未改生产；候选待Mac net-node取红，control2e70c39扩展原filter。


**42556474188 Mac net-node在途**，clean5d117deb、tree4f5179e59e64576fd6696b17f23fcc1c50fbc14b、snapshot44930072797913fbefd2a3d1f6e9269965cd0a5d、control2e70c39fe6a70bbbd09b7434f90e6c2d62e84b16。真实Node/Endpoint持dial锁的40ms deadline回归，N2生产未改，预期红未实际取得。mac-dial-budget-red-42556474188与98th ref已保存，勿重发活动run。


42556474188终态**根格式门失败**：仅新测试两处assert多行格式，格式patch已下载校验后原样应用；source/tree/control实际匹配，但无编译/测试，不记行为红。98 refs均终态。N2生产原样，下一准确源再取红。


**42556768499 Mac net-node在途**：clean2018ff95、treec60e145a72b88dd311fd90c9ff6a193983644210、snapshot654d409551e512462901a237e8bd47c7bc2c42da、control2e70c39fe6a70bbbd09b7434f90e6c2d62e84b16，原样应用远端格式patch后的N2测试源、生产未改。mac-dial-budget-formatted-42556768499/99th ref已锚定，未预记行为红。


42556768499终态 **实际N2预期红**：根格式、net66pass/1ignored、hop9/wires10/journal28绿；dial_budget1pass1fail仅真实持锁40ms调用期限仍等待到500ms防挂。source/tree/control已核并下载raw/log/classification，99 refs终态。随后最小CallerExpired+normal line有界锁候选，不记绿；过期空锁不拨号为新增覆盖，ask_early/hop/总期限等仍缺。


**42557583517 Mac net-node在途**：clean19f0e190、tree2fd3cf7ea6ba50276be1bb7e9243c784c50830d8、snapshotd76915fc0c91ffde65ceb63d5fde560fb5506162、control2e70c39fe6a70bbbd09b7434f90e6c2d62e84b16，normal line CallerExpired/有界锁最小修复候选；100th ref和mac-dial-lock-candidate-42557583517已锚定，结果未出。


42557583517终态 **SUCCESS**：根格式、net66pass/1ignored、hop9/wires10/journal28/dial_budget2全绿；tree2fd3cf7e/snapshotd76915fc/control2e70c39已核，mac-dial-lock-candidate-42557583517保存source/environment/raw/log/classification。normal持锁期限由42556768499红转绿，过期空锁不拨号为新增绿覆盖；没有完成early/hop/总期限等N2。100 refs终态。新增独立early入口持锁用例，early生产未改，预期红尚未执行。


**42557961726 Mac net-node early入口回归在途**：clean91d8777c、tree11262ccc042a1bfcd4628a5b00f669d7cafc151a、snapshot4eb0b8da9b37a2d3554c6179f479e347fd3ac5e7、control2e70c39fe6a70bbbd09b7434f90e6c2d62e84b16。normal锁修复不变、early锁生产原样，新测试待实际红；101th ref/mac-early-admission-red-42557961726已保存。


42557961726终态 **实际early准入红**：格式、net66pass/1ignored、hop9/wires10/journal28绿，dial_budget2pass1fail，仅early持锁40ms调用未结束至500ms防挂；normal控制绿。tree11262ccc/snapshot4eb0b8da/control2e70c39已核，mac-early-admission-red-42557961726保存raw/log/classification；101 refs终态。随后early有界锁/CallerExpired候选待绿，额外过期空锁覆盖不宣称旧源红；N2其余边界仍缺。N3双向调度校准协议在修改模型前冻结，尚未运行测量。


**42558490443 Mac net-node early候选在途**：clean802e13a8、treef2afc1c9a48599f6091675105bc66075c73fe830、snapshotbfb637abe9d3d8a56683a5d7c6b5fcefa0999273、control2e70c39fe6a70bbbd09b7434f90e6c2d62e84b16。102th ref与mac-early-admission-candidate-42558490443已锚定，尚未绿。


42558490443终态**格式门失败**，source/tree/control已核；仅新增early过期空锁assert内await换行，远端patch已原样应用，无编译/测试，不算early行为红/绿。102 refs终态。
N3道路双向因果校准测试已按先前协议写下，模型未改；control e7258e57155da1618f0f3b00fd1e395f481b5f19已push并在net-congestion计时前强制校准1pass/0fail。本机driver桩测试证明校准红即止、首计时轮红仍执行后两轮保留最终exit8；非Rust执行。下一准确源先net-node，再N3校准待红。


**42558820303 Mac net-node在途**：clean783619dd、treef3b250de87c64b452ccf2dac6543eb63d677938a、snapshot27bfd24c3233e98ab8998f8056bbba3f8650e5c4、controle7258e57155da1618f0f3b00fd1e395f481b5f19。early格式patch及N3校准测试源码；Node profile不执行N3，结果未出，103th ref/mac-early-admission-formatted-42558820303已锚。


42558820303终态 **SUCCESS**：根格式、net66pass/1ignored、hop9/wires10/journal28/dial_budget3全绿；treef3b250de/snapshot27bfd24c/controle7258e5已核，mac-early-admission-formatted-42558820303下载source/environment/raw/log/classification。early持锁期限由42557961726独立红转绿，normal控制保持绿；过期空锁两项为额外绿覆盖。N3测试文件只格式通过，未编译/校准；N2其他预算/取消边界仍缺。103 refs全部终态，下一net-congestion模型校准预期红。


**42559155803 Mac net-congestion模型校准在途**：clean599c57d0、treeb84a3bc4ea9a028fa8e2c147264623dfcb5fa971、snapshot54082af8662510e0548c5d4b584d06531724c8c0、controle7258e57155da1618f0f3b00fd1e395f481b5f19。模型仍旧shared_fifo_v1，先校准预期红；v2记录器也待编译，未有拥塞测量数据。104th ref及mac-road-model-red-42559155803已保存。


42559155803终态**测试fixture编译失败**，根格式绿、source/control已核；send_to误传u8，未执行校准/测量，不算模型红。已改成b"e"字节切片，模型生产原样；104 refs终态，下一源再取红。原log/source/classification保留。


**42559476637 Mac net-congestion校准在途**：clean6880d251、tree751e95115a7fb3a17faca1a5a44129e00a27655c、snapshote851d256586088d589971c2c1cb60f1461233a0c、controle7258e57155da1618f0f3b00fd1e395f481b5f19。仅修测试字节切片、模型原样，待实际红；105th ref/mac-road-model-typed-42559476637已保存，baseline六fixture合同现场复核仍保持。


42559476637终态 **实际模型红**：根格式和congestion目标（含v2记录器）编译通过，校准0pass/1fail，反向1B被未来正向包挡住至250ms超时；两ignored过滤、零计时轮。tree751e9511/snapshote851d256/controle7258e5已核并下载raw/source/log/classification，105 refs终态。随后只改测试道路为每方向独立FIFO，标independent_fifo_v2，schema仍v2且每条case/summary/sample带model；产品CC未改。control同步mode并桩检查通过；候选尚未远端格式/编译/校准/测量。STATE已汇总重写，旧页完整归档HISTORY-20261109-before-duplex-candidate.md。


**42560015454 Mac net-congestion独立方向候选在途**：clean1478bf9712ad50c00b502c78613cd932f20586b3、tree33d5ef6d47cde04620db1baa470bd7fefdf0c99d、snapshot9a6f9717470e0a741aedbdd9dd1ecf0a32d18fc8、control0cff9671fb2f80945423132e0df46995f6ff3198。模型independent_fifo_v2、schema v2，先校准1pass/0fail再三轮固定几何/seed计时；结果未出。mac-duplex-congestion-42560015454/106th ref已锚。


42560015454终态 **SUCCESS**：root fmt/congestion编译、UDP校准1pass、三轮各2pass（167.64/167.09/167.29秒）。模型从42559476637旧FIFO实际红转independent_fifo_v2绿；15bulk/51摘要/704samples，0失败/0超时；每轮235/234/235，raw samples与摘要计数/索引/分位数逐项一致，sample-audit.json保存日志hash和合并方法。tree33d5ef6d/snapshot9a6f9717/control0cff967已核、raw/source/environment下载。仅Noq模拟，产品CC未改，不是N3整包或Linux/真机SLO。106 refs全部REF:source复核、无pending。当前Node link和vendor内容与各自已验证run对象相同，applicability.json明确局部范围，不当最终Quality。


**42562380977 Mac net-node操作scope预算回归在途**：clean4b777300、treed3cc715bcac2173cdaa5905ad173ce295f931906、snapshot2daaec31a984488b7cfbf6f5ef8549448d1546c7、control0cff9671fb2f80945423132e0df46995f6ff3198。新增不可变deadline载体、未接入生产消费者/Op；真实ask持锁的40ms与300秒两个虚拟时钟回归待红，107th ref与mac-operation-budget-red-42562380977已保存。


42562380977终态仅**根格式门失败**，准确source/control已核；新测试调用排版patch已原样应用，无编译/测试，不记scope行为红。107 refs终态，生产消费者仍未接入。


**42562932053 Mac net-node scope预算回归在途**：clean28d9fe6e、tree6c3de5d38a2d302813f6c66401436d3ddf6454ad、snapshot79e97ce75b77d17889ac0a2bcb008fbff47df146、control0cff9671fb2f80945423132e0df46995f6ff3198。格式patch/ready结果须为正常返回错误的加强检查，生产仍忽略scope；108th ref/mac-operation-budget-formatted-42562932053已保存。


42562932053终态**实际操作预算双红**：fmt/net66pass+1ignored/hop9/wires10/journal28绿，dial_budget3pass2fail，只Scope40ms未到达ask、300秒被28秒截断。source/control已核、108 refs终态。随后操作入口/锁/hop/拨号/普通交换重试的绝对截止点贯通候选待远端验证；Accepted后回答裸await暂未改，需独立真实回归，不能凭锁测试收N2。


**42564150236 Mac net-node贯通候选在途**：clean1950943b、tree9f57f8181925f12763dd597fd336ead82741abd9、snapshot787c72e71038003da290e0b3009c47244eddc9cf、control0cff9671fb2f80945423132e0df46995f6ff3198；候选覆盖入口/锁/hop/拨号/普通交换读重试绝对期限，待格式/编译/五回归，Accepted后原await未改。109th ref/mac-operation-budget-candidate-42564150236已保存。


42564150236终态仅根格式门失败，已核source/control；签名/array/await排版patch原样应用，本机未执行formatter，无编译或行为测试。109 refs终态，贯通候选待新源绿。


**42564749932 Mac net-node贯通候选格式修正源在途**：cleana4ca8637、tree1f1677176b810668ef61103fafaffd9e52d1eb66、snapshotc39f8cc6f3793d2ef828f9d7b29ff83bdc479d91、control0cff9671fb2f80945423132e0df46995f6ff3198。仅修远端布局，110th ref与mac-operation-budget-propagation-42564749932已锚，不预认行为绿。


42564749932终态**Node编译失败**：根格式/net66pass+1ignored绿；dial_update旧std参数与新Tokio签名不匹配，Node测试未执行。准确tree1f167717/snapshotc39f8cc6/control0cff967/ref已核，mac-operation-budget-propagation-42564749932保留raw与classification；tests_executed=true仅net单元，不算行为红。修正候选保留更新10秒自身上限，另合入操作截止点；独立ask外层预算改由已有阶段上限合成，不套IPC档位。110 refs全终态。


**42565998212 Mac net-node更新拨号上限修正候选在途**：baseeb4f8c1b70d118394b596ff21bd31a280ad4a412 clean、treeab8e723fbb02aecb43f7c86b0577563f6c32052b、snapshot8964af950889ceec388af1af9eabf900db9206eb、control0cff9671fb2f80945423132e0df46995f6ff3198；111th ref和mac-operation-budget-limits-42565998212已锚。更新保留原10秒自身上限；独立ask外层由既有阶段预算合成。待格式/编译/五预算回归，不预记绿。


42565998212终态仅**格式门失败**：提取RTT_BUDGET_MOST后budget_for链式表达缩为一行，远端patch原样应用；未编译/测试，不记行为红。source/control/ref已核，111 refs终态。新增真实0RTT Accepted→调用取消回归正在编写，生产回答所有权尚未改。


**42566472411 Mac net-node Accepted回答取消回归在途**：base371d92b018bfd0732edda959504a0af887e6db99 clean、tree3efb5c3ef62d366356d9db978a55464b5ce4c732、snapshot9c417d4632b2040881d8898205ec5c506dd79a08、control0cff9671fb2f80945423132e0df46995f6ff3198；112th ref/mac-accepted-answer-red-42566472411已锚。生产裸回答所有权未改；五预算回归也待实际验证，不能预认新用例行为红。


42566472411终态仅**格式门失败**，新fixture的matches/链式调用/async块排版，远端patch原样应用；准确source/control/ref已核，112 refs终态，未编译或执行六测试。所有权问题尚未取得行为红，生产仍原样。


**42566727617 Mac net-node Accepted回归格式修正源在途**：basef1ef9049559cbb26b93eee361f293929842e53c3 clean、treec21e025c0cbd66cdb4aca4a5ab523d3be1be7434、snapshot9ca1c0fab8980b93903aee579ee14bf01f6b7f33、control0cff9671fb2f80945423132e0df46995f6ff3198；113th ref/mac-accepted-answer-formatted-42566727617已锚。六测试仍待真实编译/执行，生产回答所有权不变。


42566727617终态**fixture前置失败**：根格式/编译/net66+1ignored/hop9/wires10/journal28绿；dial_budget5pass1fail。两个scope40ms/300s实际红由42562932053转绿，原三预算回归均绿。取消测试因server recv.is_0rtt=false停在前置，尚未取消，不算所有权行为红。准确treec21e025c/snapshot9ca1c0fa/control0cff967已核，113 refs终态。下一fixture将server第二次accept后首TLS响应延迟100ms，仍必须is_0rtt真实前置；不删除该门、不修改生产准入。另复用fixture新增Accepted后1秒绝对by必须结束的第二回归，预期红尚未执行。


**42567204795 Mac net-node延迟服务端握手fixture在途**：based3e9a6a8772211cc603c67be4c7d55f0babf4487 clean、tree68329fdd70be7901a43a3194c79b154002d7a29a、snapshot225a9db2aa9df1ca769618ea0d4d374e9b0028f8、control0cff9671fb2f80945423132e0df46995f6ff3198；114th ref/mac-accepted-answer-delayed-42567204795已锚。取消与1秒by两Accepted回归生产未修，待格式/编译及真实前置判定。


42567204795终态仅**格式门失败**：deadline变体matches缩为一行，远端patch原样应用；准确source/control/ref已核，114 refs终态、未编译或执行七测试。随后官方Noq1.3 poll_accept源码明确is_0rtt在应用接纳当时取is_handshaking，不能当此前包加密级别证明；notes在结果取得前已保存。修正fixture以真实第二请求+Node adopt+调用继续pending20ms证明Accepted分支；撤回is_0rtt硬前置和100ms握手延迟，标志只观察。所有权生产、STOP/独立请求/计数/peer条件不改。下一准确源先取真实所有权红。


**42567527752 Mac net-node Accepted所有权双回归在途**：baseb097a2dec5a61e26032b8f4336d1caf4fe56e17a clean、tree54b47ae52986e9d1a1663a25805c212e30d94aee、snapshot6c7e63589e1a968650dcc3021dc337a471718a22、control0cff9671fb2f80945423132e0df46995f6ff3198；115th ref/mac-accepted-owner-red-42567527752已锚。使用实际adopt+pending的Accepted前置；生产裸回答所有权未改，取消和1秒by预期红未取得。


42567527752终态**Accepted回答所有权双实际红**：根格式/编译/net66+1ignored/hop9/wires10/journal28绿；dial_budget5pass2fail，只1秒by被回答忽略、调用取消后回答流未STOP。两fixture均经过第二请求/adopt/pending20ms实际Accepted前置，且最后独立Sessions成功、stable_id相同、请求数3/零dial_failed均已过，随后才在目标断言失败；不是前置失败或靠关闭conn的伪红。准确tree54b47ae5/snapshot6c7e6358/control0cff967已核，115 refs终态。下一最小修复把Aborts随Accepted回答交接，等待显式沿原by；不改白名单/请求frame/共享wire。


**42567946141 Mac net-node Accepted回答所有权修复候选在途**：base2fc650b30a3ab16792adb7babe566b8cec1bfeb5 clean、tree314894b33ed72a5ff2aeed47504d6c9e7f575bd9、snapshot78c6fb8eabf86ff755800188d9f92e64f8d4e3cf、control0cff9671fb2f80945423132e0df46995f6ff3198；116th ref/mac-accepted-owner-candidate-42567946141已锚。等待七预算/取消回归与既有子集，不预记绿。


42567946141终态 **SUCCESS**：根格式/net66pass+1ignored/hop9/wires10/journal28/dial_budget7全绿（预算1.67秒）；准确base2fc650b3/tree314894b3/snapshot78c6fb8e/control0cff967已核，mac-accepted-owner-candidate-42567946141保留source/env/raw/classification。scope两红42562932053与Accepted取消/期限双红42567527752均转绿；同wire独立请求、strict3/零peer失败控制均通过。仅Mac便携子集，不是N2全包/完整Quality/Linux或真机SLO；116 refs全部终态且本轮逐项复核。STATE已汇总重写，旧页原样归档HISTORY-20261109-before-accepted-green.md。


**42569493787 Mac net-node普通wire/重试四项覆盖在途**：base2aace27d1c7bf91aafd61600954bd8a3bd3814af clean、tree089f73291f6b809d1e2d4251e2e1ccfa36a5fd3c、snapshot35c95dc5d489b46db7382cc8a2426a8719d29875、control0cff9671fb2f80945423132e0df46995f6ff3198；117th ref/mac-ordinary-wire-coverage-42569493787已锚。生产不变，旧7+新4共11预算用例待格式/编译/运行；没有预记新用例绿或旧源红。


42569493787终态仅**格式门失败**：新增wire fixture的adopt链与一处matches缩进，远端patch原样应用；准确source/control/ref已核，117 refs终态，未编译或执行新4/旧7测试。生产不变，继续新源验证覆盖。


**42569954515 Mac net-node普通wire格式修正源在途**：basef42121b51e5ceb5252dd8594a8f6523553c33450 clean、treef198bbd5b488cf3f4e59f15b2ac1c77f6b11a1cd、snapshot40418eb1f61e2a436f583e846fc766f89e4bdfec、control0cff9671fb2f80945423132e0df46995f6ff3198；118th ref/mac-ordinary-wire-formatted-42569954515已锚。生产仍与2fc650b3相同，11预算测试待实际结果。


42569954515终态 **SUCCESS**：根格式/编译/net66+1ignored/hop9/wires10/journal28/dial_budget11全绿（预算6.73秒），新增四普通wire覆盖均实际执行：scope到期STOP本流且并发控制成功/同stable_id、真实读重试沿原by、失答读最多一次、Mkdir失答不重放。准确basef42121b5/treef198bbd5/snapshot40418eb1/control0cff967已核并下载，118 refs终态。生产与2fc650b3相同；新增绿覆盖不宣称这些用例在旧源取红，F7真正长黑洞仍缺。下一探针取消/首成功回归生产保持原样待实际红。


**42570381896 Mac net-node探针所有权/首成功三回归在途**：basea2cf865cf1c0f3eb5073ffac5612a8ab8b7a6ae1 clean、treef7d1528532848f0fccbe4b8eb674cc66183982e0、snapshot70ec29da02a513fed7c04e6a8b8810756783bbf6、control0cff9671fb2f80945423132e0df46995f6ff3198；119th ref/mac-probe-owner-red-42570381896已锚。生产pair/doctor原样，旧11+新3待格式/编译/实际前置与红，不能预记行为红。


42570381896终态仅**格式门失败**：probe fixture const/timeout布局与assert，远端patch原样应用；source/control/ref已核，119 refs终态，未编译/运行。probe生产仍原样，三行为红尚未取得；下一源继续。


**42570772550 Mac net-node探针回归格式修正源在途**：baseffbbcaac13a6d9f54591afdac1bebeeb5dbe415e clean、tree0a26b34f68b867d72eb047f9c6ec8234b68e5d59、snapshotcc5b77828bb5a5addb0db6ed8739824700c9697e、control0cff9671fb2f80945423132e0df46995f6ff3198；120th ref/mac-probe-owner-formatted-42570772550已锚，probe生产未改，旧11控制与新3红待实际结果。


42570772550终态**探针三项实际行为红**：fmt/编译/net66+1ignored/hop9/wires10/journal28绿，dial_budget11pass3fail（12.06秒）；只直接取消HTTP未停止、配对取消仍残留probe、成功second URL被slow first挡住。所有HTTP/配对pending/fast已响应且早于2.6s前置通过，无env proxy；不是fixture红。准确tree0a26b34f/snapshotcc5b7782/control0cff967已核，120 refs终态；下一修复用JoinSet收首成功并drop输者，pair_with用Aborts持有外层probe，保持原HTTP预算/proxy/诊断语义。


**42571393016 Mac net-node配对probe双层所有权修复候选在途**：based4a2e115b898fc337f5091b51fa71e45740c667f clean、treef278b9330f51268182b142729db2c86687f4af4b、snapshot3459894c81cb994310a97bfcba4ca39ae97e569b、control0cff9671fb2f80945423132e0df46995f6ff3198；121st ref/mac-probe-owner-candidate-42571393016已锚。doctor JoinSet首成功+取消输者，pair Aborts闭合外层取消；待十四专项/既有子集，不预记绿。


42571393016终态 **SUCCESS**：fmt/编译/net66+1ignored/hop9/wires10/journal28/dial_budget14全绿（9.93秒）。准确based4a2e115/treef278b933/snapshot3459894c/control0cff967已核并下载，121 refs全部逐项验证/终态。probe三实际红42570772550→绿，旧11保持；普通四项是42569954515首次取得的新增绿覆盖，不冒称旧源红。仅Mac子集，不是N2全包/完整Quality/Linux SLO。STATE已汇总，旧页归档HISTORY-20261109-before-probe-green.md。


**42573745492 Mac net-unit只读guard采集在途**：base03df5a47f37d24a0473b9693320e48aed78b52bb clean、tree919008b695a05cd3dbbb0f90b0d5db94d193818e、snapshot2d69c1a1a9c5d13c95a70bbee90d4df83258682f、control60821accc131e837da476706cb6b3ad93812d30e；122nd ref/mac-guard-capture-42573745492已锚。产品生产未变，读取admission文本/hash及net-unit通道验证待结果；未执行VM。


42573745492终态 **SUCCESS**：root fmt/net66+1ignored绿，guard两文件大小/SHA256已核，准确tree919008b6/snapshot2d69c1a1/control60821acc已核，122 refs终态。实际installed guard SHA998f5723eb447a1634411397c18a524e5ed0d50bd6dbd81001a16626219f64eb：准入50GiB、运行中止40GiB、Mac target100GiB，非此前文档35/20；当前free72254976000B，VZ=1/16GiB/10CPU。guard对子命令start_new_session/pass lock fd，TERM后10秒KILL整个PGID，但leader已退出则不杀；据此VM需foreground同组及lease supervisor闭合父死清理，不能只依赖正常finally。未启动VM或Node/完整测试。


VZ能力三run（源码一致baseb56576ce/tree2b5bfce2/snapshot6a046588）：42578713267 control97334be，固定资产下载校验通过，完整SSH临时socket107>104，未boot；42579282147 control8d39080，native raw/VZ/Linux boot及正常清理实际成功，guest探针非零空输出/无退出码，父死未跑；42579909366 controlbedd896，当前IID完整就绪、namespace/veth/netem/nft/conntrack、正常/真实controller SIGKILL=-9后lease EOF清理全绿，stop rc0/双Stopped/同guard PGID/无本次helper/清理临时state和tools。两IID不同。均product_tests_executed=false，前两executor红不隐藏，第三能力绿不当产品SLO。对应证据mac-vz-capability-42578713267 / mac-vz-short-path-42579282147 / mac-vz-ready-42579909366，123–125 refs已锚/全终态，复用者不拥有原源分支。125 REF:source逐项核一致。下一步可信Linux产品driver/固定Rust1.95后同后端基线候选原门；完整N0–N5继续。


**Turing candidate turing-20261109T075531Z-b6277f48 SUCCESS**（非GitHub run）：basec55d74ddaf697f7a389c2bc377d1e0016c47adcf clean、treeef392bb746c998a10c0b6cf8b14f40ffcfe2a2aa、snapshot8ff966dbf71bb0c027bbeac2277ccce6e315dbcf、control59dc223ad263056bc0fa5c073c69a9c5d66b592a；126th ref。Rust1.95 x86_64/kernel5.15/128CPU，jobs16、原recovery单线程三轮；源码/driver/bundle/环境/原job进程组已核、完整report与SHA清单已下载。首编译6m19s；3轮各2pass，p3最慢518.5/597.9/706.9ms、各零失败/退役、p2三绿。只收recovery；未跑root fmt/CPU/hop/full，不能跨旧机器算收益。baseline turing-20261109T075906Z-5464f2db（base6485/tree52a44260/snapshot9e64c63c/control59dc223，127th ref）随后串行launch，PID4135341，编译55.57秒，仍在途。两源五实际fixture逐字同，baseline生产原样。完整目标active。


**Turing baseline turing-20261109T075906Z-5464f2db FAILURE5/6**，准确base6485/tree52a44260/snapshot9e64c63c/control59dc223，127th ref。三轮p3最慢787.0/787.1/704.3ms均过原1s；p2 p95=73.691917/71.255568/247.204224ms，第三轮超过原200ms、其余route/保持/零错/零退役条件过，源码878行原assert实际失败。完整report采证/tuple/driver/bundle/environment/group/SHA已核。candidate同机6/6，但host load从candidate后9.12到baseline后46.12，未隔离的背景变化禁止宣称源代码或patch37的因果加速；不把原性能红改算executor红，也不删除。对照turing-recovery-comparison.json。随后4任务stage（128–131 refs），candidate hop128th turing-20261109T082539Z-a6f5aaf4已launch PID4163640，其他3未launch；同source角色tuple/原profile/control59dc223，当前131 refs逐一核、127终态+1在途+3 staged。


**Turing corrected hop136th turing-20261109T091323Z-60e73cac FAILURE8/9**：basefd1f30ee8b3761940a60c98ff7c65559bd7d16e4/tree78d063d915f033d9bffa58516488b65de9f0f14b/snapshota828463e8bc10dc9038e38e3e4777acac21cb443/control59dc223，measurement_revision=hop_occupancy_started_v2。三轮2/3、3/3、3/3，完整report已采，执行1652.41秒。三轮f4/f4s绿；f4i首轮to_relay=1超原0，X carried96.8→relay98.5→carried104.3秒，首行started条件通过；后两轮f4i绿，但第三回carried91.4秒。原20s/1.5s/30s门未改；f4i未受reducer修正。f4s最大按键8.6/10.2/10.1秒，不能将这些旧场景绿扩大为体验全绿。baseline同fixture137th只stage未launch。

Mac139th42594133373仅format门失败；baseef90/tree3f3c4c52/snapshot800a54f8/controlb998，headSha/transport/source核一致，未编译/测试；原format.patch应用后冻结新源。**Turing140th turing-20261109T094205Z-595cfb65 net-node已launch PID143315**：base6fff96a389d74aa0524257a4d4c250654bae5b90/tree6099df332c86ff7c4224cbcec0d22ec1fb4ba28f/snapshot679507082f2343b1ba4ab36d475c8ea7ee328b22/controlb9984797f412f352f3c76f99c47ade3ee10f68e9。真正Linux namespace F7待运行，不用Mac未执行的测试报绿。


**Turing141th turing-20261109T095017Z-93c2c1a2 SUCCESS**：base9eafba741772138e9dfbe59ca9a010cb843b1ef5/treef93c32b1aa8ef9b7c36d9e3e2437c2beb26851b4/snapshot6708accbbb016be75c10befdbb0d14f1ce002875/controlb9984797f412f352f3c76f99c47ade3ee10f68e9。固定Rust1.95，fmt/编译/net66+1ignored/hop9/wires10/journal28/dial_budget16全绿（21.24s），真正新namespace/nft黑洞静默后abandon一次退役与长ACK共享wire阳性均过。前一140th只nft规则语法红，report保留；改多行后独立新namespace nft --check=0，新源runtime绿。不冒称旧源行为红，也不以16绿收N2或IPC/grace边界。完整report/source/control/bundle/group/hash已核；141refs=138终态+baseline hop在途+2staged。**baseline hop137th2c64555c已launch PID161772**，原tree97040f31/snapshot90126283/control59dc223和三轮门不变，禁止重发。


137th turing-20261109T091325Z-2c64555c corrected baseline hop终态**FAILURE7/9**（2/3、2/3、3/3），准确tree97040f31/snapshot90126283/control59dc223，执行1510.22秒。f4/f4i3/3，f4s1/3；两红分别晚回退后的after-leg门、恢复26.2秒后再选Relay的原30秒无回退门。完整report/身份/字节/hash/原PGID已采核，与candidate8/9 whole-source观测对照存turing-corrected-hop-comparison.json；不能断言单补丁因果或稳定性已解决。

143rd turing-20261109T100821Z-08748d90 net-node已launch PID236331，准确tree42050b56/snapshoted287397/controlb998，Linux17（原16+F8）待实际红/绿。Mac142/144/145均只有formatter门红，原patch应用、未冒称行为红；146th42600205050已dispatch/锚ref，treebbed6503/snapshotc8035ce2/controlb998，29journal与17便携（15旧+2update）待结果，Linux F7/F8排除。


143rd turing-20261109T100821Z-08748d90终态**FAILURE16pass1fail**；准确base5aa96708/tree42050b56/snapshoted287397/controlb998，root fmt/编译/既有子集绿，F7绿。F8前置均过：对端2401ms收到并回复两请求，调用4101ms过期，heard16→16；共享wire已退役None、retired_unanswered1、控制请求失败，目标“probe grace前不判共享wire死”断言实际红。完整report/原group/source/control/bytes/SHA已采核；不归fixture红。下一个生产修复需新准确源，不能重发此run/放宽反例。

146th Mac42600205050终态**SUCCESS**：base83d620d2/treebbed6503/snapshotc8035ce2/controlb998，root fmt/编译/net66+1ignored/hop9/wires10/journal29/dial_budget17全绿（30.57s）。真实update_dial两合同及wire身份旧日志/roundtrip/人类句子兼容通过；Mac17=15旧便携+2update，Linux-only F7/F8排除，不覆盖143rd红。全部raw/source/env/transport/hash核留，146refs=144终态+2staged，当前无在途，先修F8再CPU。


147th turing-20261109T104331Z-90266679仅FORMAT红；148th turing-20261109T104749Z-3c7d804a与149th Mac42604002430同snapshot ef83ab11/tree473e41ce/controlb998，仅Node两处未限定Arc编译红（net66+1ignored过、没执行Node合同）。原format.patch应用并仅补std::sync::Arc，原始report/身份均采核，不混行为红。

150th turing-20261109T105401Z-df35bc80 **SUCCESS**：base25d0c803/tree6805b7b7/snapshote0a42573/controlb998；fmt/net66+1ignored/hop9/wires10/journal29/dial_budget21全绿（46.37s）。143rd真正F8红的函数逐字不变→绿；F7按f7_owned_confirmation_v2的真实黑洞、std thread Drop独立确认及Weak owner/replacement两回归均过。151st Mac42605391960与150准确source完全相同，**SUCCESS**，同基础/29journal/19便携全绿（32.31s），Linux F7/F8排除。两边全report/tuple/control/transport/group/hash采核；只证明Node子集，完整N2及SLO未收。

旧stalls130/131未launch已superseded，保留ref/receipt且tests_executed=false。新152nd turing-20261109T110456Z-dee800e0 candidate stalls已launch PID288279，source同150；153rd turing-20261109T110456Z-42e6ec16 baseline仅stage、未launch，复用137th6485原生产+六fixture/source97040f31/snapshot90126283，两者controlb998、原p1s/p5/repeat1/test_threads1不变，五实际fixture逐字一致。152已实核live，编译55.96秒后进入p1s；终态采证后才launch153。153refs=151终态+1在途+1staged。


152nd turing-20261109T110456Z-dee800e0 candidate stalls终态**SUCCESS2/2**，准确source同150/151（base25d0c803/tree6805b7b7/snapshote0a42573/controlb998），1193.92秒、场景1135.16秒。原p1s 600秒/20×300ms，alpha/beta0/0切路、零退役；最慢按键3.8秒、12打洞、3449包/1335065B，旧p1s不对这些体验/开销值设门，不能宣称完整体验绿。p5 150/300/600ms各4次，三档零选relay/零打洞、最慢279.9/518.6/992.0ms。全部report/source/environment/driver/bundle/原group/hash采核，classification保留缺口。host load before4.90/6.36/7.33，after5.58/8.26/8.90。

153rd baseline turing-20261109T110456Z-42e6ec16随后串行launch **PID327881/child327922**，已实核/proc与network-driver，仍用tree97040f31/snapshot90126283/controlb998原门。154th candidate hop turing-20261109T112647Z-c5e5e53d复用150准确snapshot，仅stage未launch；原单线程三轮/原hop occupancy v2，待153终态采证后再启动。154refs=152终态+baseline153在途+hop154 staged，不重发。


F9/F10 fixture：155th Mac42611015315（basefdaf20f8/treee58c548b/snapshot8f52df34/controlb998）只format红、无编译/行为测试；完整source/env/transport/control blob/hash已核，原format.patch应用后156th Mac42611507471（basedf6188c5/tree0552f630/snapshote20f36dd/controlb998）**SUCCESS**，net66+1ignored/hop9/wires10/journal29/dial_budget19全绿（32.25s）。新namespace F9/F10没有在Mac编译或执行；健康Accepted两原控制保留，生产仍25d0c803。

157th turing-20261109T114555Z-10025913仅stage，与156完整snapshot/tree相同，bundle SHA也同ef1b74f9bf0d92f4f6d6c4be096c3b1316e392afdf0f670c6830e2f6ab9b3fe1；controlb998。原net-node单线程21+新2待真实结果。stage时本地无Mac临时bundle blob，未转移/启动；沿同已锚snapshot重建bundle后SHA逐字相同，staging-note保留，非产品红。为先定共享活性风险，baseline153终态采证后先157，再hop154，当前均未重发。157refs=154终态+baseline153在途+两staged。


153rd baseline stalls终态**SUCCESS2/2**，tree97040f31/snapshot90126283/controlb998，1190.62秒。p1s alpha1/beta0移动、零退役、最慢732.4ms、5打洞/2905包/1215130B；p5三档零relay/打洞，最慢310.1/499.8/976.0ms。全部report/group/source/driver/bundle/hash采核。与152候选的单对顺序观测已存turing-shared-probe-stalls-comparison.json，不宣称整体或单补丁加速。原p1s门是每端最多1次移动/零退役，非0移动或长尾/开销门；原busy_beside忽略taskset状态且不保证2busy进程都启动，负载有效性后续先校验，不能据打印“one busy CPU”推定已强制。

随后157th net-node已launch **PID365030/child365065**，source同Mac156，实际/proc与network-driver已核。F9/F10+原Linux21待结果；154th hop仍stage，先取得活性合同结果再决定修复/准确源复验。157refs=155终态+157在途+154 staged。


158th turing-20261109T115934Z-20aa3073 **FAILURE21pass2fail**，base41c50ba7/treed0484c04/snapshot4767c09c/controlb998，199.74秒。只将F9原前置断言前移，网络/时长/门不变；F9与F10均通过完整真实namespace/零新包/drop>0/remote-open等前置后，在独立owner confirmation目标上实际红。原Linux21全绿，F8函数未改。完整source/driver/bundle/group/hash已采核；157旧F9暂定分类不改成已完成前置。当前候选对齐probe基线并使tick等待pending proof，Accepted加入guard，长取消触发当前计数独立验证；另外加16秒健康Accepted阳性。尚未运行新候选，不能预记绿。


160th turing-20261109T120752Z-38c3c975 **SUCCESS Linux24**，basedf592ad7/tree3c8bc03a/snapshote30f6377/controlb998，总217.80秒、合同92.30秒；fmt/net66+1ignored/hop9/wires10/journal29保持。158两真实红F9/F10转绿，三个core helper/F8函数SHA逐字一致；新16秒健康Accepted也绿，原F7/F8/owner/replacement/期限/读写/探针/update21保持。161st Mac42614651338完全同source，**SUCCESS Mac20**（50.01秒），相同基础子集全绿，四Linux-only排除；两份完整报告/身份/control/transport/hash/原group采核。159th仅format红，原patch已应用并加disarmed guard早返，未编译/行为测试。

旧154th hop已现场核远端无report/launcher，从未执行，superseded_not_launched、tests=false，refs/input全保留。162nd turing-20261109T121345Z-8ba7e561使用与160/161完全同source，原hop三轮，已launch **PID390824/child390871**、实际/proc与driver核live；五实际fixture对baseline137逐字一致、network.sh对59dc逐字同，证据turing-tick-accepted-hop-fixture-identity.json。162refs=161终态+162在途，全部snapshot/tree双引用批核；goal完整active。


162nd exact-source hop终态**8/9**（3/3、2/3、3/3），source同160/161，controlb998，总1688.88秒。第二轮f4i回carried52.1秒后出现relay，X同wire139750384102992 carried→relay7347ms且carried候选消失；Y的两事件wire不同。f4s三轮旧门过但按键均9.9秒。完整report/原group/tuple/driver/bundle/hash采核，classification存run目录；观测快照也保留，原因待生命周期记录，不算显示假象或排名定因。

163/164/165 Mac分别模块路径解析/format/format门失败，无编译/行为测试，完整source/control/transport/hash均已采核；source修正后166th turing-20261109T130143Z-4777e11d CPU短校准**3/3**，base886b084c/treeed97f217/snapshot349bd7ba/control7534，总61.07秒、Rust校准0.50秒，CPU14/100ticks每秒，两contender305ms各增加15ticks，actual thread affinity/负例/cleanup通过。calibration resident为owned sleep，非Node或p1s；旧CPU报告不回写。167th Mac42622111828完全同source/control，root fmt/net66+1ignored绿，仅net-unit，未跑Node20。

control7534d6e已push，仅增加SSH-only cpu-check与hop-lifecycle；驱动Python stub2＋runner4 checks过，本机没执行Rust或真实netlab。新增diagnostic默认关闭，不改cleanup/排序/人类journal。168th turing-20261109T130858Z-465046e7同166/167完整snapshot/control，原f4i单线程三轮diagnostic非SLO，已launch PID486253/child486308。168refs=167终态+168在途，snapshot/tree全量批核，完整目标active。


168th在途观察已落`observations/lifecycle-first-observation.json`：实际PID486253 cmdline匹配，前两轮f4i原门绿（回carried17.2/10.7秒），每轮三端日志共9生命周期事件，第三轮未完成；尚未重现162第二轮撤路，不能将诊断绿当稳定性通过或定因。下一继续同PID/原report，不重发。


168th turing-20261109T130858Z-465046e7终态诊断**3/3绿**，base886b084c/treeed97f217/snapshot349bd7ba/control7534，691.75秒。三轮回carried17.2/10.7/23.2秒，原有意离线后的serial1 IO结束均位于返回前，serial2返回后未结束；9日志共27事件、无hop_measured_release/link_detached。完整source/control/driver/bundle/原group/hash采核。原162同wire撤路未重现，非修复或SLO通过，保留红结论。

169th turing-20261109T133049Z-d4827d09与168完全同source/control，原netlab-stalls单线程1轮，checked_affinity_v3真实Node压力，launch PID522226/child522272实际/proc/driver核live。170th turing-20261109T133425Z-7a9c0dfa原版同条件只stage，source base6485/tree186d3b60/snapshot63043993，control7534；五实际fixture逐字同候选，除七fixture/helper文件外生产全同原6485，证据cpu-v3-paired-fixture-identity.json。新detached基线保留旧WIP不动；169终态采证才launch170。新的comparison_role在stage前明确baseline，helper guard不变。


169th真实Node CPU准入**FAILURE**，rc101，总581.57秒，原p1s在BusyCpu::start返回无路径ENOENT时停止，0个admitted记录，600秒窗口未开始；p5原门绿，150/300/600ms最慢295.9/497.0/972.6ms。完整report/source/control/bundle/原group/hash采核。170th原版相同fixture从未launch，已superseded_not_launched，inputs/ref保留。166 sleep短校准仍是真实3/3，不扩张成Node准入证明。171 Mac42627527851（basebe31e983/tree42f87d24/snapshot601c3c9c/control7534）仅format红，原patch应用，未编译/行为测试；首次下载遇只读GitHub EOF，重取同run成功，没有重发构建。


172nd turing-20261109T135004Z-7a6fa08a（base7da5608e/tree62657b3b/snapshot56ce281d/control7534）CPU短校准3绿1红，101.22秒含编译、四case40.47秒。新真实Node case定位读取/proc/550613/task/551658/status返回ENOENT，原sleep/负例/解析三个保持绿；完整source/control/bundle/group/hash采核。173rd Mac42629466427同完整source，只有CPU新case assert布局format红，无编译/行为；原patch已应用。当前fixture修正为退出感知的完整存活线程快照＋前后process identity，100ms无完整快照失败；压力和原门保持，尚未预记绿。


174th turing-20261109T140109Z-0973b7c9（base86bb24cd/treecfcde850/snapshotad2fee6e/control7534）CPU短校准**4/4绿**，101.18秒含编译、case41.38秒。真实两个Node561853/562203的274/275存活线程均CPU89，真实请求过；492ms内两个contender分别增加9/10ticks，identity/进展/负例保持。sleep控制也通过。完整source/control/bundle/group/hash已采核。175th Mac42631155176同准确完整source仅format红（两个CPU helper hunk），无Node编译/行为测试，原patch已应用。后续完整A/B以allowed CPU集合固定中间ID替代按harness PID选核，日志标allowed_midpoint_v1，压力及门保持；未执行的新选择不能当作174观测事实。


176th turing-20261109T141037Z-b813a5b3原版fixed-core exit-aware v3 stalls已launch PID570285/child570324，实际/proc/driver核live。base6485/tree49211b55/snapshot31db523a/control7534，生产除七fixture/helper文件全同原6485；五actual fixture同candidate c716/tree5ba04b34/snapshota1981daa逐字一致，proof cpu-v3-fixed-core-paired-fixture-identity.json。Mac后者准确source net-node提交中，exec82832、request b2822fb7885c4d358e3647cead841604，仅source/bundle上传已知，待真实run ID不能重发。


177th Mac42632331558（basec716c916/tree5ba04b34/snapshota1981daa/control7534）root fmt/net66+1ignored/hop9过，wires10过1前置红。新fixture两条连接都已echo真实字节，但incoming selected-carried断言失败，ownership目标未到，journal/dial-budget未跑；完整身份/transport/control/hash采核。vendor源码表明shared peer selected tuple不一定在每条conn的open paths中，watcher可无selected marker；v1没有输出具体facts，不能认定实际None或Direct。v2改为双方carrier-only物理路径＋client选择＋原outgoing IP选择、2秒有界snapshot等待和三方rawfacts，目标/清理控制不改，未预记产品红。176 baseline仍计时，CPU64准入/首2spike实核所有thread mask/两busy进展。


176th原版stalls终态**2/2**，1189.88秒，base6485/tree49211b55/snapshot31db523a/control7534。22个CPU记录覆盖610846ms，CPU64、两resident275–280thread全mask64、identity稳定，两busy每期ticks前进。p1s1/0移动、0retired、最慢537.2ms、3punch/2801包1211229B；p5三档零relay/punch，最慢273.8/501.0ms/约1.1s。完整采证及178refs双引用批核。原p1s丢弃echo error值，原门仅稳定性，不能宣称完整失败分布/延迟SLO。

178th Mac42634473431（baseb9e7d4a3/treef7a24347/snapshot0872b5bb/control7534）**真实policy红**：fmt/net66+1ignored/hop9保持、wires10过1红。v2真实双echo、outgoing选IP、incoming仅Custom且selected=false、client确选Custom、wire不同、sole-direct与closed-incoming两控制全部先过，最终live-carrier消费者不得释放目标红。完整source/control/transport/hash采核；整fixture SHA3a03d41a...已留。当前只改释放判断聚合Main/Tunnel/served live并去重，路径gain与2×5秒门保持，新增shared_comparison诊断；fixture/5 CLI timed files逐字不改，尚未复验。不把这一独立缺陷当作162原因已定。


179th turing-20261109T144404Z-49c374ac与180th Mac42637760137完全同source（base3948872d/tree3ffbc0f7/snapshotab6cbce2/control7534）**双端绿**：fmt/net66+1ignored/hop9/wires11/journal29保持、Linux24（91.99秒）/Mac20。178整v2 fixture SHA逐字不变红→绿，hop.rs/netpath不变；五CLI fixture与netlab.sh逐字同176。完整report/source/control/bundle/transport/原group/hash留核，180refs已批审。只证明Node合同，不代替完整hop/CPU/WAN/手机。

181th turing-20261109T145440Z-735ffd55 exact same source candidate stalls已launch PID624261/child624303，实际/proc/driver核live，已CPU64准入/all masks/两busy进展；182th turing-20261109T145455Z-205ad096同source原hop三轮只stage，181终态采证后串行launch。182refs双引用全批核，队列/evidence同步；两长任务结果未出，不预记绿。


181st候选stalls终态原门**2/2**，1190.05秒，source同179/180。22个CPU64 records/全mask/identity/两busy进展保持，原p1s1/0 move、0retire，但18punch、4369包1641546B、最高728.3ms，176对照3punch/2801包1211229B/537.2ms；完整比较fixed-core-original176-candidate181-comparison.json。原p1s错误未留，不能宣称零失败或整体加速。182随后launch PID662914/child662947实核live，首轮f4红、f4i/f4s绿，窗口起即Relay且结束193ms，observations/first-f4-red-context.json保留；后续仍在跑，未定因。

N0 v4源码开始记录paths/hop逐次logical_request和f4故障窗口，ignored五个真实请求校准＋schema consumer未远端跑。控制5c67b39新增SSH-only measure-check，Python driver2/runner4共6 stub checks绿；原timed case命令与门不改，当前182仍旧准确source/control。


182nd turing-20261109T145455Z-205ad096原hop三轮终态**8/9红**，source同179/180/181、control7534，1590.25秒，原group清空。三轮2/3、3/3、3/3，第一f4已Relay且未恢复；y→x同wire carried→relay5936ms并缺Custom，旧fixture无准确t0、不能定发生在故障内或归因。f4i3/3、f4s3/3（最长8.4/8.5/9.9秒）不抹掉该红。完整报告/source/control/字节/hash已核，classification与全部失败journal留证。

183rd turing-20261109T160336Z-88cf4c78 **N0真实Node v4校准1/1**，baseab2b939b/treef8336c08/snapshot4c56003b/control5c67b39，99.22秒。五请求10事件完整配对：ok2/error1/timeout1/cancelled1；SIGSTOP状态实核、200ms期限201053us、已poll后取消52273us、CONT后成功17367us。独立consumer远端valid，本机仅Python按实际原始事件11完整性/负例检查通过。非延迟/能耗/SLO通过，取消不代表远端执行停止。

184th Mac42651079158同183完整source/control，只有assert一hunk的**format红**，未编译/行为测试。source/env/transport/全部9 control blobs/hash已核；原patch应用后新准确源再验。184 refs snapshot/tree全批核、当前两端无在途；不复发已终态任务。


185th Mac42654241724，base2747e8d1/treeb461bc39/snapshotfa2cbdd7/control5c67b39，**root fmt/net66+1ignored绿**。184原patch仅assert重排，完整source/transport/9 control blobs/hash已核；不覆盖随后写的lifecycle诊断源码。39633806控制分支已push新增SSH-only netlab-hop-ownership，原f4三轮/所有原门不变、低量生命周期/非SLO；driver2/runner4共6 stub test（含新profile+parser失败子例）绿，无本机产品执行。


187th Mac42655499657同186完整source，root fmt/net66+1ignored/hop9/wires11/journal29/portable20全绿，所有source/control/transport/hash核留。188th Mac42655862010，basef17f07e1/treea56bdd85/snapshotbefe92e5/control39633806，**仅format红**（eprintln一hunk），无行为测试；原patch应用。186前两轮原f4门绿但诊断助手错误要求splice-only M也有端点载路事件，导致采集校验红；不能算产品红。修正模式校验与先保存文件，尚未重验；三轮原任务继续，不重发同run。


186th诊断终态**fixture红3次**，458.77秒；三个f4结果行绿，但M日志assert拦住末尾原断言，不能记原门已过。73/73 logical_request完整、全部ok，六x/y日志实核samples=off且仅attach＋committed，无释放/结束；M文件缺失已解释并修正。所有source/control/bundle/group/hash采核。190th turing-20261109T163727Z-f72bdaf4修正后独立诊断已launch PID774320，同189完整snapshot/control39633806，不含后续所有权修复。

189th Mac42656499764 **真实旧watcher所有权policy红**，base656e274e/tree70acbbc1/snapshota13dce7d/control39633806。root fmt/net66+1ignored过，hop9过1红；两个当前owner控制、old ended/new live/different serial/same via/never lost全部先通过，再实际release尾部关闭新link并错误lost。raw alive_after=false/lost_after=true；整fixture SHA f620fc97...完整留证。不是format/fixture红；未跑wires/journal/dial，不等同182原f4因果。下一版只限定实际释放需当前serial，测试逐字不改、原计时/quality门保持。


192th Mac42657406850（191同完整source）仅helper observer一hunk **format红**，无编译/行为测试；原patch应用。191th turing-20261109T164022Z-76b5cc1c实际remote无execution/launcher，从未launch，已superseded_not_launched，全部inputs/ref保留。190独立诊断第一轮真f4 after-answer门红（1.6s>1.5s、最长请求5.0s、无回退），修正的日志采集已过；后两轮继续，不放宽原门、不冒称本批owner修复因果。


190th turing-20261109T163727Z-f72bdaf4终态**0/3**，663.76s，完整source/control/bundle/group/hash采核。三红为1558ms>1500ms、180s未达carried前置、准备助手在前置丢失后仍返回Relay。40/40逻辑请求全部ok（21/0/19），零请求不是零延迟/验收绿；九个诊断文件保留。四次释放均属当前committed serial；第三轮释放发生于t0前81.4/54.8s，证明准备helper忽略until失败/非连续持有，不是189stale owner重现。完整时间线见full-round-timeline/current-owner-release-trace。

194th Mac42658228868 **同189整fixture实际红→绿**，base d077d020/tree49b1aeb7/snapshotb4eef0cc/control39633806，与193完全同源。root fmt/net66+1ignored/hop10/wires11/journal29/portable20全绿（49.93s）；原fixture SHA f620fc97...、所有实际CLI fixture/netlab.sh/netpath逐字同，proof stale-owner-red189-green194-fixture-identity.json。193th turing-20261109T164452Z-f0c1a5db已在190完整采证后launch PID798928/child798963、actual /proc/driver核live；Linux24待，不冒称N1/WAN/手机完成。


193rd turing-20261109T164452Z-f0c1a5db **SUCCESS**，219.72s，与194完全同source/control。root fmt/net66+1ignored/hop10/wires11/journal29/Linux24全绿（92.02s），完整source/control/bundle/group/hash采核。189整fixture SHA f620fc97...真实红→Linux/Mac同源绿，proof stale-owner-red189-green193-194-fixture-identity.json；194 refs snapshot/tree全批核、无在途。不替代当前owner质量策略、f4前置/恢复门、N1/WAN/iPhone/能耗。

193/194后新工作副本仅修改f4准备helper为held_precondition_v2：双方reported carried连续10秒，掉路重置；原两60秒等待改同总120秒绝对准备期限，过期前置失败并不注故障。记录phase，故障/恢复/稳定门不变；尚未远端编译/格式/运行，不补写旧结果。


195th turing-20261109T165750Z-d0af45b2已在193采证后launch **PID808786**，actual /proc/driver核live；准确base467336c2/treecd986018/snapshot503781bc/control39633806，held_precondition_v2独立三轮诊断，非SLO。196th Mac42660197395与195完整source相同，**root fmt/net66+1ignored绿**（0.40s），完整身份/transport/9 control blobs/hash已核。196 refs snapshot/tree全批核，唯一在途195；新CLI准备助手实际结果待，不重发同run。


195th held_precondition_v2终态**3/3绿**，491.06s，准确源/control同196。真实连续hold10444/10460/10455ms后注入，原恢复/稳定门实际过；73/73请求均ok（23/25/25），最大4.043195/3.390784/3.332251秒。完整source/control/bundle/group/九三角色诊断/hash采核；只证明正向准备校准，负向reset/timeout未跑，不是N1或整体SLO完成。

之后新fixture hop_reference_v1与control614c266仅准备同三节点几何的automatic/relay-only/carried-only ignored稳态参照，数字/配置/两方向负载已先冻结ACCEPTANCE；controller6 Python tests为stub绿，尚未远端编译/跑。普通f4几何/hold/故障体纯提取复用，不改政策；正式A/B仍需同fixture重建baseline。


197th Mac42663152585仅新reference.rs **format红**，无编译/测试；完整source/control/transport/hash已核，原patch应用。原195几何/hold体提取身份已留hop-reference-default-fixture-extraction.json，运行仍待。N4第一次macserver只读SSH握手timeout留证，GH通道正常；不将访问通道问题等于主机/设备缺失，计划沿Mac主动配对隔离Turing节点通道继续。


198th v1参照终态**6/9**，587.30s，准确source同199、control614c266。375started/375finished=372ok+3error，三次carried首准备请求错误且0稳态样本；auto/relay各3/3、两方向30个summary与raw逐项一致，隔离配置/实际nft表核留。完整身份/bytes/driver/group/hash采核。199th Mac42664955779同源fmt/net66+1ignored绿，不代替198实际校准。当前v2先保留bootstrap通道、等正确M的hop开路事件再隔离，三组至少60s发现期、总准备仍120s；控制7598180元数据显式v2，6 Python stub checks绿，尚未远端编译/执行。


200th Mac42668241618仅bootstrap消息一hunk **format红**，root尚未编译/行为测试；完整source/control/transport/hash已核，原patch应用。198三次carried准备journal没有成功开路事件，只有约10秒ask拨号失败；设备表可见不能当作载路就绪，v2校准该前置。200 refs完整批核、当前无在途，待新准确source实际验证。


用户曾暂停后明确继续；18:07 actual/proc证实201仍live，暂停未让已launch远端任务停止。201准确base9c2e7348/treedcad2092/snapshotd862e1ae/control7598180，v2首轮3/3，第二轮automatic已完成；整体未终态。202 Mac42671621101同完整source已dispatch。原request06db4946 upload_source_bundle在HTTPS SSL timeout后、dispatch前退出；核source ref正确、bundle ref404、无workflow run后，沿同request补bundle并只dispatch一次，未重发一个在途workflow。202refs snapshot/tree双引用全批核。


### 201｜2026-11-09 hop_reference_v2完整校准9/9

Turing `turing-20261109T175508Z-912bcc0c`，base9c2e73488723fc550d335883e96eba78e08a8f7e / treedcad20923717cf99207f16a53c3aceed7b1b2b93 / snapshotd862e1ae80cedefedec79b8e57992385f85f46b3 / control7598180ab85a8253ecc634eabe66b8552e31d471，1228.41s、原group清空。9/9、576/576全ok；raw重算所有n30的p50/p95/max吻合，准备60秒+最后hold10秒、bootstrap先于阻断、两端真实nft隔离与peer-hop关闭均核；source/control/bundle/ref/report hashes核。证据reference-v2-calibration.json与ops/calibrate-hop-reference.py（数据检查，不执行产品）。第二轮automatic189.106/173.863ms，fixed-carried145.221/145.701、fixed-relay190.427/193.093；同wire carried短暂出现再失去Custom，需诊断定因。通过仅代表稳态可用性和固定路参照有效，非最优选择/完整N1/SLO/WAN/能耗。

### 202｜Mac42671621101，两次GitHub内部错误，无job

同201完整source/control，request06db4946b3854c5d8d300dc247958846。attempt1、一次同run rerun attempt2均completed/failure、jobs=[]/artifacts0；原生Chrome GitHub Summary明确Internal server error，correlation分别8f4c3a68-b894-489d-aa05-f583f2a54e85与10c5e6b4-5407-4563-b375-5da3017696d6。API元数据、UI注释转录与无产物记录在mac-hop-reference-v2-ready-42671621101。没有远端source执行/format/编译/测试证明，不计产品失败；不第三次重发，等服务恢复补验。上传SSL timeout的dispatch前恢复与同request回执另留201/mac-upload-timeout-note.json。


### 203–209｜继续执行与开发缓存

203仅stage时捕获调用cwd导致root checkout快照，remote无report/launcher，保留并supersede；未执行。ops snapshot已固定产品cwd并核base，后续control SHA自动规范40位。204 `turing-20261109T182656Z-273ef96c`同205源d90bf666/tree9a7bd3d0/snapshot1a501da8/control32fa23c，219.90秒、root fmt/net66+1ignored/hop10/wires11/journal29/Linux24全绿，原group清空、全身份hash核。205 `…183013Z-fdf5424c` automatic lifecycle三轮3/3、192/192全ok，416.55秒、9诊断files与原group完整核；无当前owner释放/close，未复现201，不算因果收口。

206/207同205完整source、control9135b6d4311ce74ee3a90dac188f4b094f780e35，稳定源码缓存首次/复用，两次fmt/net66+1ignored全绿。构建17.05→0.41秒，总51.5156→35.8381秒，实际net tests11.79/11.93秒，jobs16保持；pair proof turing-source-cache-pair-206-207.json。206输入短SHA原样保留，执行前已用canonical proof/ref核全SHA与所有control字节；207全SHA。源缓存仅Turing owned root原flock内使用，未知owner/脏tracked/软链拒绝保留，未改installed policy；Mac原隔离checkout路径保持。

208 `turing-20261109T184729Z-59b8eed7`同source，追加原automatic lifecycle三轮（PID969908）；209 Mac42677483225/net-node同source、requestb2262c13…、实际control9135b6d，已in_progress。结果未预记，按原句柄继续采证。


209 Mac42677483225已终态全绿且完整身份核：fmt/net66+1ignored/hop10/wires11/journal29/portable20（49.93s），同204/205完整source；不覆盖新210通知合同。210已stage未启动：`turing-20261109T185254Z-88fdeca5`，base16a0bafc/treee6aebe0c/snapshotcbbbac6f/control9135b6d。完整210 refs snapshot/tree已逐run核，唯一在途208/未启动210，后续以实际句柄为准。


208已终态3/3、192/192全ok、435.58s、原group空，9诊断文件/phase/summary/raw核；third current serial1两次比较后被释放并关闭，later serial2重建，trace留measured-release-trace.json。不是stale-owner，不直接认定201因果；质量/age/samples的原值必须带着分析，不能只据结果行green算最优。

210实际Rust合同红：fmt/net66+1ignored/hop10/wires11过，journal29过1红；118.09秒、原group空，source/control/bundle/ref/report identity核。全fixture SHA d66be0e62f349d2298d666c8470e09403f4bee6b53832d9e2fbf9f839315a822。新测试首次IPv6变化iteration失败，IPv4变化iteration未到；是实际生产Interfaces/interface_addrs状态机反例，非物理地址重编号。当前候选仅相同seen集合不重置timer，原1秒与整fixture不动；等待Linux/Mac原合同转绿。


211/212准确功能10ae1ca0已同源全绿：tree00ca0b6f31322d551402610b7b38d5dd8a7e48da/snapshotaa259f3d1376aa9a2e96a0cb257d336eb20a57e6/control9135b6d4311ce74ee3a90dac188f4b094f780e35。211 Turing206.47秒、原group空，fmt/net66+1ignored/hop10/wires11/journal30/Linux24（91.86s）；212 Mac42679667230 fmt/net66+1ignored/hop10/wires11/journal30/Mac20（50.06s），完整source/transport/9 control blobs/hash核。210整个新fixture逐字不变，整生产journal文件仅四行guard，proof stable-interface-red210-green211-212.json；green两地址族iteration均执行，red首次IPv6 iteration失败。不是实际重编号/真跨网/能耗证明。全部212 refs batch核、无pending，queue空。


213 Mac42681901453，base34679e65/treea4bf836c/snapshot3aec9348/control ea97dd4a：新IPv6物理校准fixture只有format红，无编译/物理执行；完整source/transport/9 control blobs/hash核，原patch应用。两Public router显式DualStack后新准确source再先Mac预检；原source ref/input保留，不计产品缺陷或物理失败。


214 Mac42682546955：base89725161/treeb1ed82c5/snapshotba0e5d7f/control ea97dd4a，root fmt/net66+1ignored绿，完整source/transport/9 control blobs/hash核，无CLI物理执行。215 `turing-20261109T192606Z-22c24322`同完整source/control f5838d7d5398da99786108face83bd8f204d8c35，75.90秒、原group空，fmt/CLI编译/物理校准1/1。实际IPv6 `2001:db8:1:1::2`→`…::42`同/64，三kernel snapshot与8packet（7正1预期不可达）逐项核，IPv4 JSON相同且仍通；cache事后tracked clean。proof ipv6-renumber-calibration.json。地址ready不独立证明DAD流程；不是Node/QUIC/前缀变化/IPv4 NAT/手机/能耗证据。215 refs snapshot/tree全批核、pending空。

控制f5838d7补format失败路径缓存保护：本机实际小Git＋Python compiler/formatter stubs先见cached源码被format改动红，保持原整fixture后改为一次性副本出patch，bytes/mtime保持并原8控制检查绿；无本机Rust/产品执行。新mobility profile在Linux编译前格式预检，旧timed profiles不加新工作。
