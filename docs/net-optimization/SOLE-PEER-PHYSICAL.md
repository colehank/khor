# 唯一邻居与真实Relay第二腿的首个业务

2026-11-13；固定三次完成，完整目标见[GOAL](GOAL.md)，当前执行见[STATE](STATE.md)。

新增夹具仅两文件：`crates/cli/tests/netlab/hop.rs` 的模块声明与 `hop/sole_peer.rs`。独立分支experiment/net-product-sole-peer-v1，commit **ade7decb4aa2d8616fb028b0b0221873d9d6f494**、tree **a448adc27f7502deb7293bbfa376d96de539df3b**；产品源码与完整Quality候选117c39相同。尚未合并回feat/net或产品main。

X/M同LAN，X仍配置Relay但resident启动前双向封锁其Relay IP；Y启动前双向封锁全部UDP，因此M到Y只能经实际HTTP Relay。仅通过M配对roster，不先发X→Y业务、不等hop或no-relay长期宣告。三次实际first业务之前均报Idle，no_relay=false；正常后台机制一直开启，不能由Idle快照排除后台已在准备。

首个实际目录请求必须在预声明5秒内返回Y新建的独有marker，X→Y实报Forwarded via M，X→M为Direct，M→Y为Relay。原生产3秒hop预算不变；5秒是该受控业务的额外门，不是新的全局SLO。X的Relay防火墙每次实际drop14包，Y的UDP防火墙每次drop22包，原nft规则和counter JSON保存。

| 独立Lab | 首目录完成 | 前态 | 实际路径 | 原Rust | 清理 |
| --- | --- | --- | --- | --- | --- |
| 1 | 531136μs | Idle | Direct→M→Relay→Y | 1/0/0 | 无强制/遗留 |
| 2 | 524952μs | Idle | 同上 | 1/0/0 | 无强制/遗留 |
| 3 | 516796μs | Idle | 同上 | 1/0/0 | 无强制/遗留 |

run **turing-20261113T022908Z-54fab5de**，control **9f6d4d093dee569c27b4443e59b75b5fd4074879**，0/78.088398秒；18原reportfiles/hash/identity/group空、每case唯一实际Rust结果和目录start/finish、所选路径/真实两腿/防火墙/所有权均核，ledger闭。20/22/21个observed owner，原native checker及已校准private subreaper见证不放宽。proof `E/turing-20261113T022908Z-54fab5de/sole-peer-physical-verification-v1.json`，汇总 `E/net-product-sole-peer-results-v1.json`。固定3/新增0，不再追加相同候选试验。

来源：compile-only021226Z-a2207feb在原native_provenance准备并核实际test producer及其真正CLI，0/129.161091秒、runtime零；实际test SHA0ce7704b/CLI f129a57c。runtime不编译，原require_provenance重核源码/context/ELF。首次020722格式前置红、022103错误把环境指纹作值、022453加入未记录KHOR运行环境而被原门拒绝，均零产品runtime，原收据/日志/专属proof/ledger保留。v3只保留原producer环境、使用private TMPDIR；原Scratch负责生命周期，保留原完整日志与owner记录，未保留scratch homes。业务门/fixture/生产源未改。

该结果补上“配置了但不可用的公共Relay不能阻止唯一Direct邻居经真实Relay第二腿救通”这一实际受控病例。它不是同条件旧main对照、单补丁因果、p99、缺失/过时第二腿hint、旧Direct历史、暂停/未信任/递归负向、任意换网、真机或能耗接受；NET-16/N0–N5保持未完成。
