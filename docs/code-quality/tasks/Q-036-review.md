# Q036 独立审查

审查者：native_acceptance（非作者）；2026-10-25。只读源码审查，未运行 Cargo，未修改实现。

现场：`/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-overwrite-recovery`，分支 `fix/quality-core-regressions`，HEAD `93438e4324844d3e970f89e5de4a6cbf92c29d12`，仅 `apps/gui/core/src/{file_handoff.rs,lib.rs}` 未提交。审查diff SHA256 `9982615e89f2300087a461369aee929872bf54fa0ce3d5214228cc862f27a09c`。file_handoff SHA `974d7b60440bcde4dbf9f6d1850a8f02d1d2523ff23b5e8b8f7b7c0d40a4e17c`；lib SHA `5bfc2eca7999bbaac49fc0ae846b150e0b10b105ffab8bba41c1923e5e15aaab`。

已按review-changes先调用graph minimal/detect/affected flows/tests_for/impact。图谱仍是cab465c5旧分支且不覆盖prepare，返回0流/0风险不能当作本次安全证明；以下依据现场完整调用路径。

## 最初中等发现：scheduler新测试没有覆盖“agent”分支（现已修正）

位置：`apps/gui/core/src/lib.rs:3078–3082`。

新测试用不存在目录调用form=agent，断言no_such_dir。实际 `Node::start_session` 在 `crates/node/src/lib.rs:4782` 校验目录，直到4798才match form；所以任意未定义form在该目录上也得到同样拒绝。若将4805的agent分支删掉或改为拒绝，新测试仍通过。它能证明旧的“注册agent在目录校验前一律拒绝scheduler”已去掉，但不能证明测试名所称scheduler form被接受。这是当前入口回归覆盖缺口，不是新增生产故障。

建议保留此错误优先级断言，增加一个真正到达form分支、但无需启动agent的断言：注册命令为scratch内明确不存在的**绝对**路径；使用已存在scratch目录调用agent；与 `Node::agent_launch(&n.agent_or_refuse("zed").unwrap())` 的精确错误比较。这样命令查找必然失败，不依赖机器是否安装some-acp-agent、不制造假二进制，也不会调用open_gui_at启动agent；删除agent分支则会返回no such form，测试必红。此断言仍仅证明入口能进入注册agent launch路径，不能宣称MCP handshake或scheduler完整运行已验。

## 无生产阻断：首次下载路径canonical化符合既有契约

`prepare`的缓存命中分支原已canonicalize结果并限制在canonical版本目录内；`ready_path`也canonicalize结果并检查位于app cache内且为文件。原首次下载仅返回未canonical的landed，因此在macOS的/var→/private/var别名下，同文件第一次和缓存命中返回值不同。

候选在成功pull、取消/版本检查后，发布ready manifest前取得canonical结果并返回，manifest仍保留相对路径。它保持缓存可重定位格式，不取消既有所有权/ready检查，也不放宽原 `cached == dest` 断言。canonicalize失败仍进入原result.err清理work路径；不会发布一个已知无法解析的ready路径。现有取消/原始65MiB字节hash/版本变化/未知id/partial拒绝测试均应保持。

## 验证要求与建议结论

- 修改上述测试缺口后，先跑 `khor-gui-core` 的file_handoff::tests与wizard_agent_tests，再完整core；必须记录实际二进制/source/lock指纹及失败/ignore边界。
- 原路径红来自macOS临时目录别名，现有严格路径相等断言不得改为仅比较文件字节或忽略前缀。可补充 `dest == dest.canonicalize()` 和 `ready_path`返回路径身份相等断言；这些有助表达契约，但Linux无别名环境单跑绿不能替代已记录的macOS红绿。
- 新wizard断言应对“删除agent match分支”敏感，且不依赖本机可执行程序；真实scheduler MCP支持另由协议集成测试证明。

## 修正后复审

作者已采纳上述发现并完成同函数最小增强：注册scratch内绝对不存在的missing-agent路径，先assert不存在；保留term精确拒绝及缺目录优先级断言；再在存在的scratch目录调用agent，精确比较公共Node::agent_launch对同一注册spec的缺命令错误。实际生产代码先在match agent内调用picked.launch，错误在open_gui_at之前返回，因此不会启动agent。删除agent分支会改为no such form，新增断言可以发现该回归。没有安装假可执行或依赖some-acp-agent是否恰好可用，也未放宽原路径相等断言。

修正后diff SHA256 `9b7bb207e0a95bb655ed65cc65fef3450ffc1057cf780b22b75958cc125b0558`；lib SHA `1d9a52541dd408eb263eea365a35f00483ce588a70e49c8033afc8fb9b04c149`；file_handoff SHA保持不变。git diff --check通过。未运行Cargo。

建议：**修正后两文件源码审查通过，无剩余阻断**；等待真实针对性与完整core测试后集成。本审查不计Cargo类型/测试通过，也不据已有Q025部分绿宣称Q036或全core完成。
