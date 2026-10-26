# Q046 测试辅助函数归属

状态：root已验证并提交（集成后继HEAD见STATE）。候选基线1d2640d2、hop SHA3f929b28。

正常Node库检查持续警告pub(crate) hop::middles死代码。图谱callers_of找不到符号/HEAD旧，不能证明无caller；实际全firstparty rg确认仅本文件test调用，生产使用middles_with_cost（包含Legs策略及成本）。

候选将只投影id的middles wrapper移进cfg(test)模块私有helper，既有路由算法/测试断言/生产调用不改。更新模块doc链接到真实生产入口，保留不允许递归hop、排除自身/目标/satellite/paused等协议约束，去掉重复历史说明。不是一条行为bug，不造红测试或新增镜像测试。

待非作者看真实diff/调用面，再跑正常Node check（warning消失）与既有hop测试。此为P4按实际使用边界收窄接口的小切片，不冒称全仓风格治理完成。root独占；MCP和Doc作者不改hop.rs，main不动。


独立审Q-046-review无阻断，实际调用面/算法字节与原测试断言已核。`Q046/helper-scope-green-01`正常Node库check通过且原第一方middles dead_code warning消失；既有hop9/9，source稳定，vendor loro unused import warning未擅改第三方。没有新增镜像测试/没有改路由规则；目前仅这个低影响P4小切片完成，全仓工程治理继续。
