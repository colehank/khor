# 普通 Release 下的唤醒改动验收

本实验回答：W 的一处唤醒条件变化，能否消除原发行配置的交互失败，并维持原公开基线的体验。原 N3 高延迟退化及诊断轮全部保留。

冻结协议：`/Volumes/disk0/Khor/evidence/net/copy-wake-release-comparison-plan-v1.json`，SHA256 `f131d0901b4ea78491624fb9632d3afcde5c2be0c577f5d7a4e9e585783c7c5f`。在本批业务计时前冻结；尚无本批性能结果。

三份源码分别为原公开基线 B（tree f6c239ae）、原升级候选 C（adbc7d6a）、唤醒消融 W（f5faf2a2）。C→W 的 runtime 只改 remote_state.rs；共同业务、观察器、网络夹具逐字 hash 一致。W 不包含 R 的 peer relay 恢复修复，也不包含补丁38移除，不能将后续结果扩称最终集成验收。

先三臂各执行原5个观察器/网络校准；原 cold TCP 红不修改，继续按 MODEL-DECISION 的真实分层准入。业务顺序固定 B C W / C W B / W B C，每块 low、高延迟 high 两个原场景，每臂每种网络3个新会话：共18场景、12960个计划交互记录。720条/场景必须保留，包含 timeout、blocked、取消和未发意图；不替换失败轮次。

真实远端 PTY、目录与完整16MiB/192MiB文件同跑。原2秒交互总期限、120/240秒文件期限、每操作每阶段120个计划记录、实际重叠和结束/取消/清理门全部保持。成功条件仍为原全部业务门；工程回归筛查仍为每网络/阶段/操作的 p95 不超过基线1.10倍加20ms、完整文件中位吞吐至少基线90%。所有失败明示，p99报告实际样本量，不作任意WAN或能耗保证。

本机只做源/证据编排。Turing 在原50/40GiB与排他lease下编译，计时期间不编译；每个case先实测CPU及压力，若资源或所有权不满足就停止、后续记未执行。还原原producer的准确source和全部selected ELF/dep-info，使用未修改的 native_provenance 核准，不重写旧凭据。

W 的8个原稳定性控制已通过，但这些helper实际是目录读取，不能当作真实PTY接受。Mac补丁38库检查已对照绿，真实NAT退出尚待。本实验也不替代同步/预览/SSH公平性、空闲能耗、IPv6/手机换网及最终同源N0–N5验收。

执行已进入固定业务矩阵。root使用串行stdlib编排器（当前PID及句柄只查STATE/progress），每块真实终态后核两类证据再派下一块。活跃期间该进程独占source-refs/progress写入，root不另行调度；它不会因产品红补跑，也不会因观测超时替换仍在途的run。未知测量/所有权/launch会停止后续并保留准确句柄。

## 完整批次结果

2026-11-13本批已终态、收集、逐块核验，root编排器PID20826已退出，原source-refs无pending。固定18业务场景、12960计划交互全部留证；无替代轮次。三臂各3个low及3个high新场景，共同业务夹具和原门保持。

| 源 | 原业务门通过 | 交互outcomes | low文件中位Mbit/s | high文件中位Mbit/s |
| --- | --- | --- | ---: | ---: |
| 原公开基线B | 6/6 | 4320 ok | 7.163660 | 19.715313 |
| 升级原候选C | 6/6 | 4320 ok | 7.252352 | 24.307740 |
| 唤醒消融W | 5/6 | 4200 ok、2 timeout、118 blocked | 6.917343 | 23.493630 |

**拒绝W作为超时修复或默认设计。** W的low终端pooled scheduled-to-end p95：bulk为6.379894s（B为80.491ms），idle_after为8.396699s（B为43.197ms）。high有2个原2秒caller超时，后续118个blocked是未发意图，不能计成118次网络超时，也不能从分布删去。完整文件仍成功不能抵销交互失败。旧8/8稳定性控制不替代这份业务反例。

C本轮没有交互超时，但也未通过原工程门：low的bulk目录p95为371017us，对照239185us，原界限283103.5us；high为580146us，对照452402us，原界限517642.2us。C的本轮TTY数据不能覆盖原Release批次high 2/3失败，不能据本轮全功能绿宣称已修复或最终晋升。

原3个cold TCP参考红全部保留；三臂分层模型准入均过，并未把TCP原门改成通过。p99仍仅为有限样本nearest-rank，失败条件分布明确，无独立显著性或任意WAN保证。

完整数据在E/copy-wake-release-comparison-verification-v1.json；C对B按原门补核在E/copy-wake-parent-recheck-analysis-v1.json。原cohort progress的12个闭块各有identity、block及resource三个证据，全部源、日志hash、实际CLI/selected还原和无强制清理见证保持。下一步的单一原生CC分量诊断见[CUBIC-ABLATION](CUBIC-ABLATION.md)，不重新包装或替换本批失败。
