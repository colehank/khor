# feat/net 当前状态与接续入口

更新于 2026-11-12 09:44 UTC。用户暂停后已明确“继续”，执行已恢复；用户随后澄清“我没有验收 打错了”，没有新Lab/真机验收结果，完整目标“完成计划”**未完成**；范围仍为[PLAN](PLAN.md) N0–N5，root串行，不派发子智能体。
暂停前后完整原STATE逐字保存于[历史状态](HISTORY-20261112-pipeline-start-and-pause.md)，SHA256 `9a774440c70f3c3ca851b37aadcce9a867a2a22ca9bb687016a1091cf7856948`；该历史继续链接更早完整证据，不按摘要覆盖未提交内容。

## 当前执行与下一步

**当前唯一在途：真实CLI取消合同前置/执行 turing-20261112T094221Z-91240bdf。** source02807c79 / treef87d31f9 / snapshotdc21d783，control **21b3990597853288b44f1bfef75a4433fa804905**，tasknet-node/revisioniroh14_actual_mcp_workers_v1，pid1489353。目标只补两个先前因缺CLI失败的MCP/remote-cancel命名合同：同源CLI/node test二进制按scripts/native_provenance.py prepare/verify（CLI prior→node）构建；借用源码run-node-acceptance.py的PID/PGID所有权与清理run/require_test_success，每用例独立进程，控制Python sidecar，**不是完整真实agent bundle验收**。已读现有Turing conda Python3.13.13，固定二进制SHA c46ed00dd978dbb20e32eb34d9e16660775b2ed4fc84395a73120804e3e66379，任务内临时wrapper用-E -S，系统环境不改。尚不记编译/合同过，先查询此run，禁止重复调度。

**完整node诊断前后名单完全一致。** 旧源 **turing-20261112T092140Z-5216816b** 原101/636.509801s、12原files完整核验；与新源090450Z-bfc82a31同control340b3243、同1387人口、同75ignored，均1274过/38失败；new-only与baseline-only失败均空。E/iroh-upgrade-full-node-comparison-v1.json，原两个红轮保持；不能据此豁免原失败或接受完整N5。两项明确缺KHOR_TEST_MCP_EXECUTABLE；另外两项账户是runtime状态失败，不能仅按消息唯一归因“未安装”。当前新run只补前两项，其他Quality前置/原失败保持。

**Noq2退出同样证伪。** **turing-20261112T093340Z-deb1f7de** / source79d2ca64/tree373767ae/snapshot25fdf0c0 / controlfc6fe1d1，原101/37.609123s、12原files完整核验。保持双方both、服务端尚未提供握手响应的相同原fixture，移除两道生产保护后ObservedAddr TX=1而应0；原Noq1.4基线467中同命名过。fixture及锁字节不变，仅mod.rs改；noq14-zero-rtt-exit-verification-v1.json与E/iroh-noq-zero-rtt-exit-comparison-v1.json。保留Noq2，ablation不晋升。账目v4：1项上游替代库验证、2项实证保留、57历史条目待逐项退出核查。所有前述run ledger已终态，只有当前094221在途。

**历史：完整node旧源对照启动（现已同红终态），稳定升级未晋升。** **turing-20261112T092140Z-5216816b**，source **dc73837e** / tree dabe98b2 / snapshotfc610303，沿与新源相同control **340b3243275f961baf2b8c50074908edcf51ede5**，role baseline、1387全库零skip/单线程。先核查现有run，禁止重复提交。新源02807c79完整node原run **turing-20261112T090450Z-bfc82a31** 已终态101/552.029829s，1274过/38失败/75原ignored，12原files/identity/SHA/原group空，stable-upgrade-full-node-failure-v1.json；不记全库接受。嵌套long_load_worker子测试footer保留，不加到1387根人口。20项直接观测Git不识别initial-branch=main；Turing Git2.25.1/tmux3.0a。两项MCP缺KHOR_TEST_MCP_EXECUTABLE；两项账户报runtime missing仅为状态报错，尚不据此唯一归因。当前full-node driver没准备Quality要求的同源CLI、真实agent bundle/运行时/provenance与分进程acceptance，后续须按quality.yml与scripts/run-node-acceptance.py补前置，不能改产品迎合未备齐环境，也不能将38项全归上游。

**Noq5退出已真实证伪。** Turing **turing-20261112T091835Z-cb319e57**，sourcee69b137c/tree78b6f96e/snapshot50a79b50，control958dc554，原101/33.786267s、12原files完整核验。4原fixture三过一红：reclaimed合法已分配发送状态ACK误使样本计数5→6，其余保留最大号正常采样/ACK-only/未发送号拒绝均过。两份fixture与锁相对028逐字不变，仅mod.rs移除patch5保护；baseline新Noq1.4全467中的同四命名均过（Mac/Linux）。noq14-ack-exit-verification-v1.json及E/iroh-noq-ack-exit-comparison-v1.json；保留Noq5，ablation源不晋升。迁移账目v3：1项上游替代库验证，1项明确保留，58项仍待退出核查，不把其全部写必要。
Noq2退出树 **79d2ca64** / tree373767ae / snapshot25fdf0c0，E/iroh-noq-0rtt-exit-source-v1.json，controlfc6fe1d1，仅准备未执行；待当前baseline终态后Turing执行（Mac仍有50GiB门红，不重发Mac）。

**历史：升级应用固定回归与完整node启动（完整node现已红终态）。** source02807c79/tree f87d31f9保持不变。Turing固定12 **turing-20261112T085339Z-ab57a92e** 原0/385.723121s、21原files，273过/identity辅助ignored1，所有命名人口相对旧整合源逐项一致；stable-upgrade-node-suite-verification-v1.json。真实流合同 **turing-20261112T090144Z-223b1901** 原0/31.781419s、31原files，36命名/4准确frozen metadata重算/独立74Fetch RPC及10peer，stable-upgrade-transfer-verification-v1.json。两lane人口重叠，不相加。
Mac库门 **43186999295** 已成功核验16原files/11controls/ZIP f8cbf61034871f8591f02bbbea4ff58953ac8c0974ade643cdec2a8278c48024；Noq467/iroh197及原flaky ignored1与Linux命名人口一致，node lib/tests编译过、三锁一致、post-check diff空。E/mac-iroh-upgrade-43186999295/stable-upgrade-gate-verification-v1.json，ledger终态。
当前完整Linux node库：**turing-20261112T090450Z-bfc82a31**，source02807c79/snapshotdc21d783，control **340b3243275f961baf2b8c50074908edcf51ede5**；先列出1387测试，零skip filter、单线程全库，在途，尚不记全库接受。

**补丁退出对照转Turing，尚未启动测试。** Noq5仅删保护的隔离源 **e69b137c** / tree78b6f96e / `experiment/net-iroh-noq-ack-exit-v1`，两份原ACK fixture字节不变。Mac **43189070840** / request2b678f44a26c4cbe9e801414ab152971 / snapshot50a79b50 / control **958dc5545d2894ef5e448a669cef4f5588b82bab** 被50GiB磁盘门拒绝，零产品测试；5原files/ZIP 4f7ca2212b4ad3968ec775e54e747c6e9f1be01ea0061f6352984a6d5891c981、11control与bundle transport核对，缺source/environment执行证据，identity/behavior均不记接受；disk-admission-failure-verification-v1.json，ledger终态。不改余量门、不清缓存。待完整Turing库终态后执行同一ACK退出对照。另Noq2移除保护候选79d2ca64 / experiment/net-iroh-noq-0rtt-exit-v1仅准备，原双方both resumption fixture不变，control fc6fe1d1，未提交。两ablation源不晋升，稳定版候选始终保留Noq2/5。

**历史：稳定版库门完成、应用回归启动（现已终态）。** 库门 **turing-20261112T084724Z-35d6dfe3** 原0/226.384657s、17原files完整核验；Noq467/iroh197过、iroh原flaky ignored1，node lib/tests编译过，三锁与冻结源一致，post-check diff为空。stable-upgrade-gate-verification-v1.json，root ledger终态；新增[升级说明](IROH-UPGRADE.md)及E/iroh-upgrade-patch-ledger-v2.json。补丁22源码已用上游且库合同过，59项仍待逐项审查，不把pending写成全部必要。
当前Turing应用固定12：**turing-20261112T085339Z-ab57a92e**，同source02807c79/tree f87d31f9/snapshotdc21d783、control68cadb57。Mac同树库门：**43186999295** / request2bbd0ca023644131880d97030df1e8a9，snapshot82206b8f、controlbd13e1e7；E/mac-iroh-upgrade-43186999295，bootstrap过，driver在途。两者独立host，未并发Turing计时bench。全N0–N5及真机未闭。

**历史库门启动（现已终态）：turing-20261112T084724Z-35d6dfe3。** source **02807c79** / tree **f87d31f9** / snapshot **dc21d783**；control **bd13e1e7f8b2f12a6089978a69180d108eec4f14**。根锁及vendor锁均已确认iroh1.4.1、Noq1.4.0；保持原准确源码，Noq全lib→iroh全lib→node lib/tests编译，尚无新源行为结论。

准备轮 **turing-20261112T071440Z-2f31345c** 原101/226.706692s：已生成锁/格式patch，Noq stats重复导入FxHashMap编译红，零行为接受。19原files/SHA/source/bundle/原group空核验，dependency-preparation-verification-v1.json；准确patch c719b20fe841b3df6e453a91e5a567b2c01c1eb52a709ae0a4ec462c81a06d01应用到候选并删重复导入得到02807c79。
原首gate **turing-20261112T072057Z-c53a26f8** 原1/0.645742s，准备轮修改了共享source cache，materialize保护拒绝、零测试；2原files身份/SHA/原group空及cache-guard-failure-verification-v1.json，整轮失败保留。root在独占build.lock下核对旧HEAD/tree与完整patch SHA，将source和owner marker整体改名保存在原准备run/work/preserved-preparation-source，未删除/reset任何内容，E/iroh-upgrade-cache-preservation-v1.json。恢复空缓存后才提交当前gate；后续准备control **94b1a637f51d272c4472d28d3ef0af435a70b70b** 改为一次性clone，bash -n过、未提交运行。两原run ledger均终态；当前仅上述gate在途。

**历史：iroh升级候选建立及首准备轮（已终态，见上）。** `/Volumes/disk0/Khor/worktrees/current/khor-net-iroh-upgrade-v1` / `experiment/net-iroh-upgrade-v1`，source **60620d0e** / tree **aa767462** / snapshot **a6644dc7**；control **54cfeb034c6bd17e8d9a046c9ddaf571eb32a2ae**，run **turing-20261112T071440Z-2f31345c**（pid1326978）。仅准备锁/格式并诊断编译，输出patch必须应用后再冻结验收，不记当前原源码行为绿。已校验发布archive SHA，完成三方迁移7处冲突；保留旧新增函数名iroh158/Noq33（仅语法存在证明）。补丁22改上游IPv6 QAD，待新源测试。60项初始迁移账目E/iroh-upgrade-patch-ledger-v1.json不把pending写成全部必要。candidate说明见其IROH-UPGRADE.md。canonical仍原依赖/运行时，原24性能源不变。

**最新用户决定U18：实施最新稳定版升级，优先减少custom。** 已重新确认官方Latest为iroh1.4.1，配套Noq1.4.0。下一步从已验整合候选dc73837e建立隔离升级树，复用已校验的发布源码，先盘点最小应用接口与补丁专属合同，再逐项迁移/退出。原性能冻结树、原红项与N0–N5范围保持。尚无升级候选构建或手机验收。


**2026-11-12用户要求检查iroh更新与官网多连接模式，已复核。** 三个源码分支仍是vendor iroh1.2.0/Noq-proto1.3.0；官方Latest仍1.4.1，依赖Noq1.4.0。以前UPSTREAM已记录版本但没有实施升级，今天新增[版本与多传输审计](UPSTREAM.md#2026-11-12版本与官网多传输复核)及E/iroh-version-transports-audit-20261112.json。
已确认khor已接custom transport用于SSH/peer hop；BLE/Tor/Nym需独立集成且实验性，不据首页宣称默认启用。建议提前隔离升级基线/补丁语义对照，但本次没有改依赖/vendor、没有新构建/部署；原N0–N5及所有未闭项保持。

**原24场景已全部终态并复算。** advance 1–12唯一完成，不重发。[结果与决策](PIPELINE-RESULTS.md)完整记录；全计划未完成。

**Turing三次诊断已终态**：`turing-20261112T062130Z-c432d9c1`，原0/210.563419s；仍用宽2 rolling准确冻结source2438172c/treeada3cd2a/snapshot204945db，control **8b8620ace473657c10ba2cd1bc062c1dd015a058**。固定2 guard off→low诊断3fresh/lifecycle，E/terminal-tail-diagnostic-protocol-v1.json在实施前冻结；未混普通24/未重试挑绿。25原files/2control/原group空、source/bundle/hash、准确frozen control内Python与frozen source observer重算、独立2160人口/6resident日志hash和mode及ledger终态；terminal-tail-diagnostic-verification-v1.json。
三次终端service最大84.365/87.069/83.094ms，未重现原长尾；没有修复/唯一因果结论，原block9问题保持，不能因这三次通过而关闭。Turing当前无在途。
既定24场景已收件核验24场景（12blocks），21文件完整、3个high文件原240秒超时；**17280交互全部ok**。每block两profile各720，准确冻结metadata/独立人口与时间戳/17原files及hash/identity/原group空/ledger终态保持，超时不冒充文件完成或verified rate。

| block | 宽度/arm | run | 原结果 | low/high文件秒或attempt |
| --- | --- | --- | --- | --- |
| 1 | 8/batch | turing-20261112T041552Z-e46a2819 | 0 /345.480199s | 18.737180 /84.314138 |
| 2 | 8/rolling | turing-20261112T043119Z-eb6c2320 | 0 /327.143174s | 19.548505 /76.611212 |
| 3 | 8/rolling | turing-20261112T043815Z-c69a3018 | 0 /280.823312s | 18.867576 /76.625635 |
| 4 | 8/batch | turing-20261112T044402Z-04628dd6 | 0 /335.025722s | 18.765883 /85.975258 |
| 5 | 8/batch | turing-20261112T045513Z-6ec94f5a | 0 /284.840748s | 18.808332 /82.563249 |
| 6 | 8/rolling | turing-20261112T050117Z-b72c7a40 | 0 /308.945589s | 18.791625 /61.708015 |
| 7 | 2/batch | turing-20261112T050750Z-7f2df1ec | 101 /489.697986s | 19.614184 /240.001025 timeout，未完成 |
| 8 | 2/rolling | turing-20261112T051657Z-96f322fb | 0 /419.470064s | 18.482548 /170.928677 |
| 9 | 2/rolling | turing-20261112T052628Z-e17ce649 | 0 /438.903209s | 18.776145 /181.190526 |
| 10 | 2/batch | turing-20261112T054019Z-099afff5 | 101 /487.764855s | 19.571980 /240.000902 timeout，未完成 |
| 11 | 2/batch | turing-20261112T055818Z-dbf52df1 | 101 /445.603326s | 19.650718 /240.001489 timeout，未完成 |
| 12 | 2/rolling | turing-20261112T060625Z-9b6a4cbf | 0 /423.610877s | 18.576697 /173.916041 |

block9 low idle_before实际延至bulk start20.002024s（不是常见12秒），terminal overlap117；原门≥30仍过，全部720人口保留，不删除该run或将调度停顿改成网络因果。host load_after9.73/free473999843328仅观测，未证明唯一原因。


宽8完整12cases/8640交互/102原件hash与全部准确冻结metadata、独立人口重算已保存 **E/pipeline-width8-comparison-v1.json**，由E/ops/summarize-pipeline-campaign-v1.py --width8生成，未选winner、不声称显著性/p99/N3接受。
low batch文件18.737180–18.808332s、rolling18.791625–19.548505s；high batch82.563249–85.975258s、rolling61.708015–76.625635s。high文件对应位置三次均rolling较快，但bulk终端service p95对应位置两次更慢、一次更快；目录与计划延迟也有取舍，不以文件速度定默认。

campaign **E/pipeline-campaign-v1.json** 保留原pause/resume记录，用户“继续”恢复running；先8后2、各ABBAAB、每block真实w3 guard2→low1/high1，总24，旧prefetch不作baseline，原期限/失败门保持。无新人工Lab/手机接受。
完整汇总E/pipeline-comparison-v1.json SHA **14f62be1fafaaac2ae57fdb61ee892d50a8f3860c94d08b47d302af966279c90**：12block/24case/17280交互/204原files SHA、全部准确冻结metadata与独立人口复算。宽2 high batch三次均原240秒超时，rolling三次170.928677–181.190526s完整；宽8结果仍见原独立汇总，未复写。

**不可隐藏的尾部**：block9 low终端bulk service p95775.951ms、scheduled-to-end p957078.141ms；idle_before/idle_after也分别有8.122/9.168秒scheduled p95。原720均ok，原功能门过不代表UX达标。本地20ms probe三个阶段最大lag2.498/2.465/4.673ms、无cap，不能据此排除resident/host/PTY/网络；未证rolling或CPU唯一因果。E/pipeline-block9-terminal-tail-v1.json绑定原SHA逐条复算。保留该轮，下一轮独立诊断如顶层。宽2不设全局默认，宽8 rolling仅作后续整体验证候选；canonical仍原batch8，尚未集成实验代码。root为source-refs.json唯一串行原子writer。

**Mac固定12已完成**：**43153603327** / request36ef1faf8c2c4a7aa5c16cbc91e57e58，sourcee5527f93/tree78bb8acf/snapshota5e2b8c3，control **68cadb57a0ed72d40a4ca9a843e86b9c6797bf34**。
artifact13915416046、ZIP **28ca97ce700577496fd9024916e58225a228061cc15912a561257b96402a8262**，E/mac-pipeline-node-43153603327/（原件在artifact/）。20files/11controls/完整原footer/hash/source/bundle身份与ledger终态，node-suite-verification-v1.json。
net68+原identity创建worker ignored1、hop89/wires16/journal30/budget35/IPC18+六single全部过。budget包含三新pull真实RPC测试；此固定lane没有nocapture，成功RPC原marker未保留，Mac只接受三命名Rust断言，独立计数证据仍在四个Turing合同run。不扩称Mac四Window逻辑、files_wire、全部Node或N5；旧Finder红项不豁免。

**取消修复隔离树**：`/Volumes/disk0/Khor/worktrees/current/khor-net-pull-cancel-v1` / experiment/net-pull-cancel-v1，HEAD **f3ccd2f80e6da097ab3d97d5155fe0facee7277c**，clean；未集成canonical。生产修复在31720fa6，只改road.rs/link.rs与两个通知unit：stop/cut设置原atomic并唤醒所有等待者，pull复用caller拥有的通知future参与拨号与每片等待；无新timer/task/RPC。冻结性能四树保持原版本，不能把新取消修复冒充该性能源。

- 原格式红 **43156306374**（8d44285b，九原files，零行为）准确formatter patch映射87e9356a，format-only-verification-v1.json与ledger终态保存。
- **43156779606** / requestaee09c7e22ad473b8f50d015c4ab6a7b，source87e9356a/tree2203af1e/snapshot5e5eefe9，control **9690e99032d0f651a799313b5c0fec1b1ebb98cb**：Rust0过/2真实红；首片/八片窗口road.stop后502990/503138us调用者未返回，随后fixture drop才收到1/8 STOP_SENDING，同wire控制正常。11原files/hash/source/bundle/身份、准确冻结metadata与独立10RPC人口/phase顺序及ledger终态；job-stop-verification-v1.json。
- **43157963440** / request8e1e456fed304b01bf71dbb818025b6f，source31720fa6/tree0f7fc0c8/snapshot63d815d2，同control9690：同两故障fixture/metadata逐字不改，Rust2过；first/window分别49/329us自行返回cancelled，part/note清理，1/8 STOP且同wire控制通过。仅loopback诊断，不混ordinary时延。11原files/准确metadata/独立人口/原hash/身份/ledger终态。
- **E/pull-job-stop-red-green-v1.json** 冻结两run source/control/三fixture SHA相同与仅road/link差异，真实红→绿；不是生产server、远程DropStop RPC、SCP/host-child、Offer所有取消、最终rename竞态或完整N3已闭。

**相关回归与准确短尾见证**：

- 原 **43158565837** / request90cf5af4814a4d56afe608a896e45f5e，source31720fa6，control **74e6bf9832e34243118c315215a549af8275e4b3** / feat/net-pull-job-regressions-v1。Rust六组共29全部过（road9/drops6/tree4/post3/Window4/真实RPC3），但原v1 metadata把乱序提前返回的最终短片offset3145728/1234bytes误要求整SLICE，整轮failure保持；18原件/身份/hash、原frozen失败复算/独立实际短片与源固定总长度3146962核验，regression-metadata-failure-v1.json及ledger终态。
- f3ccd2f8仅在test snapshot加实际total字段、metadata v2按exact min(SLICE,total-offset)核每片；限定总长度/对齐/范围/重复/内存上限，保留原门。八种参数注记合成副本校验过，原日志不改、零新增网络样本（E/pull-window-final-slice-data-check-v2.json）。生产road/link字节相对31720fa6不变。
- 新 **43159900154** / request5a17596eee744acaa7a852174b510b8e，sourcef3ccd2f8/treee2c417d0/snapshotb9a40bc9，同control74e6；Rust29过、准确frozen RPC v2/独立35真实RPC人口与三peer核验通过。18原files/11control/完整SHA/source/bundle/身份及ledger终态，E/mac-pull-job-regressions-43159900154/regression-verification-v2.json。新两个Notify unit覆盖预先停止与8个已注册后暂停poll的waiter，stop和cut各验证，全部过。
- 新artifact13917950334 / ZIP751e41bd6974d4f59c66617606e1d26b817d75c4d7210c80fa289a640a2c3d37；原失败artifact13917575466 / ZIPdfc6fcd332f73451d0016396d2c25f09ef4be69d8a3962b98dab70786e50faf8均保留。没有Finder、全Node、最终Native/手机、Linux新取消源或完整N5接受。

**真实鉴权Stop RPC隔离测试**：`/Volumes/disk0/Khor/worktrees/current/khor-net-pull-stop-rpc-v1` / experiment/net-pull-stop-rpc-v1，从f3ccd2f8创建；测试/fixture/metadata变更，没有相对f3的新生产改动。真实receiver Node.handle→dispatch，独立已准入controller和未准入stranger，控制文件source持有8片；验证未配对/畸形token拒绝、错误token不停止目标、正确token与重复Stop、8流停止和landing清理。设备表预准入，不声称配对协议或生产文件server/host-child/移动已验。

- control **91d804e74b1a2ce2cbc87f3df0b9b60eef9e12a5** / feat/net-pull-stop-rpc-v1，仅固定一个Stop RPC测试、nocapture/full_diag，准确source-owned metadata pull_stop_rpc_v1。
- 首run **43164711650** / request8ef0779e2f8c4015b2cd0c71acf6aa51，source0476639b/tree460cb041/snapshotede655e2：格式门失败，**零行为测试**。artifact13919103982/ZIPe2f88767643a6c0a2c6beca76f6ecff5d973cb37d592cf81bbc5c18fc80c6813；9原files/11controls/source/bundle身份/SHA及准确formatter patch映射，E/mac-pull-stop-rpc-43164711650/format-only-verification-v1.json与ledger终态。
- formatter后的 **43165197874** / request070fc62c2c8e455aa35ca88e9ebf15c2，source17755002/treeab3f474e/snapshota5f801c6：编译过、Rust0过/1失败，fixture只upsert设备表，未加入signed roster，首次admit断言失败，零Stop/Fetch RPC。11原files/11controls/准确frozen失败复算/hash/source/bundle与ledger终态，fixture-admission-failure-v1.json；不是产品鉴权或取消回归。
- source **73b0d900a8f791115b7ccf7ff5412c88fb129105** 只在fixture用现有enroll_after_pair将controller签入receiver roster，不改产品门。**43165801580** / requestf9bdce8888424991a3833ba66c077d2b，tree19332f21/snapshotbe3a5e97，同control91d8：Rust1过，准确frozen metadata重算、独立14条实际request stream（9 FetchPath/5控制）、先负控后完成、9已发/8STOP、同连接存活/landing清理均通过。11原files/11controls/SHA/source/bundle/ledger终态，E/mac-pull-stop-rpc-43165801580/stop-rpc-verification-v1.json。artifact13919652865/ZIPf544f91d6e145fcf03e7726afd6dd8e5d4bc58e97dad92709db3ea8da21ffea1。生产link/road相对f3字节不变，不声称pair协议/生产source/host-child/移动验收。

**新增Offer停止遗漏**：静态复核发现link::stop_sent两处直接写job.stopped，绕过新Notify；pull_one仍join_all整批Fetch，没有等待stop通知。此为未闭入口，不能由上面Path/DropStop绿推导Offer已过。隔离树 `/Volumes/disk0/Khor/worktrees/current/khor-net-pull-stop-sent-v1` / experiment/net-pull-stop-sent-v1，从f3建；原测试源98a3d3c4，格式修正90a70a61；最新 **8de4511ab431dab8e8d12ea75e624d5761607b53** 仅生产road/link加可复用通知入口和Offer借用等待select，未集成canonical。

- control **76e71b4f59f7a6216a756caf785c01fe29ef3bb1** / feat/net-pull-offer-stop-v1，原9690仅net-node分支固定四个offer_stop测试/full_diag/nocapture，其他任务/资源门不改，CONTROL HEAD不动。
- **43166380936** / request4a5e90ae301d4b6e86e2038cd04cb1c4，tree3685e137/snapshotd3963043：原formatter红，零行为；9原件/11control/identity/SHA/准确patch与ledger终态，source90a70a61精确应用。
- **43167054406** / request7c3e9796e8644ed2855e7386cafb5b39，source90a70a61/tree470040ef/snapshot33e7c8e6：真实Rust0过/4红；四caller停止500696–502453us仍pending，后fixture drop才收到1/8STOP，同wire control正常。11原件/11control/准确frozen metadata/独立20个已发Fetch人口和先caller后cleanup/identity/SHA/ledger终态；offer-stop-verification-v1.json。artifact13920582350/ZIPaa9606e02e5e083516627998b36ab8b097e09f2656eefa01f99fe53ea94f1a55。
- **43167824001** / requestfaabef6ec75544d68cd12a18b68ede1a，source8de4511a/tree6e741650/snapshota61d93c2，同control76e7：Rust4全过，stop first/window57/378us、cut first/window58/244us调用者自行返回，marker退役/partial原长度、1/8STOP和同wire控制正常。11原files/11controls/准确frozen metadata/独立20 Fetch人口及phase顺序/source/bundle/SHA/ledger终态，offer-stop-verification-v1.json；artifact13919884421/ZIP334a11be661c05a12ed41037551c73da4581877a1e8337ad5693826a69da98e1。
- E/pull-offer-stop-red-green-v1.json绑定四fixture/metadata逐字相同、同control、仅生产road/link差异；不冒充run_take完整清理或全N3。

- **43167824001** / requestfaabef6ec75544d68cd12a18b68ede1a，source8de4511a/tree6e741650/snapshota61d93c2，同control76e7。Job.request_stop仅改crate可见，stop_sent两处改用它；pull_one一个borrowed stopped future参与原join_all，不新任务/timer/RPC，不改Offer调度/原partial与cut规则。真实pull_offer/pull_one分别在首片/八片挂住时，经Node.stop_sent（running Job）或Job.cut取消；500ms内调用者仍保留，后续fixture drop单独观察。控制source是真实QUIC自建responder，partial必须原长度留给run_take owner、marker在自行返回时退役；不声称run_take完整清理、配对或生产source合同。原红已保存，绿源四fixture/metadata字节不改；四项实际红→绿已闭，其他范围仍待验。

**隔离整合回归**：`/Volumes/disk0/Khor/worktrees/current/khor-net-transfer-integration-v1` / experiment/net-transfer-integration-v1，HEAD **dc73837e5bf33238b5cc2ed80ba8719fbb793601**，从canonical068ba267整合U/S。只有共用test Fixture的构造入口冲突，合并为with_options(kind,offered,node_identity)，保留原new/build/with_identity接口；所有named tests/metadata不改。生产link/road与已绿8de4511a字节相同，未合入canonical feat/net，更未main/发布。

新control **69d23d4f912c2e3674d47010eea5b2577230e182** / feat/net-transfer-integration-v1，原74e6固定29项+独立原Job2/Offer4/StopRPC1，**36个唯一测试**，各source-owned metadata分目录；原wire3只增加显式skip三个独立suite以免混计，其他任务/资源门保持，CONTROL HEAD不动。E/transfer-integration-control-v1.json。Mac原 **43168695982** / request22fda3178df34d6c8eb2d3f7678eb6fc，treedabe98b2/snapshotfc610303：测试driver步骤success，GitHub原job131016533263日志九footer36测试全过；但upload-artifact FinalizeArtifact报ECONNRESET，artifact列表为空，whole workflow failure保持。macserver SSH恢复在banner超时，未取得完整report。原job日志SHA **574690eb5d96ea0040e12a6f5735682cd84ed6db648478802f9284d0910f0b98**，E/mac-transfer-integration-43168695982/artifact-transport-failure-v1.json；ledger终态behavior_verified=false，不能冒充完整artifact验证。

仅此传输故障后同source/control重跑一次 **43169808292** / request21b56402aac94251b92383e1895faa65，base仍dc73837e/treedabe98b2，新snapshot2bb6af91：在可信control bootstrap读取阶段JSONDecodeError（Unterminated string），源码未materialize、driver skipped，**零产品测试**。artifact13920659924/ZIP194bd691d6055c1ac30c708aa75c85b46892909cd7cc35cb9ea9895b25548c30，3原files和job131020141739原日志保留，bootstrap-fetch-failure-v1.json及ledger终态，identity/behavior均不接受。未证网络唯一因果；不继续盲重跑Mac，也不把该失败记产品回归。

**Turing同源固定12回归已终态通过**：`turing-20261112T063227Z-f69d52c8`，原0/375.221223s，同source dc73837e/treedabe98b2/snapshotfc610303，control **68cadb57a0ed72d40a4ca9a843e86b9c6797bf34**，task net-node/revision transfer_integration_node_suite_v1；前一诊断收件后才启动，无计时负载竞争。
21原files/2controls/原group空/source/bundle/全部SHA与各原footer/七个取消命名断言及ledger终态，node-suite-verification-v1.json。net68+原identity::creation_child ignored1、hop89/wires16/journal30/budget46/IPC18+六single全部过，共273通过；budget比Mac多Linux条件用例，不按Mac35硬编码。不是全Node；成功dial-budget没有nocapture，接受命名Rust断言，不声称该lane独立RPC人口，Mac36补足。

**Turing同源36整合合同已终态通过**：`turing-20261112T064251Z-04c926ef`，原0/31.735045s，同source dc73837e/treedabe98b2/snapshotfc610303、同control **69d23d4f912c2e3674d47010eea5b2577230e182**，task net-node/revision transfer_integration_wire_v1。前一个Linux固定12已收件后启动，原UID/flock/资源/期限不改。
31原files/2controls/原group空/source/bundle/全部SHA及九footer、36个唯一命名测试、四套准确frozen source-owned metadata重算，独立74个实际已发Fetch/10个source peer人口均过；transfer-integration-verification-v1.json和ledger终态。它补齐Linux完整相关合同，不冒充Mac原件已恢复、全Node、生产source/host-child、完整run_take清理、最终rename竞争、移动/手机或全N3/N5。

**当前所有已提交任务与本地句柄均终态，source-refs.json无pending。** candidate只在隔离整合树dc73837e，canonical feat/net仍是5064运行时代码，之后仅文档提交；尚未集成/发布/变更已装App。下一步仍按[资源归属设计](TRANSFER-LIFETIME.md)补真实server/worker与最终提交边界、定位终端尾部，再扩完整矩阵，不重复已闭的24探索和控制responder取消测试。

原终端秒级尾部未定位，server/host-child/落盘竞态和完整N0–N5仍未完成；不能由本批绿项缩减范围。用户澄清未做Lab验收，真机与Lab均保持未接受。

## 已冻结输入与四真实RPC合同

E/pipeline-source-quad-v1.json：宽8A0c4e4e35/B e5527f93、宽2A ea414bfc/B2438172c；E/pipeline-w{8,2}-{batch,rolling}-source-v1.json完整四树。priority0、A原batch函数逐字不改，B函数逐字来自57ff原型；同helper/逻辑/长files_wire/所有vendor锁，宽度内只有调度循环/必要短读重启差异，跨宽只有PULL常量。共同Python wrapper只标记本轮，沿用原recorder全部人口/期限/失败门。
宽8batch36a4357d、rolling436c6664、宽2batch0a61687c、rolling19528761各四逻辑/三真实合同全过，13原files/准确metadata/独立人口/hash/identity/原group空与ledger终态，各pipeline-wire-verification-v1.json。
宽8慢尾A已发9/客户端解码8/提交1片，B已发11/解码8/提交8片；慢头两者已发9/解码8/提交1片未超开。两宽两armfuture drop均收到全部等待流STOP_SENDING且同wire独立Sessions成功。
这只证明控制responder与原future drop，**不替代生产server、Job stop、host-child或最终N3**。首格式红5f783fc8原1/零行为、原patch准确映射b020a801与原证据保持。

N1诊断静默最新a09450d0原0/339.545858s/29files，十二准备/十二≥10秒静默/十二真实请求阳性见证过；仅full日志诊断准入，不是ordinary性能、手机或完整N1。旧v1静默红、普通静默缺证与全剩余范围保留在下文及历史。

## 工作位置、执行与授权

- PRODUCT `/Volumes/disk0/Khor/worktrees/current/khor-net`，branch **feat/net**，创建基线6485cb83；当前已测代码提交5064fe2a，之后STATE/历史文档提交不冒充重新测试的源码。
- CONTROL `/Volumes/disk0/Khor/worktrees/current/khor-build-net`，HEAD仍 **5d3691f252f9dee0297fa3c494374be6119bca29**。新控制只建Git对象/ref，不移动HEAD；现有两个未跟踪__pycache__保留。
- 隔离旧rolling原型 `/Volumes/disk0/Khor/worktrees/current/khor-net-pipeline-v1`，branch experiment/net-pipeline-v1，clean **57ffe973ed11d03dc76c5fa01904719b27651b36**；未集成、priority仍-1，不能直接与当前priority0比较。
- EVIDENCE `/Volumes/disk0/Khor/evidence/net`，ops在其下ops；canonical ledger `source-refs.json`仅root串行原子写。
- 本机source-only：不跑本机Rust/Cargo/rustfmt/Node/Xcode/产品。Git、Python标准库数据/合成检查、bash -n可用。重型验证Turing与macserver+私库colehank/khor-build；具体控制和收据不得混用。
- Turing原UID1110/flock/user-netns、Rust1.95/jobs16/nice10、50GiB启动/40停止、90分钟保持；不sudo/改宿主网络/在同Turing计时中插入编译。Mac共享guard/jobs2/90分钟保持，可独立重叠。
- 用户已授权“完成计划”实施，旧AGENTS“仅保存计划”不是永久暂停。无本任务main合并/发布、中继采购/生产部署、已装App变更授权；中央GUI任务的ship授权不套用。
- MacLab与Turing同LAN；macserver为另一构建机。localhost是旧0.2.0 iPhone；此前人工反复开关恢复不算候选自动接管通过，旧手机窗口已结束。
- 图谱partial/stale，不覆盖vendor时的0nodes/0flows不是覆盖证明；未建新的CodeGraph索引。不派发子智能体。

## N1首次操作与静默：已知证据及未闭合同

协议 [IDLE-ACTION](IDLE-ACTION.md)。原p7 active10秒/idle20秒门不改。原idle选路恢复16–17秒与15秒备用保活量级接近，只是假设，不证明唯一根因或极致体验达标。

**fixture所有权旧红→修复绿已闭**：

- 旧source e471a2f1、control e9a443c4，**turing-20261112T015902Z-e8569de4**：真实caller drop后device future继续，原1失败/1负控过，清理后才断言，原101/90.034秒/13files。
- 修复 **0a8b90d9986d3426f65b61ae9d994083f1fa3321** 只在LabNode.with由调用者持有AbortHandle Drop。**turing-20261112T020453Z-b9c5278d** 原0/53.827秒/两项全绿/13files。
- 两份idle-action-ownership-verification-v1.json：guard逐字相同、control相同、准确metadata/原marker/footer/hash、旧红绑定、原group空与ledger终态。只证明fixture caller→device future；不替代server RPC/host-child/产品恢复。
- 原workload filepull是父future直接借用tokio::join!，没走LabNode.with；不据此改写prefetch结果。

**普通六次首次读取已观测，静默合同未由它们证明**：

- 首个源码725e12f7的e6569ee7仅格式红/零行为，原件保存。
- source8f87adcb的 **turing-20261112T022049Z-7b65087d**：六次读取全ok，但metadata误期望两个netem都在alpha，缺router ingress读回，原1/226.568秒/17files失败保持。
- 修正source **5a5d7e84a557b2f2000aa0dcf513d61adc4538cb** / tree d1e6496d / snapshot67db6c6f，control2cfce93d；**turing-20261112T023151Z-8d7b0ea1** 原0/224.643秒/17files。六caller/十二起止/162条命令、真实双向150ms安装及恢复清除、veth ifindex/link_index双向绑定、逐接口计数独立复算、全部原SHA与ledger终态。
- alpha phase0/14/5首次读取297.685/327.577/321.867ms；beta230.688/265.406/250.937ms。实际phase与两端事后Relay路径、全部接口差值见first-action-verification-v2.json；每格只有一个fresh，无p95/p99/显著性/唯一因果或能耗结论。
- quiet-scope-audit-v1.json绑定上述proof：没有主动前台调用不证明无后台STREAM；完整N1/quiet未接受。普通source-owned输出显式continuous_stream_silence_verified=false/full_quiet_contract_accepted=false。

**Iroh50被动见证、v1真实红与v2准入绿**：

- 只在原250ms reselect已有stats读取上加DEBUG快照：所有受跟踪连接STREAM tx/rx、actor序号、增删代数及单调/墙钟；连接在采样间新建关闭也会改代数。原合计/ActiveRefresh/策略/生产计时器不改，无新任务/定时器。
- source **09374004906a9a400004874fa957726e857bef1f** / tree b23ff21f / snapshot8ee7e55c，control **1bdb2272f23bfd59961af79cbebf04242791ce33**；**turing-20261112T030608Z-15ebb293** 原1/305.229247秒/29files/identity/原group空。
- Rust原两guard+六读全过、十二真实请求阳性对照全过；十二静默见证全红。stream-quiet-verification-v1.json准确冻结复算、独立原人口/全部原SHA与ledger终态。
- 十三次STREAM变化被见证于候选窗起始后307642–645086us，actor/registry不变。这是观察tick，不是精确发包时刻或唯一业务来源；原失败保留，不把后半段选出来改绿。

**当前v2修复的是准备阶段**：

- 新revision first_foreground_stream_witness_v2，控制73fa35f5只改同task MODE literal；runner/其他分支/原UID/flock/netns/资源/期限逐字不变。
- 仅诊断模式、候选窗之前只读双方serve.log，实际counter/actor/registry签名连续≥1秒相同、快照年龄≤1秒、总准备期限10秒。无请求、无Node状态查询、无关闭后台任务。
- 记录准备起止和各端实际稳定快照；外部逐条绑定原日志。随后原12秒候选窗、固定500ms边界内≥10秒完整见证、整窗恒定、最大1秒采样间隔/100ms时钟增量差、原phase0/14/5与30秒caller、六fresh顺序及阳性门全部不改；候选失败不重试挑窗。
- 普通模式仍10秒/off且不等待此诊断准备，诊断不作为普通计时/CPU/能耗样本。
- 11种见证+6种准备+19种first-action+9种guard合成数据，4 unittest已过；6种调度stub保持guard off→action full与首原73/79失败。全部只是工具合成准入，真实v2已按顶层run/proof通过该诊断准入；完整N1/ordinary性能仍未接受。

## Mac默认vendor与其他回归

**新Iroh50准确源码默认库已绿**：Mac **43146243502** / request **ff36367cf58e4a1ebd0e5d6f5d1c460c**，source09374004/treeb23ff21f/snapshot **901d85407023827999147998c3318192f8be302b**，control **4e34fb7dce174345410a33e892b0eb76a5cc5c23**。
artifact **13911657397** / ZIP **4d8db782bfd2767c88855d5a14f3b6249aa06e338617ed1ba7e715e5a4904ce3**；E/mac-stream-vendor-43146243502，实际文件在artifact/（不是report/）。
16files/11controls/source/bundle/两vendor锁逐字一致/各原footer和SHA核验、vendor-full-verification-v1.json与ledger终态。
Noq完整默认库447过/0忽略/0筛除；Iroh完整默认库184过/原1忽略/0筛除。唯一原ignored flaky为endpoint::tests::connect_via_relay_becomes_direct_and_sends_direct，未运行/删除/称通过；58路径/11home专项与184重叠，不相加。
相对当前5064完整vendor差异只有KHOR.md文档，运行时代码与锁逐字相同；该证明适用于vendor字节，不证明CLI新准备逻辑或最终Node/native/手机。

原e471准确源码Mac **43134579036** 固定12项net-node全部通过：net68+1原worker ignored/hop89/wires16/journal30/budget32/IPC18+六单项。该门不是全部1378 Node，也不能声称新Iroh50之后最终Node已全跑。

隔离rolling原型57ffe：4窗口逻辑、跨八片的files_wire和handoff真实scratch合同已两次过；files库32过/唯一Finder trash AppleEvent -1712失败，Mac **43124311515 / 43126157862**，整轮失败保存。一次确认已用尽，不盲目同源重跑、不豁免该门。只读主机观察43128627304为零产品测试，未证权限/响应唯一根因；后续仍需收口。

## N3已闭对照与接下来rolling

[优先级](SCHEDULING.md)：low/high各完整18fresh，分别12960交互全ok、文件全完整、原population/hash/准确metadata独立复核。低profile目录p95九对-1更快，但high文件/目录/终端服务九个对应位置六次-1更慢、三次更快；不把位置差当逐请求因果。canonical c3287169恢复中性priority0，PULL_AHEAD=8；不是全局最优已证。

[批量prefetch](PREFETCH.md)：原8,1,2,1,2,8,2,8,1九组18fresh完整终态，12功能通过/六high文件原240秒超时；12960交互全ok、153原report文件SHA与原始人口复核。low九份16MiB全完整；ahead8三份192MiB完成82.239568–98.920231秒，1/2各三high超时。超时是右截断attempt，无verified payload rate，不称文件完成时间。
prefetch-comparison-v1.json SHA **7dede5f2c580d46aff0e6138b968da2550db2e095d6de351652a56d8a7e537b0**；全部advance1–9与旧high1–6终态不能重复。

[rolling协议](PIPELINE-COMPARISON.md)及prefetch-to-pipeline-decision-v1.json在任何rolling计时前冻结：**先宽8，再独立宽2**，priority0，每宽A B B A A B、每block原w3两项再low/high各1，共12/宽、总24fresh。宽2在low的完成/交互取舍值得保留但high原超时，不能直接设全网默认。
四源码与campaign已冻结；当前十二个block/二十四场景均收件核验（含三个原high文件超时）；原campaign终态，独立尾部诊断不算新普通样本。匹配baseline/candidate包含同一未调用helper/四逻辑/大于八片的files_wire载荷；A原batch函数不动、B仅rolling循环及必需短读重启；四树审全部Git/runtime/fixture/参数，只在相同宽度比较。不拿旧prefetch当新baseline，原失败/完整人口/期限门不改。
真实RPC的已发/完成/待提交上限、补齐与caller drop已有四源码合同，用户Job/DropStop/Offer stop/cut本轮真实合同已闭，但控制responder不替代生产source。后续仍需生产server/worker、完整run_take与host-child、最终发布所有权；全前台预览、公平/背压/连接架构/规模成本仍必需。

## 完整剩余范围（不以已有绿项缩减）

- N0：最终分场景UX门、充分尾部样本与p99、相同条件完整前后对照、观测成本与成本前沿；对载荷/调度/模型作实际记录。
- N1：严重/quiet/recovery尾部、跨连接独立质量/旧样本、恢复后固定好路径匹配参考、启动/配对旧失败根因、动态模型延迟突变问题、真实静默及后台成本。旧16–17秒/单轮绿不算极致接受。
- N2：同一实际连接独立/同时IPv4与IPv6地址/前缀/NAT映射变化、换网/睡眠/黑洞/LOST，owner/cancel/late/revoke与carrier在调用开始后失效；身份、总期限、写入/0RTT不重放。
- N3：上述24rolling探索已闭；仍需完整前台文件/预览、已发现的终端尾部、持续公平/饥饿/背压、共享与分连接实测、真实server和独立host-child取消/退出、1/3/5设备CPU/字节/唤醒/能耗。文件快不代替其他行为。
- N4：Direct/公共Relay/peer relay/SSH双腿，唯一可达middle自动救通（第二腿可Relay或旧信息）、多个/移动middle、独立双栈变化、北京/香港整机/单出口/寻址故障域，自家relay约束、双家与地域/容量/运维设计。部署采购另授权。
- N5：主60项（Iroh1–41/45–50+Noq1–13；42–44仍隔离）的逐项上游等价/保留退出，其他feature/integration/physical/Noq1+physical5；最终准确源码Quality/完整Node/native/proxy、Finder依赖、多设备/手机自动换网/peer relay与真实能耗及一致报告。

历史动态f2f1775027、恢复ce3b12226、长稳de04bcc1、四peer6970ad46、模型容量c50ab6ef和cold high5d27f45e等既有证据在历史入口完整保留；warm v2路径失败、启动/配对超时及旧严重红项也保留，不能拿新窄场景覆盖。没有用户iPhone候选安装/最终接受。

进度估计：此前2–4天为低置信度估计，不因compact/session重新起算；目前没有可信完成日期。完成必须设计、状态职责、持续矩阵与同条件报告均有准确版本证据。未获准的合并/发布/采购不自动执行。

### 2026-11-12 04:03 UTC 状态回复与估计

用户再次询问；明确尚未完成，普通性能与iPhone自动换网/peer relay/能耗未接受。下一批真实RPC合同加24匹配场景暂按4–8小时预留（含排队与构建、低置信度、不是承诺），完整计划仍数天且无可信日期；此前估计偏乐观，不从本次回复重新起算。全部N0–N5范围保持。
