# Q-023 现代 owner fallback 补丁复审

状态：working（发现字段存在性漏洞，待作者补正后复验）
审查者：validation_audit。原 Q-023 实现作者为本审查者，原实现的非作者审查由 root 完成；本卡只对 root 接管后写的现代 claim 收紧补丁作非作者复审，不伪称整个原实现为独立审查。
只读目录：/Volumes/disk0/khor-code-quality-wave1。只写本卡和隔离证据；不修改 root 已接管源码及 Q-023.md。

## 审查依据

root 在 a065b891 原实现上发现同名不同机器风险：没有 installed binary 时，name相同不足以证明 machine fingerprint 相同。原现代缺binary、同名不同fingerprint两项补充用例在修前失败；补丁要求现代claim必须由可执行binary解析，legacy name-only才允许名称回退。

先查 graph minimal/detect_changes；图陈旧且错误关联 commitCase，不用其风险/节点结论。实际补丁仅给 khor_owner_name 增加fingerprint_authority参数、无binary两处传no/当前authority，以及三类测试调整。

作者证据 Q023/review-red：68项中66通过/2失败，失败正是missing-binary-modern-claim和missing-binary-same-name-different-fingerprint。Q023/review-green：68/68。红绿同测试SHA256 6fc5c7a0f65bac6bba35501e03aac8822fee711218c5180ba5a63d85efc3d027；补丁lib SHA256 1ffe6880307ace3582d90572344c7c14058c49d09e67993caebb0c9ac8512648。

## 发现 R1：字段内容的数值假值绕过存在性判断

位置：scripts/onset-lib.sh 的khor_owner_name，`machine = $2`及`if (machine && fingerprint_authority != "yes")`。

awk将输入字段的数值字符串`0`/`00`/`0.0`按false处理，因此具有machine字段的claim仍可在authority=no时输出hostname并返回成功。Rust Owner解析将非空machine字段作为Some字符串；无binary时不能靠name确定它属于当前机器，故这仍绕过“现代claim必须由binary决定”的保护。

独立只读probe：把各claim写入scratch文件，source真实onset-lib，然后调用khor_owner_name文件no。`fingerprint-A`返回1；`0`、`00`、`0.0`全部返回0并输出B。没有调用卸载/SSH/kill/rm。证据 `/Volumes/disk0/khor-code-quality-evidence/Q023/review-zero-fingerprint.json`。

建议以独立seen_machine布尔值判断字段存在；该helper只返回name，不需要保存fingerprint内容时可直接将machine设为1。重复字段判断也必须用存在标志，否则第一个machine0还可能允许第二个machine字段。应追加该claim拒绝用例再复验。

其余补丁方向成立：authority来自binary是否成功解析，不能由调用者hostname代替；home owner与选定store owner两处都传递限制；保留bare/name-only兼容。待R1关闭后补最终结论。

## R1 关闭与 R2 追加

root 已改为独立 machine_seen/name_seen/legacy_seen。审查者对真实helper再跑6项scratch只读探针：machine 0/00/0.0与重复machine字段都返回1；合法name-only `name 0` 与旧单行 `0` 返回0并保留字符串。证据 `Q023/review-zero-fingerprint-fixed.json`；R1已关闭，新增72场景完整回归由作者记录。

### R2：`khor root` 是路径选择，不是归属证明（待处理）

补丁仍将可执行binary的root回答设为authority=yes。实际调用链：

- crates/commands/src/lib.rs:834–855 的bare root明确不打开Node，直接打印command_root；注释明确store可能拒绝open而此命令仍回答。
- crates/commands/src/execution.rs:54–58回到Node::root_from_env。
- crates/node/src/lib.rs:1774–1814先搜索匹配owner；没有匹配后，仍返回`hosts.join(here)`，不验证该默认同名room是否已有别的机器的claim。
- onset-lib当前authority=yes只验证owner文件格式，不验证它是否属于当前硬件。

触发例：共享HOME主store属于A，B当前hostname为B，已有`.khor-hosts/B/.khor`却属于C（复制或同名目录）；没有任何owner匹配B指纹。原生root仍按fallback打印该B路径。当前卸载helper会接受C的有效现代claim，继续停/删该store。

独立scratch helper probe使用只打印上述合法root的mock binary、主store owner=A及选中side-room owner=C，`khor_uninstall_root`返回0并输出C的store路径。只读，不调用实际卸载/SSH/kill/rm。证据 `Q023/review-root-not-ownership.json`，所测lib SHA256=16f0194c418dcc001be5ca994d7d38509014b54a57f60a0f76df8df931e1b7b5。静态Rust fallback与shell probe共同证明缺少归属校验，不声称已运行Rust二进制。

需要由binary提供真正只读的ownership验证，或在无法取得证明时拒绝；不能单靠路径选择、hostname或格式合法的现代owner放行。已发root，尚不给最终通过结论。
