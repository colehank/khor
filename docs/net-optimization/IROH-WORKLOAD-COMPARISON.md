# iroh 升级的混合负载对照

2026-11-12，首轮计时前冻结。目标是检查依赖升级是否带来体验退化；不重复或覆盖旧batch/rolling24场景。

A=旧整合dc73837e（iroh1.2/Noq1.3），B=新02807c79（iroh1.4.1/Noq1.4）。两者都是rolling8、响应priority0；crates/scripts差异只有两份依赖Cargo.toml，工作负载/辅助函数/解析器逐字相同。完整SHA清单和不可变输入在E/iroh14-workload-plan-v1.json，SHA256 **a5a6565df56b75e176f689da2dc886f77efad6bdc5bb671ab1798d6bd1803226**。

顺序 **ABBAAB**，每block独立low/high各一个fresh，每stack/profile三次。low16MiB、high192MiB；每场景720交互样本，计划共8640。每block先原两w3 guards，再两场景；所有原操作/file期限、内核profile校准与完整人口门不变。资源/flock/netns/jobs16/nice10保持，Turing计时期间无本任务并行编译/测试。

复用准确control **d28538241f29a1608a7fed606da25d2fde57470f** / task netlab-workload-pipeline及冻结metadata。该parser返回的部分limits是旧batch/rolling活动的历史文本，只作单block验证，本计划才定义此处OLD/NEW比较；不改原组件伪称另一个实验。

失败文件保留为失败，不计算它的有效吞吐；缺失/重复/不完整计时人口停止campaign供定位，不补跑替换。全部六block结束才做成组描述；不以三次fresh声称显著性、p99、全局最优、手机能耗或完整N3。原peer首次连接间歇红仍未解决。

动态执行/收据见STATE和E/iroh14-workload-campaign-v1.json。预计六轮约30–40分钟，受编译和主机负载影响；这不是整个N0–N5完成估时。
