# Q033 后端非作者审查

结论：**固定后端两提交源码审查通过，无阻断，建议进入串行集成并接续独立GUI适配**。不代表Q033完整用户体验或原生App验收已完成；Q037全局取消/进程期限另列。

审查者native_acceptance，2026-10-25；未修改源码，未Cargo。基线`2ab71a6c`，交付`219948301df7e3aab3e775ad8609ddb8d469c605`加完成收据followup`3d0dc41f724744d19857e5d215186481fb2bcb4e`。工作副本外盘current/khor-quality-native-io，fix/quality-upload-cancel，tracked clean。10文件指纹逐项与delivery.json和committed-followup-delivery.json合并结果一致；review/receipt.json保存具体SHA及独立测试binary指纹。

## writer入场、Stop与完成

- submit_say在同步调用时登记独立Sending/原token并入队；Stop只有成功入controls队列后才在同一sends锁下标记当时已入场的未commit输入。后来入场的turn不能被该次Stop吞掉。
- write_op_guarded对每帧先拿socket writer锁，再调用Sending::before检查Stop并更新Stage；Whole/Parts在同writer临界区进入Committed。Stop若先拿到Sending状态可取消；若writer先进入Committed，Stop沿普通host Stop，不声称回滚已发输入。Committed/Finished不会设置本地cancel，claim_notice也因stop为空而拒绝取消收据。
- 上传取消等待Stop实际写线，才在原socket补空Parts终止host discard；正常下一turn在turn队列后面，不能越过旧上传收尾。首Part前取消仅本地完成，不向空闲host虚构Stop。
- StopSignal由队列闭包内StopWriter负责完成；closure因queue close/拒绝/异常被丢弃时Drop发错误并唤醒Condvar，不留下等待一个永不执行Stop的取消turn。Queue close先从队列取出job、释放queue锁再调用取消；发送worker等待controls时不持socket/state/sends锁。未发现本批新锁环。
- SendJob拥有原Chat/Sending，执行前被queue丢弃、preceding-controls错误、socket写错误均有原token收据；Sending同一notified位仲裁取消/失败/成功，避免重复取消和Unsent。状态只保留小token，不把附件/原text放进长期frames。

## host/wire与legacy

- 新host在其connection Reassembly上把Stop后的未提交upload转discard-until-Parts，剩余Part不能再解码启动prompt；后续Parts复位，原socket下一turn仍可用。上传Stop且尚无prompt时返回既有Turn(Cancelled)，不是通过截断解码/断连伪装正确取消。
- GuiOp、Hello、Welcome未加positional字段；GUI能力使用成功Welcome.why=`khor-gui/1`。共用connect_welcomed仅做cookie/deadline握手；terminal仍独立识别原HOST_VERSION并发ClientOp::Version，GUI不会误发终端协议帧。每次far重新连接也更新能力。
- 新client遇legacy上传取消，安全性来自原attachment EOF和host丢弃未commit aggregate：left/gone、两queue close和Connection独立shutdown一起退役，SendCancelled先入frames再gone。原Connection.closed拒绝迟到replace，rejoin也读取left，不能把旧Stop补到重开的attachment。
- 明确兼容边界保留：旧client首Part前的将来输入没有身份，host无法猜它属于此前Stop；新client有本地同步入场身份。降级Welcome链路证明当前client legacy分支；真正旧host行为另由保存的Q025旧probe/ACP日志证明，不把降级当前host冒充旧二进制。

## token与HTTP统一完成

- token可省略/null，非空ASCII限定字符且最多128字节，native/HTTP共同验证。其仅存在本地API/frames，不进入GuiOp或ACP positional wire。收据闭包捕获旧Arc<Chat>，不会按同id重新lookup到新attachment。
- HTTP/bridge chat_say同步admission后返回null以免单请求loop堵住Stop/poll；native的pending可等实际write。因此promise resolve不是跨transport的统一commit。
- followup在write_op_guarded真正成功、包括Whole/最终Parts写完之后发SendCommitted{token}，HTTP/native均可从同frames消费；它不表示host/agent已接受或完成。失败/取消仍按Unsent/SendCancelled，legacy无token成功不产生无法归属的完成收据。GUI必须按结构化token收据释放snapshot，不能据resolve清稿；该GUI工作不在本后端提交中。

## 证据与独立复验

作者固定版本：committed-core **15/15**、committed-host **4/4**、committed-native **2/2**，source_stable=true；实际gen_bindings、types过。delivery与source-before全部10文件及lock已核对。早先14项日志保留，新增HTTP完成用例后才是15项，不混用计数。

本审查者直接复用作者日志所列、已编译的固定测试binary，**没有调用Cargo或重编**：

- khor_gui_core-d3a0aad91eaddf52 io_tests --test-threads=1：15通过、0失败，0.37秒。
- chat_upload_cancel-2f473834503f83be --test-threads=1：真正host+ACP/门控TCP 4通过、0失败，0.65秒。覆盖新host原socket继续、legacy显式重连后settings/权限、旧client剩余parts不复活prompt、committed小turn普通Stop不退役。

binary SHA256及运行前后inode/size/mtime不变记录在`/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q033/review/receipt.json`；core.log/host.log有完整断言结果。独立测试使用隔离fixture，不连接用户会话。未冒称独立再跑native宏/生成绑定；这些是已核对的作者固定版本证据。

## 未完成边界

网络不读导致Stop writer卡住、agent不合作或子孙进程持有pipe导致runtime wait/join无界，仍属Q037；StopSignal解决的是已关闭/被丢弃队列不完成，不是所有Stop都有全局deadline。此次不得据此写成“全局取消期限已解决”。

GUI尚须消费send_committed/send_cancelled/带token unsent、保存与恢复附件独立snapshot、明确legacy重连提示和Stop错误反馈；需最终集成TS生成/交互与实际App验收。上述是交付边界，不构成本后端固定范围源码阻断。

图优先已执行minimal/detect/flows/tests_for/impact。图谱落后followup且tests映射不完整，impact高提示API边界需复核，本次据具体锁/调用链及真实固定测试判断，不使用0 affected-flow作为安全证明。
