# 第三轮发布记录（2026-11-01）

用户验收Lab后明确授权“可以合并main并ship了”。功能与规划均无冲突合入并推送main，正式构建输入为 `ee7151381412e523c0018db791b1a400afee14aa`，产品来自 `integrate/round3@ae398aa7`。其后发布记录提交仅允许文档变动。共同界面收据为 `khor-ui-assets:/assets/index-C9KeWEdI.js`。

本轮交付终端内单击／修饰键打开、会话关系药丸、共享工作区文件／历史导航与预览标题。会话内容标签承载浏览过程：所有文件／提交／网页在所属标签内跳转，只有显式新建增加标签；返回导航显示新会话标签页，草稿与阅读状态保留。实现与Lab反馈已由用户验收，见[第三轮审查](ROUND-3-REVIEW.md)与各任务卡。

## 构建与验证

- 当前main的TypeScript、unit、生产Vite构建均实际0。完整生成文件来自已验收实施树，复用前核产品源码等同；源／gen／dist和界面收据均保存完整指纹。
- 门索引仍为既有非绿：本次34条输出均为上轮已有问题，无新增；原退出1及对照保留在gates.log/gates-comparison.json，不称所有门绿，不追加无关治理。
- Root独自执行并作者自审，无子智能体或独立审查。本次真实原生TUI点击／系统应用交接和iPhone手势／键盘未验；编译、进程／签名及上传检查不代替真人体验。

## 实际交付

| 目标 | 结果 |
| --- | --- |
| Mac | 第二次release Cargo0／2687.214s，CLI／GUI全UI与完整MacPi字节、strict签名、输入稳定均通过。CLI SHA `78f753adcff6a2bccc72e5466dbaaa52ef17b7df896f893565fcf1e263ce5342`；GUI `afa8c59b961bd82c4acc05be3f07a8d29aab7d10bb021cae6969172f382aca32`。CLI原子替换，GUI91072→95781（23:07:37），实际5467监听与HTTP收据一致，前后8个列表ID相同、全部原宿主保留，无新增失败。 |
| 三节点 | hinton单jobs1/nice19的local profile Cargo0／1917.435s，完整LinuxPi/UI、源码清单、ELF x86_64与GLIBC最高2.30通过。一次NFS安装，hinton479960／turing1444967／lecun1085393实际5467 exe SHA均为 `f74f2b6f0ac94f46123a943a55fa89ed08aa2707b45498412bb64fec00f33dfc`，HTTP收据一致。三节点当前前后均仅wide/all列表行，无会话宿主，无新增失败；不把该行冒称真实终端会话。 |
| iOS | archive-only实际0／1454.715s；arm64 **IOS**（非模拟器）／SDK26.5／min15.0，deep strict签名、team／正式ID／profile、新lib与App完整UI通过。App SHA `06dc6c3107072d451b9b2552e9280c308f26519d7e6dbe32fd31b4466667a111`；lib `7790abc1e9fb98c0707a8328a4785aaf173e73414c08ab803b84ef0e9286cc49`。两签名源按原bytes恢复。 |
| TestFlight | 单次Xcode12082／exit0／137.709s；Upload succeeded与EXPORT SUCCEEDED齐全，当前ContentDelivery真实数值字段确认为 **0.1.14（62）**。源、App／.a与导出选项首尾稳定。Apple处理可用性与手机安装未查询，不称已安装或真机交互通过。 |

安装器返回1：“serve did not stay up”，本次空日志；实际服务随后均正常采用新二进制，独立核验进程/收据/列表后合格。保留该1，不重复安装追绿。Linux构建附带GLIBC字段的旧sed反斜线诊断无效，真实GLIBC资格以独立readelf解析的stage-validation.json为准，未重编。

## 资源及恢复

- 首次Mac编译因系统盘8GiB保护主动停止（Cargo-15／261.565s），未发布该次结果。16GiB内存机器的交换空间增长造成系统盘下降；原失败保留，未删除系统swap或结束用户程序。
- 第二次Mac与iOS采用release、单jobs1、incremental0、CODEGEN_UNITS=16、ThinLTO，保持同一源码；这次参数变化重建了依赖缓存。后续同机发布应保持参数与缓存路径一致，避免反复切换配置重编。
- 已逐文件校验并将Cargo registry、旧恢复工作副本node_modules及npm下载缓存迁至既有外盘Khor缓存树，原路径留链接。node_modules物理路径保留完整层级；实际Vite导入通过。源码／内盘恢复集不动。清除两份归属明确且无引用的已完成Khor Xcode导出临时副本；未触碰浏览器用户资料或系统swap。
- 保留旧Mac可执行文件、旧iOS61归档／静态库、当前正式产物和所有失败证据。后续清理仅处理已确认本次所有、无引用且有恢复凭据的临时资料。

全部原始证据集中于 `/Volumes/disk0/Khor/evidence/releases/20261101-round3`，当前进度读release-checkpoint.json／RELEASE-RESUME.md；中央STATE／ROUND-3-CHECKPOINT及内盘recovery镜像在交付后更新。

- 上传后关闭本轮私有Lab8946／exec88030，脚本自行清理临时fixture；截图、正式组件样例和全部证据保留。另按实际build62、正式bundle ID及无打开引用核实，清理本次新Xcode导出暂存，保留IPA、归档、当前lib与真实distribution logs。
