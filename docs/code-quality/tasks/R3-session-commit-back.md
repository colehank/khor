# Session 提交内容返回

用户指出session内commit也是工作区浏览内容，需要返回。桌面提交页现在复用原ContentToolbar，顺序为工作区/浏览器选择器、返回、内容药丸；药丸显示真实机器与仓库目录，提交标题/正文仍只在原CommitPage元数据中显示。没有改独立工作区的提交页。

返回浏览该commit所对应的设备/仓库目录，保留commit标签实体、原展开/收起和阅读状态；不使用会把文档重置成新建页的returnToPicker。浏览器切换与关闭沿原会话导航逻辑。手机保留既有内容甲板返回，使用同一工作区返回目标。

证据 `/Volumes/disk0/Khor/evidence/round3/session-commit-back`：red实际缺少返回(0!=1)；green/geometry五组Chrome实际通过，包含提交甲板列位、返回不改变commit视图、重开保留收起状态，以及原文件/草稿、独立工作区预览、手机返回。滚动后的提交文件标题实测不与新增甲板相叠；截图session-commit-back.png已查看。types/unit实际0。作者自审；无原生构建，无main合并/ship，原Lab8946保持运行，刷新原地址。
