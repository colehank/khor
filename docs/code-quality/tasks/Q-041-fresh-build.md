# Q041d — fresh-source frontend prerequisite

Owner root；状态 ready / 设计待实施。完整治理与当前开发协议授权内，无main合并/发布授权。

## 证据与准确归属

MCP候选树 `khor-quality-agent-protocol` @7550e4f0 的 build-01 实际失败：khor-web/build.rs因apps/gui/dist/index.html缺失终止。该build.rs明确规定直接Cargo编译前要先正常构建GUI；这个诊断本身按设计工作，不称为业务回归红，也不凭它宣称onset已实跑失败。

root随后授权同一树标准npm offline build：prebuild正常gen_ts/gen_bindings、tsc、Vite全链成功，1615输入稳定。277 dist逐文件producer指纹加入后续2047完整输入。build-02新CLI/stub成功，不借main旧dist。证据Q037/mcp-worker/frontend-build-01及build-02，以实际目录/receipt为准。

source trace集成92742250 scripts/onset.sh的SOURCE=build只准备Pi bundle后cargo zigbuild；没有GUI producer。clean checkout缺dist时将碰到上述必需资源门。尚未用真实完整onset复现，不把source推断当红。此前Q041c预检/17脚本绿/真实Darwin Pi bundle三smoke只证明其范围，没有证明fresh onset全链。

## 下批目标

统一完整源码构建入口，保证当前GUI与Pi均来自同一次选定源码/锁/生成器，依赖先备齐再跨编译和远程写入；保留khor-web缺资源fail-fast。SOURCE=release/ontarget不能被迫安装本地Node/npm/Rust。不要只检查目录存在而默用过期dist，也不要引入静默空前端或复制旧构建。

先核现有prep/build-on/release workflow的producer路径与构建锁，提出单源最小接口再修改。针对真实fresh树和带旧dist源码更新两情况验证实际producer次序/失败停止；fixture可验证编排，但真实打包/制品检查须分开。实际Linux/musl/SSH部署未授权额外外部写入、不冒充已执行；本轮实现验证继续在既有独立树与quality cache。

唯一Cargo/gen槽目前MCP作者；此卡不授权新编译或源码修改。
