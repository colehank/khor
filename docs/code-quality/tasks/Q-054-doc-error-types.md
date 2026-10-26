# Q054 Doc本地错误分类边界

状态：ready after N1 flow correction，尚未改源；root，2026-10-27。此项属于原P4错误约定治理，非新增产品能力。

已确认来源：N1 `Error::from(&str)`默认为Invalid，三个 `BUSY.into()`使真实瞬时Busy被误成硬失败；candidate04已改为显式Error::busy，保持最小功能修复。另当前 `Error::from(String)`仍按完整英文文案 `document transfer state busy`识Busy，源于私有io gate返回String。这种内部语义依赖展示文字，未来改文案可能改变调度，是应清理的边界。

目标：私有本地gate/IO的Busy以类型贯穿，普通存储错误仍正确分类；删除以展示文案决定Busy的特殊分支。沿已有Doc Error/标准IO错误做小边界调整，不引入全仓错误框架、新crate或改公开协议/UI。外部未知诊断保留为诊断，用户语言在catalog边界处理；不能为统一风格吞掉错误、把Hard变Pending或反向。

先追实际io gate调用/typed kind传播与现有真实Busy用例，再实施；与N1作者协调同一文件写权，不能和当前flow修复并改。验证需真实持有gate得到typed Busy，正常释放后可继续；坏metadata/权限或IO仍显式硬错误，不能仅靠同一字符串断言。原N0/SeenPin/预算/角色/receipt不变量仍保，适用回归按影响范围。源码和运行证据齐才done。
