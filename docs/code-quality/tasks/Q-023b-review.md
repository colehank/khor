# Q023b 非作者审查

审查者 root；对象固定提交 fb0c67001d003dda9d639f00cc10b86af0d50aec。审查生产 Node/Owner 提取、严格 CLI 分支和 onset proof 前后调用顺序，并独立执行 check-store-owner.py。

结论：未发现确定阻断，可以在原生验收冻结窗口结束后串行集成。

原 Owner parse/render/is_mine 逐字保留并共享，避免改变原首次打开/补 claim 行为；新增 destructive verifier 的严格格式校验与该 permissive 规则分离，缺失/空白/歧义/重复/损坏 claim 不会成为删除授权。现代 claim 必须有实际本机 fingerprint，hostname 相同不绕过；同 fingerprint 改名仍允许；legacy/name-only规则保留。路径必须绝对且不含遍历，root祖先、.khor和owner拒绝symlink/nonregular。硬件fingerprint本地读取并进程内缓存，未打开Node或创建identity/owner。

root --verify-owner 只有严格单flag可用，输出版本标记与实际request-local root；额外参数均拒绝。onset先验证候选必须处于HOME或直接side-room布局，显式KHOR_HOME=该候选后请求证明，整个回执内容与候选逐字匹配成功后才继续stop/remove，旧binary只回路径不能放行。HOME/room换行拒绝避免回执注入；两边shell命令替换均规范化尾部换行。

独立轻量验证：真实生产Rust模块规则6项和Rust verifier→mock SSH/onset80场景全部通过；源码与fb0c6700对应，完整证据在 Khor/evidence/runs/khor-quality-Q023b/root-review。作者真实 Cargo精确CLI测试1/1，确实执行非0测试，并证明没有identity/owner写入；源码fingerprint与任务卡一致。未操作真实SSH/卸载/kill或用户store。

边界：不支持 --verify-owner 的旧binary拒绝卸载，需要升级；这是阻止未证明归属的破坏操作的兼容变化，已记录。无本机fingerprint的现代claim拒绝。未声明防御同权限恶意进程在证明与删除间替换路径的TOCTOU，也未声称Windows/Linux真机通过。CI unittest入口会在缺rustc时显式skip，后续Q024工程治理需在quality脚本job显式提供Rust工具链，避免依赖runner预装状态。
