# Relay静默故障中的真实前台请求

原idle c观察到fresh TCP20.0秒、再次heard22.9秒，但其原30秒合同不包含前台业务。独立测试树62b28e084a8cf72593a9ed1ba270327d281f9d49只新增2测试文件162行，产品逐字I2/R，原idle测试保持。

冻结协议E/relay-silent-demand-design-v1.json：原双strict IPv4 NAT与单Relay几何，真实旧alpha TCP tuple双向drop、新tuple可通；每场2个setup读，再40次新marker目录读取，caller2秒、250ms计划间隔、全部样本与串行积压保留，首个有效响应≤2秒。3个固定独立重复，无替代。

普通Release producer184400Z-8e2c478a 0/234.817081秒，Rust3分03秒，22files与严格来源已闭。测试5f607b385e9e6dfd96af183bd33adb265247dca699b6f7b8ba7ea3b9f6c80634；实际CLI重新编译后逐字同7e8c52bc，产品源未改。

runtime184914Z-5cfe53b4 1/198.559489秒，15files/identity/native/CPU/owned清理闭，**三场全红**。每场setup2/2过，故障后40读10次2秒超时、30次成功；共126实际调用96ok/30timeout。首个有效响应分别20042371、20040904、20049739μs。全部nft规则、原TCP tuple、42起止记录/样本关系已独立核验，见run/relay-silent-demand-runtime-verification-v1.json；失败不删除。

该几何复现了真实前台等待，不等同手机现场、全部故障域或所有策略。静态常规15秒Relay ping加最长5秒pong与观测相符；需候选对照验证因果。仅有空闲30秒内恢复不能作为最佳体验接受。

下一候选设计：实际请求已发且尚无ACK时，在按RTT缩放的短等待后请求现有Relay健康检查。由iroh继续判断pong/重建TCP；不因请求慢直接判死QUIC，不重放写，不把轻量健康检查升级成全endpoint重新绑定。检查仅属于有界请求，调用方取消后不保留新watcher；iroh现有actor合并并发提示，后台常规15秒节奏保持。具体候选与重复协议在E提前记录。原慢健康Relay、长Accepted、取消及后台成本必须回归，绿后才讨论晋升。


## 窄修复同夹具复验

隔离候选81ae5c9812b06298a06549f1c16a3728baac2c8d/treeba3da3451c027904b657183bc26d990191d75c30保留原fixture全部字节，按实际resident request在未收到datagram时提示既有Relay健康检查；TCP actor只持一个未完成health ping，匹配pong或原timeout完成它，避免并发提示/周期ping延长deadline。没有新周期timer、整endpoint rebind或应用写重放。该exchange也供后台同步使用，成本必须实测。

普通Release producer191019Z-979f0ccd 0/594.656134秒，22files/identity/strict native闭；CLI ef9a3dd1061ab557c4aa1709b9a14901813efff2e91251bbdef5e415f8c49519、test2296f0f2b87910dee724f65b9b52957e7e255bf9e0d9a29ea24fbb63f1816f4b。runtime192107Z-941d6b67 **0/166.445581秒**，15files/identity/native/CPU/owned闭，固定原3场 **126/126成功、零超时**。首个有效响应960623、958965、906570μs，均在原2秒内；基线分别20042371、20040904、20049739μs、30超时。各40故障后真实新marker目录读和2setup均保留。run/relay-silent-demand-repair-runtime-verification-v2.json。

对照不是同期交错A/B；只支持该固定几何里3次红到3次绿与首响应改善，不估总体可靠率/p99，不替代真实iPhone现场。v1仅原生compile保留、因并发deadline风险未运行/晋升。v2七组件回归、慢健康路/Accepted/取消/成本及全部N0–N5仍需完成。


## 健康延迟回归拒绝F2全面采用

同产品F2、仅加健康jitter夹具的J2 a199ff48，runtime201418Z-2b427674固定原3场 **1/112.210432秒**，15files/identity/native/CPU/owned闭。124→976ms只加延迟，无drop/reset/换址，42/42实际读成功，但三场原TCP均约1.11秒消失并由新tuple替换，158/159/161改变观测；kernel qdisc/u32/stat/无drop、100ms采样与2秒尾观察、原5秒caller全部独立核。run/relay-health-jitter-runtime-verification-v1.json。因此不能用silent3绿宣布F2整体可用。

F3 7a794274仅改actor，保留两夹具：固定full pong期限，另以RTT早期检查判断完全无回包；收到Relay消息只让过早期检查，不能结束当前pong验证或延长full期限。无新周期timer/额外ping/应用重放。新增3原生合同加原7先验，接着普通Release原silent3与healthy3共同通过，才讨论窄修复晋升；当前候选未验。


## F3共同复验通过

准确F3 7a794274/tree173ce959普通Release producer203855Z-f070edef，CLI a2a655f73f168dc10ed580e1a578c5db5c3056a6d5207aacceebc82aebba450d，test788230d99aec5e32ae4db895f55dcaf5b1caad9b08b295bfbede80a600e6f181。十原生组件已全过。原两fixture逐字不变，固定先silent3再healthy3，无运行编译/替代；各run15files、identity/native/CPU/owned独立核验完整。

silent205454Z-a05d8a1c **0/166.430165秒、126/126成功、0超时**，首有效响应953959/872597/934411μs；相同原2秒caller。healthy205820Z-118a2b49 **0/102.174631秒、42/42成功**，原TCP三场均保持、changed=0，原124→976ms延迟与无drop、5秒caller不改。总6场168实际读，E/relay-health-progress-combined-runtime-verification-v1.json汇总并绑定两份完整原件及SHA。

这支持窄修复同时改善完全静默与避免这类健康抖动误断。原基线30超时、F2三场健康误断保留。非交错同期A/B、不估总体可靠率/p99；发送已堵、单向断、one-shot/长请求后半程、其他协议/peer relay、后台成本与手机仍须验证。下一步独立准确源集成、完整Quality和原生Lab，不合main或发布。
