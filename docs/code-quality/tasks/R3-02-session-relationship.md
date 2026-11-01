# R3-02 会话关系药丸

基线：integrate/round3@dee0caab。Root单独实施和作者自审；main与用户运行实例不变。

## 交付

- 正常会话不再常驻显示独立“会话恢复”按钮。原药丸承载连接、续接、失败、结束和待确认；展开为已认可的同排短状态＋原GlassItem操作，工作中无重复按钮，操作后原药丸收回并呼吸。终端与聊天既有进行态移除“立即重试”，不另建工具入口。
- 进入时首个就绪offer只对自有GUI、已确认结束、已声明Resume/Load且提供epoch的情况自动续接；原持久intent只核对同一请求。正在查看时的后续退出不自动重开。普通Shell结束只显式新开新run，未决／未知不冒新执行。
- Pending按同一持久payload串行核对；Unknown或传输失败保留原intent供明确重试。切目标／卸载清理timer和notice，迟到结果仍由原hook的generation和IDB结算规则处理。后续revision清理旧成功结果，未决intent不清；最终拒绝后若已确认live且无未决请求，恢复普通身份。
- Shell的live状态复用已有endpoint核对：Meta/HostFile、进程出生、child-parent及退出记录；不是仅看PID存在。Ended与new_shell准入仍按原结束证明和row gate/token算法，不因UI显示Live扩启动权限。旧记录与新SID关联保存在后端；移除菜单中的技术身份／上一会话ID，不删除旧记录。

## 证据与边界

目录：`/Volumes/disk0/Khor/evidence/round3/R3-02`。

- `shell-red.log`：健康Shell旧投影为Unknown，真实Rust断言先红；`shell-final.log`与`ownership-final.log`：新状态投影和原输入归属两项通过。
- `final-result-red.log`：本轮自审补获“最终拒绝仍盖住健康live”的状态问题，保留先红；最新`unit.log`包含29项生产关系投影、终端点击及全部既有GUI单元检查，通过。
- `ui-final/report.json`：18组Chrome生产组件/真实IDB回归通过，含13组原历史、4组原动作与1组进入自动续接／后续退出。随后状态优先级小改的相关5组在`ui-relationship-final/report.json`复跑通过，来源首尾一致。
- `types.log`：最终类型检查实际退出0。`catalog.ts`与`catalog.log`来自真实gen_ts producer，未手改生成结果。
- WebKit尝试因缺少本机webkit-2336运行器退出1，保留`ui-combined.log`，没有下载依赖或冒通过。Chrome的固定传输不等于Node真实provider执行或Mac/iPhone真机验收。

原catalog重复短语复用已有语料；字体改为同等500的语义font-label；两条连接／继续文字是进行态，加入既有ongoing名单，未放宽动作标签省略号规则。初始夹具的手动菜单／未等待目标切换等旧假设已按产品契约调整；原13组历史捕获与清理断言保留。

## Clean Code自审

关系判断集中于纯recovery-relationship，展示与Node执行资格分开；notice、timer、异步目录读取有明确组件生命周期。持久intent和执行监督器沿原owner，未增加第二套恢复注册表。两个药丸宿主共用同一展开和活动规则；必要注释只说明进入／后续退出、原请求结算和平台时序。未顺带重写恢复协议、格式化整个Rust模块或启动其它候选专项。
