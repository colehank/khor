# 当前集成版的移动地址回归

此页记录有限 Linux netlab 结果，不代表完整 N4、物理 iPhone 或能耗验收。

当前产品 runtime 逐字 I2 `3093f866`（R `9081b94d` 加验证工具修正），iroh1.4.1/Noq1.4.0；测试树 `a5de5685` / tree `90a1d7813377d611ea93b3276a36d3876e28e09c` 只增加两个 Release bridge，共35行。原始场景和 helper 不改。

固定三场 `163223Z-b6d7f5c7` 已闭，0/108.146424秒，15files/identity/native来源/CPU/owned清理均核：

| 原始场景 | 已核结果 |
| --- | --- |
| 地址注入校准 | 7次正向包检查＋1次旧地址负向检查；实际同前缀IPv6替换，IPv4不变 |
| IPv6-only，双向发起 | 124次真实目录读全成功，含2次setup；换址后的首次a→b/b→a为258230/1217614μs |
| IPv6-only，远端先发起 | 124次真实目录读全成功；首次b→a为1732189μs，完成后a→b为13665μs；原事件顺序已核 |

Node场景保留IPv4内核地址，但在两侧真实namespace中阻断eth0 IPv4输入/输出；不是双栈fallback测试。核验了旧IPv6消失、新IPv6存在、IPv4快照不变、前后nft隔离规则与实际包检查。原read30秒/setup120秒/scenario240秒/owned300秒保持；以上首读只有各1次，不作分位数或UX达标声明。

证据：E/turing-20261113T163223Z-b6d7f5c7/current-ipv6-mobility-block-verification-v1.json，E=`/Volumes/disk0/Khor/evidence/net`。CLI普通Release SHA `7e8c52bc4d21a3471e3c0592ad1a874147a7d024c067bda23a7f4d08213b34df`；新原生准入与两次编译前控制器错误见STATE，失败原件保留。

随后在同一源码、同一CLI上，重用完整netlab原生目标，完成原始四项换网场景 `170134Z-6a92a593`。0/268.968832秒，16files、源码/原生产物、CPU准入及全部owned清理均核；496/496次读取成功（每场124次，含2次setup），没有重新编译或替代场次。

| 原始场景 | 换网后首次a→b / b→a（μs） | 独立核验 |
| --- | --- | --- |
| IPv4/NAT，远端先发起 | 4267 / 640826 | namespace实际IPv4变化，外部UDP反射地址变化 |
| 双栈，远端先发起 | 4462 / 745793 | 实际IPv4及IPv6 /64前缀变化，外部UDP反射地址变化 |
| IPv4→IPv6-only，远端先发起 | 4599 / 662618 | IPv4移除、IPv6添加及原远端先发顺序 |
| peer relay的middle换网 | 702225 / 572910 | middle实际IPv4变化，换网前后持续peer relay状态分别观察 |

前三场均先完成b→a，再发起a→b，不能颠倒两者解释。middle场景保留原45秒前置hold预算、360秒scenario和60秒恢复观察；换网后再次观察到原carried状态并持续10秒。该状态摘要不等于独立包级路径或最优性证明。所有读取仍用原30秒期限；以上各一次首次响应不作分位数或UX SLO声明。完整test SHA `e4f2f52c7c1ff7f17b5e970c20c1aab5d40d5620dd118fc8491631ee8fa84913`；证据E/turing-20261113T170134Z-6a92a593/current-replug-block-verification-v1.json。

剩余包括双端同时变化、更多地址/故障组合与middle失效。发布/登记收敛、新连接地址hint和旧epoch不能只凭这些读成功认定完成。真实Wi-Fi↔蜂窝、锁屏/休眠及能耗仍待候选真机验收。当前执行见STATE。
