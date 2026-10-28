# Q038 非作者审查

结论：**当前生产reader/drain与executor接线源码审查通过，无阻断**。新真实sidecar pipeline用例仍待唯一Cargo槽实跑；未把std绿替代完整node构建/runtime验收。

审查者native_acceptance，2026-10-25；未修改实现，未Cargo。工作副本`/Volumes/disk0/Khor/worktrees/current/khor-quality-agent-protocol`，分支fix/quality-agent-stream-io，HEAD`4a6bf596aa389426d7c6de18ae575f69eeb9024d`，3源文件候选未提交。源码SHA：

- khor_agent.rs：`76ba361d07b94643f2b8ba51d9d3f253bebc6e185a76a3f8e745953e73f09215`
- khor_agent_io.rs：`e45c6a510c18503724951f26fc2d77a64bf837124bcead666264264e882b1387`
- khor_agent_output_tests.rs：`72fcc00131889b5d533343e4642c090eec9b6ecad8648cef0031bdf65e053544`

## 边界与合法事件

原executor按String::len检查2MiB，是UTF8字节限制；候选保持同一个字节预算，在读取时先以Read::take截断到2MiB+CRLF+1，再read_until分隔。EOF、LF、CRLF的剥离与原std BufRead::lines语义一致；无换行末尾的单独CR保留，空行也保留。完整行转换UTF8发生在读取预算之后，合法多字节字符跨BufReader块不会被拆坏。

最大合法payload+CRLF恰好可交付；超过1字节，无论是否有换行都会产TooLarge后结束，不再把巨行送入channel/executor；TooLarge仍映射原agent_arguments错误。允许BufReader本身最多一次buffer预读，实际测试用其8KiB上界断言，不声称底层pipe严格只读预算字节。Vec容量也可能多于有效字节，但分配上界与事件总长度脱钩；仍是按行预算，不宣称channel64个事件的总存储预算已减少。

UTF8/真实读取错误产一次Err后结束；executor的line?收到该错误即退出读循环、杀子进程、drop receiver、wait与join，不再消费后续输出。合法事件仍走原JSON/update/MCP验证；只是把既有超限判定前移，没有删去语义schema验证。

stderr固定栈上8KiB原始buffer：不构造String/行，不保留凭据，invalid UTF8/无换行均可drain；仅Interrupted按标准约定重试，EOF和其他永久错误退出，不因同一ConnectionReset无限循环。

## 独立实跑

在证据目录新建wrapper，以#[path]导入**实际生产叶模块**。`rustc --edition=2024 --test`编译，无Cargo/依赖重建。

实际11/11通过、exit0：生产7项；额外4项分别验证：

1. 0/1/limit-1/limit字节、LF/CRLF/EOF与std::lines逐值一致；中文/emoji及最大合法UTF8内容跨1/2/3/8191/8192/8193字节读取分片一致。
2. limit+1字节在三种结束方式均TooLarge，下一次None。
3. stdout永久ConnectionReset只读取一次、只交付一次Err，下一次None。
4. stderr先Interrupted再invalid原始字节仍可完整收尾。

证据：`/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q038/review/{receipt.json,review-tests.rs,compile.log,tests.log}`，记录确切命令/二进制与源SHA。实际leaf SHA与作者green-final记录一致。git diff --check通过。原3绿4红和前置wrapper编译错误已区分，不覆盖原证据。

## 真实pipeline与收尾边界

新增8MiB shell sidecar完整/无换行两形式通过真实execute→failed receipt路径，父worker有20s监控与kill/wait guard；writer-finished标记应未触达，原始输出不写入receipt。新用例尚未Cargo运行，必须等待该证据才可声明pipeline通过。完整行旧基线的writer标记与kill存在调度先后可能性，因此确定性原红以已记录module读取字节预算为准；新实现下pipe读取提前停止仍应让8MiB writer无法触达标记。

**全局child.wait/join期限未由本补丁解决**。子孙进程持有pipe、done后子进程不退出、部分管道长期不结束等仍可阻塞现有wait/join，属于明确另列Q037；不因本批reader正确退出而把这些runtime生命周期标绿。

图优先已调用minimal/detect/flows/tests_for/impact；图谱旧版本不覆盖新leaf，因此低risk/0 flows不能替代源审与实跑。非阻断建议：将独立分片/限值对照测试择要纳入叶模块，以保持未来读实现替换的兼容检查。先完成真实Cargo pipeline与既有node输出回归，再固定提交/集成。
