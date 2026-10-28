# Q035 root 非作者审查

对象0f5476ee0e250ffe6247fc0acf29813b5f340c87，审查者root；补测followup c546dde5由root编写。

生产6文件无阻断：hold取得资源后依最新已发布owner union立即清理，首次未发布owner仅暂存；BrowserPane必填owner、3caller显式，MachineWeb按真实组件寿命/at发布集合，独立cleanup保留已退休空集合以接住迟到结果。SessionContent的onOpened不再提交旧闭包pageIds覆盖新集合。open/close共用per-tab序列且失败不毒化后续open，同目标StrictMode仍共享pending；close后重开不会晚到关闭新页。borrow仅in-flight session去重，保留失败不无限重试的原约定。8活页cap在每次hold即时施加，owner集合仍记录所有tab，旧页可按需重开。

固定作者报告module10/10、Chrome/WebKit组件8/8、types/unit、session-content及browser-yield通过，源指纹匹配提交。新module门发现原型名选择器漏洞：`name in cases`把toString当测试，root真实执行PASS 1/1、exit0、零断言；证据Q035/root-selector-red。root将其改Object.hasOwn，独立10/10复跑且toString/constructor均exit1，Q035/root-review；followup c546dde5。必须与原提交一起集成，不能忽略测试工具自身漏洞。

结论：两提交按序可集成。组件控制native IPC Promise不等于真实WKView/远端借网句柄计数；实际App资源退役仍待后续原生验收，不因单元/React绿自动关闭该平台边界。未知owner暂存只有初始化含义，不是永远保留路径；当前3caller均有明确集合owner。
