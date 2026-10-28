# Q032 root 独立审查

对象按序：5ce2721f、c4ec0fa10469920a0158481275819e8351225eb3；审查者root，非作者。

核对鉴权后的route实际调用json_request::dispatch，四路raw仍保留自己的处理器。CountedRead在交付serde scratch前计encoded/decoded/single-string/depth，保留跨Read的escape/surrogate状态；Tree在容器插入前计宽度与总节点，重复key仍计资源。serde仍唯一语法/UTF8验证器，decoder.end保证合法前缀后的错误不被忽略；读取错误/非法JSON绝不触发callback。空/空白/null与合法无参数命令保留，预算413、读取/语法400由真实route映射。

单字符串64MiB；普通合计65MiB、merge193MiB；encoded6倍+结构余量，宽度65536、节点262144、深度128。这是对过去无限大请求的新资源边界，非“所有历史无界输入兼容”。数字不以字符串保留，Value大小受结构预算；不是请求并发总RSS承诺。

核对真实cargo-web-locked.json与当前8项source/lock hashes逐字一致，前后source固定；26项真实Cargo通过（18 parser、4鉴权loopback HTTP、4既有）。HTTP测试使用隔离scratch/auth和静态console_catalog，无用户命令副作用；真实413由65537元素触发。64MiB普通/6倍转义生成流、3份64MiB merge与20k DropItems为模块兼容证据。

root独立校验vendor UPSTREAM 43原文件SHA：只equal_reader.rs变更；cached archive来源checksum与原锁一致。同0.12.0，仅workspace patch及serde依赖边，无其他版本升级。Drop固定8KiB缓冲仍drain精确remaining，保留原EOF/error/channel语义。root实际std-only5/5复跑通过，日志Q032/root-vendor-review.log。上游master也核对仍有原分配，不能用未验证升级替代。

结论：无本提交阻断，可以集成。慢连接deadline/总drain字节、整个请求并发内存和handler/output资源限制不在本修复闭环；需后续传输层审计，不据此声称整个HTTP服务全面有界。集成精确版本回归与CI注册仍待完成，main/发布未授权。
