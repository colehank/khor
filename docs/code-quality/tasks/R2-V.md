# V 构建来源与OS验收契约

状态 queued；属于既定L04/L05，不是新增范围。尚未分配实现owner或开始代码修改。首次整合native App验收前插入已有作者槽；root不另开第三实施线。

## L04 已有准确定位

当前integration 8f9506d3的 scripts/native_provenance.py::environment 在66行附近，显式names与CARGO/RUST/KHOR等前缀均未覆盖TAURI_CONFIG、REMOVE_UNUSED_COMMANDS。原实际Tauri producer已声明这两项，verify_build保守拒绝是既有证据，不得删未知env拒绝或回填旧run成功。

限定下一实施：将这两个已证输入（包括未设置状态）纳入哈希化环境快照与producer key；新增必要回归证明设置/修改/移除均失效、同源稳定，并保持未知rerun-if-env-changed拒绝。仍不打印配置原值。真实整合App build后用原strict verify验证，不复制构建系统、不清空target、不重跑无关业务矩阵。原Q055详细过程见Q-055卡，只有本缺口待本轮处理。

## L05 范围保持

OS退出harness需区分GUI拥有的临时进程与应跨GUI退出保留的持久宿主。调整归属/退出契约前核真实脚本和当前产品生命周期，不把仍活着的用户agent误当泄漏并kill。正常GUI退出及重连/恢复验收保留，不能以忽略所有进程存活追绿。原失败独立保留；实施时先给具体owned PGID/host identity/cleanup语义及最小验证。

## 资源与完成边界

唯一native槽、共享target、jobs1/incremental0、外盘TMP沿执行计划；源码/生成输入/产物真实指纹与accepted run关联。纯Python检查只能证环境key规则，不能代替实际Tauri producer。main/ship未授权。
