# Q046 独立只读审查

状态：review-complete / freeze及卡交回root，可排正常check与既有hop测试；未发现阻断。审查者native_helpers，作者root。基线wave1 HEAD1d2640d2；开始及结束唯一dirty为crates/node/src/hop.rs，前后SHA同为**3f929b2862f18c85591433c85837de5cc9f5e027f6a8e255ccaef0253b021e18**。

先用review-changes技能minimal/detect、file_summary及middles/middles_with_cost callers查询。图谱built34ef与HEAD1d2640不符，文件0节点、两符号not_found，明确不能据此证明无生产caller。随后读实际diff并用全crates/apps第一方Rust搜索核对：middles六处调用均在同一cfg(test)模块；生产调用middles_with_cost，另roadbook同名局部变量不相关。没有发现外部wrapper caller或被删除的测试调用。

实际差异只是原id投影wrapper移入cfg(test)私有helper，依旧调用middles_with_cost(..., Legs::Dial)并按原序去掉cost；闭包绑定b改id不改值。逐字比较确认middles_with_cost算法本体完全相同，移除新增helper后既有tests尾部逐字一致，原算法调用/断言未改。

简化doc链接转向真实生产入口，仍保留排除两端/satellite/paused、禁止recursive hop、按策略控制relay、Measured纯成本排序/其他直连优先及未知RTT按u16::MAX计。具体Dial/Measured/NoRelay/ToNoRelay规则仍在未改Legs文档和原match中，模块one-hop说明亦保留；未发现真实规则丢失。原rows/closures角色可由签名与Legs文档确定，本小切片不要求保留重复wrapper文档。

未新增测试符合纯范围收窄；本审未Cargo、未修改生产或测试，无进程/槽占用。正常lib dead_code消除和既有hop运行结果仍待root新帧，不能借源审声称已绿或P4全面完成。main未改/未ship。卡写权及freeze现交还root。
