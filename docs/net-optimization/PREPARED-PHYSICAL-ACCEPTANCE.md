# 双middle实际prepared竞争验收 v1

2026-11-11，fixture实施前冻结。目标是验证用户要求的多个peer relay及时接管，并验证输家不会迟到覆盖活通路。此文件是待实施验收，不代表已有通过结果。

## 与既有矩阵分开

原f4/f4s/f4i都是X、Y、M三个khor节点，加独立公共Relay；只有一个合格middle。原物理矩阵能验证完整两腿转发与恢复，不能单独证明两个prepared请求并行竞争。c2b原三轮9项已通过，但f4i仍28.2/17.8/34.6秒；比较候选a73的同条件原矩阵正在运行。两份冻结源码与原fixture保持，不将新测试混入此前前后归因。

新case可使用既有netlab-hop的f4_前缀，另外命名结果f4r，沿同一trusted control、namespace/资源/单线程/三轮规则。加入后属于四case×三轮的新12项协议；原三case的拓扑/门/持续时间和测试正文不变，不用旧9项checker伪装成同一集合。没有必要为此放宽控制入口或修改host网络。

## 真实拓扑与前置

X和Y位于不能直接互通的严格NAT，M和N是两个可达的非递归middle，公共Relay按场景限制或慢于接受界。所有节点为实际khor resident，走真实Tunnel、实际双腿和内层端到端TLS；不使用Publication的transport adapter。

前置必须证明X知道两个middle且两者都符合信任/暂停/单跳规则，Y的prepared能力hint已到达，M/N首腿与至Y的目标腿已暖好，M实测排序在N之前。开始测量前X↔Y没有已发布的carried通路，不能让自动比较提前完成测量。仅限fixture临时配置/网络控制；具体实现需沿现有Lab Node.with、freeze/thaw、namespace规则并核实际状态。

## 必需合同

1. 阻塞M的准入回复而保留其首腿已连接，N仍能实际到Y。发出一次X→Y只读业务请求，原总3秒/更早caller不变。首个有效业务响应须小于1.5秒，同时确认实际Carried经N完成；这个界严格小于原legacy第一slot可占的1.5秒加后续网络代价，不能用最终变成Forwarded替代。记录从操作起点到首内容、实际inner wire、carrier/via与所有请求结果。
2. 取得响应时M仍未给出准入回复。N胜出后恢复M，持续至少10秒和至少30个只读业务样本，全部成功，内层wire连续、两端carried经N且无误退役/反复换路。若声称迟到的M→Y准入也已覆盖，必须另证该迟到请求/回复实际发生；仅解冻或恢复网络不够。必要的额外诊断与普通时延样本分开说明。
3. 记录两个准备的活动范围、输家流的取消和目标端carrier提交，确认只有赢家有承载数据。未发送carried payload的迟到准备不能替换活peer，也不能因旧owner清理删除赢家；保留候选取消后outer仍健康的事实。
4. 记录工作量与限制：候选/请求数、CPU、字节与额外诊断；缺失项记unknown。一个新case不替代N3能耗/公平性、N4第二腿Relay/混合旧版本/独立v4-v6换址、唯一middle和真机验收。

初始化/暖路/拓扑前置失败与业务合同失败分开保存；不重试抹掉失败。物理故障控制必须命中实际第二腿，不能仅按假定私有IP下规则。资源和总期限保持，缺证据不补零。

## 后续扩展

再覆盖某middle经公共Relay到Y、只有一个middle真正能到Y、目标或middle独立IPv4/IPv6变化，以及旧/新协议混合。当前hint单元六合同只证明准入缓存上下文，尚不证明新物理第二腿被使用。最终与iPhone自动换网、交互/批量共存、后台成本一起接受。


## 连续性观测修订 v2（2026-11-11 11:36 UTC，待验）

v1的f4r实际三轮首内容1.22–1.25秒、后续各30次成功，但started-only helper未获得wire，三项原红保留。Path journal对同peer多连接去重，标签started或selected不是业务所有权凭证。v2增加实际Ls/Dir完成时的lifecycle收据：normal decode成功、early握手Accepted后成功响应、server完整write/FIN成功；记录连接/stream身份与客户端路径事实，不含路径参数和内容。要求首请求加30持续请求双方31收据、stream集合严格一一匹配、各端只有同一个wire、首响应/单一carriercommit在M恢复前，随后无outer detach/I/O结束/新commit/业务wire退休，每次成功时carried且两端viaN。原业务、时间和次数门完全保留。此case显式开启lifecycle，即使从无诊断的suite调用也含日志成本；原三case模式与正文不改，不把该case作为无日志SLO。v1原记录与v2不能合成同一测量版本的前后对照；迟到M OK、输家取消和完整成本仍独立未证。
