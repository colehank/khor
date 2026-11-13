# 新旧稳定依赖栈的实际互通

2026-11-13，实际双向互通与完整host/child见证v2通过；旧准备和v1观测失败全部保留。完整目标与当前执行见[STATE](STATE.md)。
本项在两组冻结长稳全部终态、收件核验之后执行，不能在其计时中插入编译或网络测试。

## 要回答的问题

以原 `sync_mixed::an_older_khor_and_this_one_still_sync_both_ways` 验证真实新旧CLI及resident双向同步session row和pin。
旧源为 `dc73837e5bf33238b5cc2ed80ba8719fbb793601`（iroh1.2/Noq1.3），新源为 `3b734073b410ec4130b4b57def3bf096b10c3e7a`（iroh1.4.1/Noq1.4.0和当前恢复修复）。
这两份app都已实现Request::Versions，不能沿用fixture历史注释而宣称测到了“旧端不懂Versions”的回退。
原Rust fixture、scratch helper和两份native来源/进程所有权工具在两源间逐字相同，SHA在E/iroh14-mixed-static-preparation-v1.json。

原测试缺 `OLD_KHOR` 会返回绿色；本验收要求真实旧CLI、唯一正标记 `[mixed] older alpha, newer beta:` 和准确唯一Rust通过，不接受空跑。
原双方session row/pin门及40秒等待不改。原close命令忽略退出码，因此还要独立核对本run两个host和其sleep子进程确实退出。
只测原双向row/pin，不扩称全部RPC、WAN、peer relay、安装版iPhone或能耗。

## 来源和所有权

协议E/iroh14-mixed-version-plan-v1.json、控制E/iroh14-mixed-control-v1.json已保存。私有control **05419f73cadb7787ee5c4356004b79f672a2c1c7** 仅替换原d07eec56的net-node分支，原runner、其他任务、资源/时间守护和CONTROL HEAD不变。
本地只检查Python AST、bash语法和Git源等同；这些不算远端编译或互通通过。

旧源码从owned run `turing-20261112T110236Z-baeba5f5` 的package创建独立checkout，先核原bundle SHA **09cc5828c4e8137944cfaea5acf60e4a3b4a158cc670f38a54dba559902a9771** 与准确tree/build commit。
两CLI分别执行原native_provenance准入，随后将已准入的链接可执行文件按相同SHA复制到本run不可写路径，再改变共享构建缓存。旧、新来源各自留证，不能借用一个现成cache文件冒充准确版本。
新integration test通过原native_provenance的prepare_build/verify_build API请求 `kind=[test]`；不改验证器或放宽artifact/dep-info/source检查。

运行用短且唯一TMPDIR；scratch的khor-scratch tmux socket只能落在该目录。设置KHOR_TEST_KEEP保留本run homes，避免fixture末尾的兜底reap把close失败隐藏掉。
进程观察沿用原隔离native runner，额外从两个私有host.json/meta.json核对birth identity、host→child关系、实际sleep argv及旧/新host可执行文件SHA。要求两个角色均被完整观测，原owner退出且没有强制清理或私有可执行文件遗留，失败原样保留。
不按进程名杀进程，不碰共享tmux socket；本run新建目录留存于证据清单供核对。

## 当前准入状态

原工具库存Git2.25.1/tmux3.0a不自动构成本fixture阻塞。当前发现的不兼容initial-branch用法位于Git单元夹具；本项先使用现有环境，不为它安装宿主工具。
编译和运行各只按冻结协议执行一次，所有准备或行为失败留证，零替代重试。原90分钟/50GiB启动/40GiB停止守护不改。
以上为原冻结协议；实际执行结果如下，不能由旧准备文字推定仍未运行。


## 实际结果与观察协议v2

1. `turing-20261112T190345Z-5b6f85b7`，control05419f73：controller1/459.251317s，零互通runtime。普通CLI build与cargo test实际链接CLI不同，原hash门正确阻止运行；不改verifier。26原files、来源/hash/group空与mixed-preflight-failure-v1.json保存。
2. `turing-20261112T192613Z-c2c8ebe2`，controlce4998f5：原Rust1过、双向row/pin均同步，controller1/187.838121s；仅old host见证完整，原198行后置门失败。21原files与mixed-runtime-screen-v1.json保留，不叫完整通过。
3. `turing-20261113T011900Z-19b49119`，controlde74a134：controller0/160.457763s、原Rust1过/0失败/0忽略；old/new两个常驻host、sleep child、birth identity和ELF hash均实见，原进程退出、零强制清理/遗留，23原files/hash/identity/group空。专属proof `mixed-observer-v2-verification.json` 与ledger闭批。

v2仅改变私有controller观察，先独立校准：Linux subreaper捕获已分离但由内核收养的本run子进程，仍要求私有runtime image目录、准确已准入旧/新ELF与原birth/owner关系；不按进程名扩大清理范围。同一inode的ELF只哈希一次，size/mtime/inode变化使缓存失效。校准中原祖先采样无法见证而内核收养可见、缓存及原子替换失效均过，零产品runtime。旧失败不作唯一原因归因。

v2协议 `E/iroh14-mixed-observer-v2-plan.json` SHA256 **2bc1488f9a2f1c4cdd1aec44a7ed2cbc908c7651826d7d0cf4f917385ae50815**；controller SHA **0064f4af98972bd86557ea1d77659f191a35bcd6cafa6db278912eb67529881b**。原Rust fixture、runtime和最终断言未改变；复用同一原verified test producer与只读旧CLI，通过原require_provenance再次准入，编译零、产品运行一次、替代零。两个角色ELF为旧b2745d6e、新418df330；哈希读取共2次。

该结果只接受原session row/pin混版合同；两版本都懂Versions，不是pre-Versions fallback，不覆盖全部RPC、最终产品集成、WAN、peer relay、候选iPhone或能耗。accepted/full_N5_accepted仍false。
