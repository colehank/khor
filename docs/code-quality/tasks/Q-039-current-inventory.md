# Q039 当前inventory收敛

状态：integrated@1d2640d2；root独占wave1的crates/node/src/link.rs、proto.rs，以及catalog双语消息与必要当前协议测试。基线76db834b；Bun真编译结束后开始，不改agent输入，不改native_helpers的sync层。

依据用户无旧Khor兼容决定，先移除Versions拒绝→Vouch→legacy全量sync_everything双路径。仅真实admission门返回typed NotAdmitted允许一次既有签名Vouch后重试当前inventory；一般Refused/未知响应明确错误，不能猜成旧端。保留认证和已有table-lost自愈真实流程。

所有已选doc继续且任何失败可见，不再optional global doc静默失败；epoch仍隔离，但结果明确部分同步而非Already in sync。大body传输当前尚未改，原25MB失败隔离回归仍适用，不把本batch称为chunk已接线。Pair专用初次双轮仍属当前bootstrap，留待统一chunk后替换，不再从inventory拒绝落回它。

验证计划：当前proto roundtrip/拒绝类型、真实table_lost/现代Node isolation、正常库check。当前版本重构可直接实施，不为旧端制造业务红；真正缺陷的原始红保持。root每步固定diff与source，候选冻结后请非作者审，获唯一Cargo槽后运行。main不动/不ship。


实际交付：root完整candidate后samefixture两红→两绿；正常库check、166不同相关Rust/完整scripts过。两处doc-only校正后再次正常check+当前26+GUItypes，生成catalog真实producer一致。来源及资源/skip边界读STATE最新收口和Q-039-current-inventory-final-review、Q-039-inventory-root-candidate-review。Node25MB本体/Pair chunk仍开放，不用此批关闭全Q039。main未合并/发布。
