# R1 owned runtime close A8 实际验证独审

2026-10-27，root。source05仅新测试guard命名并立即drop修复，生产相对原A8未变；root已核33文件双盘及实际delta，不延长原锁释放时点。

实际run `Q037/remote-r1/close-a8-run-20261027T034134Z`：Node libtest67.753s、GUI integration test60.667s均正常构建；两phase各自实际normal库variant/module/source、catalog/detect生成Rust来源门通过。真实CLI来自上一成功正常构建并保全内盘，KHOR_TEST_MCP_EXECUTABLE指向该实际copy，无重复CLI编译。

root独立读取result、六个actual raw/hash与GUI执行记录，核80份外内字节一致。版本化entry的六个ignored exact测试分别1/0/0，GUI tools/network与restart两项各1/0/0，合计8 distinct；不是worker或列表计数。整体180.977s/exit0，source稳定、cleanup_errors空，实际自有进程/组及private exe footprint清、TMP清。作者补核66PID/32PGID全退；原CLI/test/GUI及两phase17normal证明稳定，无guard/forced/fallback。

该结果支持本进程agent producers/cancel owner关闭屏障与GUI fixture清理，不支持整个Node IO shutdown、远端Cancel完成或真实Tauri AppExit。原03 setup/04编译失败、S22晚写14文件与Stop5.27s旧失败仍保留。原B19回归、实际Tauri、慢metadata Stop、R2未知首次Send/lateSend fence待完成，R1整项不关闭。

## 原 B19 回归独审

原paired target在 `close-b19-run-20261027T035233Z` 实际52.566s构建成功。R0完整raw中，内部remote_cancel_worker与外父各有成功summary，实际是1个业务父；root读完整日志核六Prompt/六Cancel/四held及后5/6投递、claim/进程清理均通过。控制误要求唯一summary而中止，原41dual/失败不回写，不重跑R0。

随后 `close-b18-runtime-20261027T035817Z` 0Cargo/0mtime刷新续原tail18，源与actual4二进制/normal8证明未变。root逐读18个command.log，均唯一1/0/0且无skip；result passed、source stable、cleanup_errors空，91文件外内逐字相同。结合此前实际R0恰为19 distinct。作者另核163PID/76PGID全退、TMP清与private4exe footprint0，无fallback/forced。

因此A8新增8项及原B19 19项在当前source05已通过，当前R1仍待新增Tauri调用实际类型检查、慢metadata Stop复现/修复、R2及整App验收；不称整个Q037完成。

## Tauri正常调用方类型检查

`tauri-caller-check-20261027T041629Z` 实际正常macOS `cargo check --locked --offline -j1 -p khor-gui --lib` exit0/35.297s，总37.103s。khor_gui、Node closeAPI、Sync、catalog四项实际fresh=false及当前ROOT/dep-info来源通过。root独立读result/normal metadata，核26双盘/source稳定、group/TMP清、无guard/fallback/cleanup错误。正常tauri-build生成四个ignored schema保留并另存实际字节，未当源码或前端生成。

该证据证明新增RunEvent::Exit调用方实际类型检查通过，0测试/0App运行；不冒真实Exit验收。已授权作者提交exact12 WIP源码checkpoint，不含schemas/证据/文档，再由root集成；慢Stop与R2另续。

## 检查点集成与后续慢Stop

f010/8242/c81已无冲突集成为e5e913ff/47d509e2/dd9c45ca，随后N1集成至c13a5c44；三个共同Node源保两方更改，非共同23条R1源精确保留，组合native/App另待。

后续真实metadata启动fixture先遇test误用DeviceId.id编译101/0父，已仅删.id修正；再 `metadata-startup-red-run-20261027T050904Z` 实际libtest63.730s过，zero16Stop567ms真绿，真实2s metadata gate使slow首次Stop2123ms并触原1s主断言。实际intent/close/Pi与metadata ESRCH/TMP门齐，50dual；是新产品红，不倒推旧5.27。

已授权仅Stop登记准入后不等metadata的最小修复，未完成时诚实PersistenceUnknown、owner/quota持续持有，Status/Retry旧语义不改。source03已交未独审/运行，原R0与close等待者夹具适配另授权；本卡8+19/Tauri原结果不回填为新fix通过。
