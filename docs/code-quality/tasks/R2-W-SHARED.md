# R2-W 公共监听接口子片段

2026-10-30，owner `round2_history`。W 的 Node/Core 业务叶与前端消费者由 `round2_input` 持有；本卡只记单一 shared owner 的公共桥、真实生成及来源映射。整 W 未完成，main/ship 未授权。

## 固定来源与合同

同一作者树 `khor-round2-history` 临时切 `fix/round2-watch-shared`，保留 S `fix/round2-history@b14f73a8`。共享基线 `6c20c13b` 是 Root `e409a993` 顺序取 W 的 bf11/dae8/e65/1dc/0f 五个固定叶；没有引入尚未接 Core 的 S Captured。

- `57de45439e901ab18e9fe0260ffce7760e3f2bb4`：Core API、Tauri、真实 bindings roots、GUI facade、mobile 命令合同及私有 HTTP 测试，共六路径。
- `72527c7b05c113ddac2b410dbd8aaf2e7f287fe4`：两 catalog 源唯一新 key `gui.git-history-updated`，中英文为“提交记录有更新”/“New repository history is available”。刷新复用 `gui.file_reread`。

三动作 `dirs_begin(machine,scopes)`、`dirs_wait(machine_id,id,cursor,wait_ms)`、`dirs_leave(machine_id,id)`；五 DTO `DirWatchScope/DirWatchCoverageMode/DirWatchCoverage/DirWatchBatch/DirWatchOpened`。Begin 只解析一次机器名；后续使用返回的稳定 DeviceId 和 opaque lease id。HTTP 在校验 u32/cursor 形状后于服务端强制 wait=0；Tauri async wait≤20s。无旧 DirWatch/seq wire 兼容层，无手写生成类型。

## 实际验证

`evidence/round2/workspace/watch/shared-1`：exec67436/driver207，真实终态 exit0/154.478471s。strict actual metadata、Core+GUI ordinary-default lib 签名 check、Core libtest producer、command_tables producer、bindings/catalog producer 全阶段0；V prepare/verify/require_provenance 来源完整。

实际行为为 DTO exact1、HTTP exact2、command_tables4，共7。HTTP 使用真实 dispatcher→Core→认证 cookie IPC，仅 resident response 是测试夹具；验证 pinned identity、服务端 wait0、三动作与列出的非法形状拒绝。它不是 kernel watcher/远机38流验证。Core/GUI check 不等于当前前端资产或 App 验证。

2174 source frame 前后全同，八固定源 SHA 亦逐 git-show 一致；env/tools/BUNDLE 稳。双盘最低内盘7,435,186,176B/外盘9,139,712,000B，owned remaining[]，cleanup errors[]，TMP absent；唯一 native 槽已明确归还。

实际 bindings 169 managed files（TS＋inventory），先空 staging，再同一个经来源核验的 binary 管理发布，逐 SHA/文件集相同；五 DTO 存在、旧 `DirWatch.ts` 不存在。`generated.json` SHA `155c7b80f9909db109a05487ec9415d1261fa06febb21ecacedda5337d7f5544`。实际 catalog SHA `4b4be54136e0b6092e75a8deb4e1c33cc40660d0aa8f7a230c88c82f4d189241`。

`shared-js-1`：source/gen 前后同，mobile static []、words0；真实 tsc exit2，仅旧 FileTree/StorePage 消费者九条旧参数/字段错误。原日志保留，交 W 前端作者合并消费者后重验，不用兼容层掩盖。首次 mobile source checker 对 camelCase 值变量的误识别记录保留，改成真实 snake_case 参数，未削弱 checker。后置证据映射/bundle 命令的两次 setup 失败亦单独记录，没有改写 native 绿日志。

固定八源 manifest SHA `71580cdbaa89b1e00623615d6096232b7cbd3f33dc9ef794621f2487b5a2f719`；最终来源 `shared-fixed-72527c7b/validation-map.json`。runner/日志/来源及小 bundle 已双盘镜像；恢复目录 `recovery/round2-history-current/w-watch-shared-fixed-72527c7b`。

## 未闭项

等待固定 ref 尾核与 W 前端精确合并/真实 types、当前源 Vite assets 后，才申请真实 CLI `watch_tree` exact1（内部38流，不能称38测试）。本片未跑 CLI/Vite/浏览器/App，不关闭 W023 或整 W。S NoLoad Core/GUI、startup byte budget、L01/L02、H 全文搜索/复制按原 R2-S/R2-H 留账。


## 2026-11-01 发布决定与历史记录边界

用户最新明确授权“先直接commit到main并ship”。本卡以上实施、失败及阶段权限记录原样保留；本次授权及发布状态以[本轮发布记录](../ROUND-2-RELEASE.md)为准。原 Mac 真人中文 IME 与真实 iPhone 操作仍未验，按用户决定移到发布后，不登记为通过；本卡记录提交不代表已合并 main、完成任一 SHIP 目标或已发布。
