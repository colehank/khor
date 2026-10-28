# Q033 GUI 独立复审

- 审查人：agent_redaction（未参与这两项 GUI 实现）。
- 固定作者树：`/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-store-owner-recovery`。
- 范围：`3f2b8b18^..8da0d063`，即 `3f2b8b18` 与 `8da0d063`；HEAD 为 `8da0d063ae9e364bc8ff74fb26bdced79c14883b`。
- 结论：指定旧 retry 矩阵通过；新增 P2 notice 归属缺口在后续 7746378f 修复，独立原红例与有效 cancel 正例已通过。固定 3f2b8b18 + 8da0d063 + 7746378f 在本审查范围可接受；集成/完整门归 root。无 Cargo、未修改作者源码，结束 git status clean。

## P2：旧 notice 的 cancel 仍可清掉新 token 的失败提示

位置：`apps/gui/src/views/ChatView.tsx:1006`，cancel 回调中的 `setUnsent(null)` 无条件清空当前 notice。相邻的正文 discard 已按 `say:${draft.token}` 选择性清理，retry 也验证 mounted/session/owner/当前恢复条目的身份，但 cancel 未有同样保护。

真实最小顺序：A 发送失败，捕获其生产 notice 的 cancel action；B 独立发送失败；从正文丢弃 A，确认 B notice 仍在；调用先前捕获的 A cancel。此后 B notice 消失。B 的 saved snapshot 仍存在，故未证明稿件丢失或误发送，但旧 action 能撤掉新失败提示及其中的重试入口，违背本轮第五用例要保护的归属。

以作者第五用例为基础，只在独立证据目录复制运行器和 fixture，把捕获的真实 action 从 retry 改为 cancel，并在 B notice 验证之后调用它；仍加载固定生产 ChatView、registry、production api.ts。Chrome 与 WebKit 各执行一次，均在“旧 cancel 必须保留 B notice”断言真红，pageerror 为空，源 hash 未改变。没有将夹具/编译失败计作业务红。

建议：cancel 的 notice 清理需按原 notice id/身份核对，消费/恢复 draft 继续受 session/owner/条目生命周期保护。添加 stale cancel 与正常当前 cancel 的对照，避免为了阻止旧回调而撤掉正常操作。仅按 token 处理 token-null 老消息时还应考虑多个 legacy notice 共用 id 的身份关系。

证据：`/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q033-gui/review-stale-cancel/`，含 `review-driver.mjs`、`review-cases.mjs`、`report.json`、两引擎失败截图和 run.log。该额外实验 report.finished=true，2 执行 / 0 通过 / 2 业务断言失败。

## 已独立通过的指定矩阵

使用作者原始脚本，明确选择：`discard-notice,add-notice,switched-notice,app-reopened-notice,discard-other-notice,retry-token`。Chrome 6/6、WebKit 6/6，总计 12/12。原始真实 notice action 在丢弃、加入输入框、keyed unmount、实际 App 同 id close/reopen 后均不能再发；正文丢弃旧 snapshot 保留新 notice，新 notice 仍可正常 retry；正常 retry 使用新 token，保留 images/docs，旧 receipt 不改动新 turn。

命令（作者树根）：

```sh
PLAYWRIGHT_BROWSERS_PATH=/Volumes/disk0/Khor/caches/dependencies/khor-playwright CHAT_CANCEL_WEBKIT=1 CHAT_CANCEL_SCENARIOS=discard-notice,add-notice,switched-notice,app-reopened-notice,discard-other-notice,retry-token CHAT_CANCEL_CHECK_DIR=/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q033-gui/review-final node apps/gui/scripts/check-chat-cancel.mjs
```

证据：`/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q033-gui/review-final/report.json` 与 `run.log`。report.finished=true，12 个实际结果均 ok，pageerror 为空。最初未指定共享 cache 的尝试在 Chrome 6 例后因默认 WebKit 浏览器路径不存在退出；按卡使用已安装共享 cache 后完整重跑成功，未安装依赖或改锁。

## 源与检查边界

- ChatView SHA-256：`f9742cf69795050217a460096b990f3c24546f796aa9146d0b9b0948bbc8274c`。
- cases fixture SHA-256：`cdfa83cb6073def3fe11061fb46a4272cb71b530ae416421875be3dec0e4df0e`。
- 两份 report 的 14 项 source fingerprint 与运行结束现场逐项一致，作者 HEAD 未变，git status clean，git diff --check 通过。运行器临时创建的夹具已清理。
- 先使用项目 code-review-graph；指定作者 repo_root 可定位实际变更及影响。图索引 build 为 3f2b8b18，落后 8da0d063，故 followup 依据固定实际 diff/source、实际组件断言，而未以图覆盖缺口冒充正确性。
- 已只读检查 registry 的 owner/pending/saved/timer 转移、读者代次、精确 token receipt、附件恢复 thumb=null 和未确认条目不提供 replay 的约束，未发现本次范围内其他已证实阻断。
- 本轮是两引擎真实 React ChatView/App 与 production API 的组件检查，IPC/HTTP 和文件选择接缝为合成；不证明真实 native/webview/host 协议或全 26×2 场景重跑。中央卡已有旧完整矩阵，本复审只认自己本轮实跑的 12 个通过与 2 个额外红。

## 7746378f followup 复审（P2 已解决）

固定 HEAD：`7746378fcd2c3ebd29a984d84f57dde1f27211d7`，作者树起止 clean。新增生产差异仅8行：cancel 先核对 mounted/current session，再仅在当前 unsent 与原 notice 对象 identity 相同时撤回；null/legacy 帧也把原 notice 对象发布，避免 spread 造成当前有效 cancel 永远不匹配。draft 恢复仍走 addDraft 的 owner/session/条目消费检查。对象身份比 token/null id 更精确，也能保护共享 legacy id 的后续 notice。

非作者实际重跑：

- 作者正式 `stale-cancel-notice,cancel-notice`，Chrome2/2 + WebKit2/2 = **4/4**：旧 A cancel 不能撤掉 B notice，B 可继续 retry；有效当前 cancel 恢复原始含空白文本、2附件、1 doc，撤掉自身 saved row/notice，发送次数仍1，没有重复发送。
- 原冻结独立 `review-stale-cancel/review-driver.mjs` 与 `review-cases.mjs` 不改一字，在固定新生产 source 上重跑：Chrome1/1 + WebKit1/1 = **2/2**，原两条真红均转绿。结果另存 `review-original-red-green`，没有覆盖原红。
- 两份新 report.finished=true，所有 pageerror 空，14项 source fingerprint 与结束现场逐项相同，git diff --check 通过。无 Cargo、未编辑作者源码；未把作者56/56或types0当作自己复跑。

证据路径：`Q033-gui/review-followup/{report.json,run.log}` 与 `Q033-gui/review-original-red-green/{report.json,run.log}`（同上 evidence 根）。运行原正式脚本时 scenarios 明确为两个名字，使用既有 shared Playwright cache；原冻结 reviewer driver 仅改环境中的新输出目录。

新 ChatView SHA-256：`a9e621364270310fc38c115be45df873b839ff15804a782061e373c7419b5395`。
新 cases SHA-256：`c66665b548a002200f8ebdf268615aa8bc0c9012ead6ad410fc5f7273b6e26cb`。

本轮 followup graph 索引仅到8da0d063，故仍以固定生产 diff + 实跑为准。未发现此小修复的新阻断，建议 root 连同前两项 GUI 提交集成后跑完整全局门。
