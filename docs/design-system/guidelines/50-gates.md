# 守门:什么必须走 token、有哪几道门、怎么加新档

> 2026-09-25 从旧 `docs/DESIGN.md` 第一至三节原样搬来(设计系统换成 `docs/design-system/` 时,这几节没有别处可去)。
> 文中「这份文档」「第二节」指的就是这一份。`npm run gates` 读这份文件:**每个 `npm run` 入口都必须在这里被点名**。

## 一、什么必须走 token

字号、行高、字重、圆角、间距、像素尺寸、动效时长——组件里出现这几类的
字面量就是在 tokens.css 之外新开了一个出处。写法上:

- 字号/行高:`text-xs/sm/base/lg/hero` 或语义层 `text-title/fact/aux`,
  每档自带行高;
- 字重:`font-body/label/title`,**不用数字名**(`font-medium` 这类)——
  数字说的是「多重」,语义层说的是「这行是什么」,前者每个人手感不同,
  后者查一次表就一致;
- 读数(百分比、token 数、字节、时刻)一律加 `tabular-nums`:
  数字不齐列,扫读就得逐个对齐小数点;
- 动效时长:CSS 侧 `--time-yield`,TS 侧 `lib/tempo.ts`——同名同值,
  型式守门核对(setTimeout 读不了 custom property,这是那对双胞胎
  存在的原因,不是允许再生一对的先例);
- 触控热区:凡手指要落的控件,热区不低于 `--touch`
  (`min-h-touch`/`min-w-touch`;依据与出处在 tokens.css 注释)。
  **扩的是热区不是图形**,且按下/选中的底色要画在扩后的盒上——
  否则手指找得到的目标,眼睛没被告知过。

**它挡不住的**:选错语义档(标签穿成标题)编译照过、守门照绿——
那要靠「一个块一个 title」的版式判断,写在 tokens.css 的 ink weights
注释里,没有机器能替人读意图。

## 二、守门不止两道——能跑的清单在 package.json,判据在这儿

新增会话运行时回归：

| 入口 | 验证 |
|---|---|
| `npm run session-content` | 正式 App + 独立节点/PTY：内容原地转换、自动保存、预览保持、会话隔离、CLI 同步与持久化、放大/恢复、拥挤时头像与两个 ＋、手机布局；需先构建 CLI 与 bridge，可用 `CARGO_TARGET_DIR`、`CONTENT_CHECK_DIR` 指定外置盘 |
| `npm run session-cwd` | cd、活动 pane、地点筛选/搜索、工作区归属、旧附栏恢复、默认跟随与主动固定目录 |
| `npm run appearance` | 真实终端与设置的预设、DIY、JSON、跨窗口同步；缓冲/滚动/PTY 不重建；背景只合成一次；能力与输入边界 |
| `npm run scroll-target` | 真 xterm 的内层 mouse 能力、分页累计/反向/停歇、触摸与滚轮不重复输入；连续输出期间每次手势只刷新一次 tmux |

可选实际 Codex 验收：`python3 apps/gui/scripts/check-codex-paging.py`。
依赖本机 Codex 与 tmux，用临时 home、虚构历史和不可达模型端点；不发送输入，
验证分页到顶到底及草稿保留。它不属于默认门。
加 `--codex <真实 Codex 路径> --mouse`，fixture 在启动 Codex 前开启独立
tmux 的鼠标，验证真实 Codex 逐行滚动及反向回到底部（草稿保留），而不只验证 PageUp/PageDown。
真实手机还要验：慢拖、快甩惯性、方向反转、输出中拖动，以及从 khor 新建
shell → 连接已存在的 tmux server → 启动 Codex 的完整流程。
`cargo test -p khor-cli --test codex_launch` 覆盖 tmux 丢环境标记的真实 PTY 启动。

`python3 apps/gui/scripts/check-codex-selection.py` 则通过真实 CLI 导入隔离 tmux
中的默认 shared-daemon Codex，验证同一行在 resume → `/new` → resume 后保持
普通 shell 形态。加 `--native-title` 显式设置 fixture 的标题配置，
另验原生标题随当前线程改变、撤销／重启用配置后同一客户端正确绑定，以及回退为
shell 后同一导入 ID 仍能重新接上 tmux 终端。只发送本地 slash 命令，不提交模型输入；预览归属由 Rust
回归验证。两项实机验收已在 macOS、Codex 0.162.1 通过；Linux/Windows 未实机复验。

**这一节的标题从 2026-08-27 傍晚起改了口,而改口的理由本身值一条判据。**
上一版叫「两道守门,各守半边」,底下就列了 `type` 和 `face` 两条。那不是
省略,是**这份文档在造假绿**:照它办事的人跑完两条,会理直气壮地在提交
信息里写「守门都绿」——**而我当天就是这么写的**,`check-px` / `check-icons`
/ `tsc` 一条都没跑,是别的车道跑自查时替我撞出来的。**「文档说有两道」和
「`package.json` 里有几个」是两件事,我拿前者当了后者。**

**所以这一节只写判据,能跑的那份清单以 `package.json` 为准。** 两者一旦
分叉,分叉本身就是下一个人漏跑的原因——而下面这张表也会过期,**出处是
`npm run`,不是这张表**:

| 怎么跑 | 是什么 | 拿 cargo 锁吗 |
|---|---|---|
| `npm run connectivity` | `check-connectivity.mjs`：真实地图/矩阵/机器详情，5 机夹具，宽屏与 390/375 窄屏；方向切换、失联来源旧路、不通字样、relay 单线头像、矩阵选择与反向、详情浮层与完整日志（不碰真实 store） | 不拿 |
| `npm run file-face` | 文件各入口默认源码、手动预览、路径/历史版本切换重置、附件只读源码，1200/390 宽度 | 不拿 |
| `npm run merge-face` | `check-merge-face.mjs`，工作区合并里每一处的「保留 / 放弃」按钮的**面**（2026-10-14）：两个主题下按钮自己的底色 alpha 是 1 且等于 `--popover`、被删的代码确实压在按钮下面（否则前一条是空的）、按钮内部画出来覆盖最多的那个像素就是面本身（底色半透时它会是底下那行代码的颜色）、标签的墨在面上仍 ≥ 4.5。改前先红：`--fill-strong` 是 0.1 / 0.15 的半透明灰，实测按钮内部覆盖最多的颜色是 `[200, 151, 148]`——红线代码透过来了 | 不拿 |
| `npm run console` | `check-console.mjs`，真实 Composer 的斜杠、机器与路径补全、命令幂等重试、结构化记录更新与会话跳转；已读仅覆盖前台且在底部实际渲染的消息 id 与版本快照，同毫秒后来消息不被误清，历史、后台和隐藏页保留未读（系统 Chrome，API fixture，不拿 cargo 锁） | 不拿 |
| `npm run test` | `pretest` → `gen` + `check-shim` + `check-px` + `check-icons` + `tsc --noEmit` | **拿**(`pretest` 是两条 `cargo run`) |
| `npm run type` | `check-type.mjs`,静态 | 不拿 |
| `npm run words` | `check-words.mjs`,静态(WORDS-1008:时间的方向只在 `time-ago` / `time-in`、`span` 不出 `words.ts`、延迟一律 `ms`、连不上叫不通、机器列表的筛选和行同一个推导、钱只有一个格式化;也在 `npm test` 里) | 不拿 |
| `npm run face` | `check-face.mjs`,渲染(自起 vite + 系统 Chrome)。`FACE_ONLY=native` 只走第 11 节「像 Mac 的 app」(选字、手、键盘、右键,一分钟);`FACE_ONLY=row` 只走第 12 节「第一行一条中线」(一分钟;2026-09-30 起每一站也量 R0 两行标题的两档行距,名字那一行 18、第二行 14);`FACE_ONLY=filter` 只走「机器、工作区两栏能筛」(一分钟,2026-09-29);`FACE_ONLY=phone` 只走第 18 节「手机不托管」(电脑看手机、本机是手机两臂,两三分钟,2026-10-02,docs/IOS.md);`FACE_ONLY=scan` 只走第 19 节「加入网络的扫码」(没有扫码器、有扫码器两臂,一分钟,2026-10-02,docs/IOS.md 第三步之二);`FACE_ONLY=bar` 只走第 20 节「窄档的会话页」(内容在上、标签与操作在下,鼠标与手指、窄与宽四种,附件键的字节、手指拖动的滚轮报告,一分钟,2026-10-02,docs/IOS.md 第三步 A;bar x(item 62 / 72):390×844 带 34 的指示条、375×667 没有，手指折起后网格止于胶囊上沿之上、胶囊的中心在最后一行的下沿(去掉滚动跟手的半行位移)和视口底的正中 ±1 px,终端字号 14 和 19 各量一次，胶囊中心按下去落在胶囊上;两个字号下「回到底部」和胶囊之间的空都等于抬之前的那段 ±1 px(ux-q,旧代码上先红：8 / 4 / 10.5 / 3.5 对 11 / 11 / 16 / 16));bar y(item 73):键盘起着(开发版的 `__khorKeyboard`)时 CDP 的手指长按再拖，焦点还在终端、格子里是「select me here\nsecond」、两个把手；视口高 120 时网格不重排、选区还在；菜单的「拷贝」(`__khorEditMenu("copy")`)写进剪贴板的是格子里的字、选区结束，压着的重排随后跑;键盘收着时 item 54 照旧：静止时焦点放开，长按不出 khor 自己的选区;bar z(item 74):自动模式下屏幕上没有光标(`?25l`)时，点一下屏幕：键盘收着就升、起着就落；程序要了鼠标时它照样收到这一下(SGR 报告),键盘也照样升;光标显示时点输入区以外、键盘收着，什么都不动(旧代码上红：起着时点一下没落);`FACE_ONLY=store` 只走「存储」(STORE-BRIEF 第一阶段:宽、窄两档的页,换机器,删除那一问,改位置与恢复默认,附栏更多里的位子,两三分钟,2026-10-07);`FACE_ONLY=mchat` 只走「对话里的文件直接传」(STORE-BRIEF 第二阶段:留言的文件卡片每一步、×、落地的路径、重试、「存到」与更改…、智能体对话的「存到」和告诉智能体的落地路径,一分钟,2026-10-08);`FACE_ONLY=bcast` 只走「广播」(STORE-BRIEF 第三阶段:那一行的种类、空白脸、第二行和词,打开后的名字、没有「存到」,邀约卡片的每一步与接收、重试、×,发端的 n/m,说话与上传都对着 `wide/all`,一分钟,2026-10-08);`FACE_ONLY=save` 只走「文件自己保存」(save-a:停手一秒的那一次写、路上只有一次写、存了能撤销、离开就写、✓ 说已保存、盘上被改了停下问、写不出去留着改动自己再试、关标签时能写就写不能写才问,路由后面是守 `write_whole` 规矩的假盘,一分钟,2026-10-08);`FACE_ONLY=notice` 只走「药丸上的通知」(NOTICE-BRIEF:手机的重新连接、长出来的起点框与左缘、一件一行、遮罩的误触、立即重试、失败的传入与件数和详情、减弱动态、宽窗的板与标签回到自己,一分钟,2026-10-09);`FACE_ONLY=afix` 只走「审查的四处 bug」(长词在框里折、诊断页的事实行、灰掉的行的字、ErrorBoundary,afix-a,一两分钟,2026-10-08);`FACE_ONLY=quota` 只走「额度页的三档两主题」(UX-53:1280 / 390 / 375 宽、亮暗两主题、两部手机用手指——列表每行的仪表盘同一个 x、和两行字一样高、钱包和读不到的行没有仪表盘,账户页每一行不折、bar ≥ 36、不越过右边线、手机没有表盘,中位线的墨和对比,model 的颜色,手机的表三列不截,三四分钟,ux-q3,2026-10-08);`FACE_ONLY=import` 只走「导入是一次搜索」(imp-a:打开时焦点在框里、只列连得上的机器的、最新的在前，打字当场筛标题 / 目录 / 机器 / 正在跑的命令、没有匹配时一句「没有匹配项」且列表不变高，机器标签「全部」默认按下、不通的机器标着且不在「全部」里、两枚是「或」，命令只画在终端行，点一行只导入那一个并关掉面板，一个也没有时一句话、没有标签和注，宽窄两档、两种主题，一分钟,2026-10-09);`FACE_ONLY=iphone` 只走第 21 节「真 iPhone 报的三处界面」(窄档回到列表没有选中行、会话行不写来自哪台机器、状态栏与主屏指示条下面的两条带子是 app 自己的底——四种手机 / app 主题组合,一分钟,2026-10-02 fix 车道,docs/IOS.md「真机报的五处」);`FACE_ONLY=term` 只走第 15 节「终端的键」(拼音下的符号 a–d、粘贴 e–h(item 55),以及拖文件进终端 i–v(item 60):开着两个终端、一个藏在同一个框里时一次放下只给露着的那个 `term_drop`(旧代码上红),框外放下不算，放的时候药丸的线按 `term_drop_progress` 填到那一格、完了就没，被拒时那句话写在终端角上，从清单里拖出来的行带着它的机器;OS 的拖放走开发版的 `khor:window-drop`(`src/lib/window-drop.ts`),真拖放在真 app 清单里;手机上的 📎 文件面板 vi 起(item 60 第 3 部分):机器单里会话所在的机器排第一、标着，不托管的机器(手机)不列;vi 还量键行右端(item 72):「…」是横滑区的最后一个，摇杆单独钉在最右，它的标记 26×26(`--key-pad-glyph`);vii 在机器的清单里点一个文件就是这个终端的一次 `term_drop`,带着路径和那台机器(本机是 null);viii 照片的隐藏 input 收到一个文件：先 `term_drop_stage` 一次(名字和字节数),再从本机 `term_drop` 暂存的路径，药丸的线立着；超过 200 MB 的在终端角上说、什么都不暂存;粘贴图片 ix–xi(item 60 第 4 部分):剪贴板里只有图片时 ⌘V 先 `term_drop_stage` 一次(名字是 `paste-<时间>.png`),再从本机 `term_drop` 暂存的路径、不打字;图片和文字一起时只贴文字、什么都不暂存;右键菜单的粘贴读到图片(`navigator.clipboard.read` 打桩)也走同一条路;xii(item 60 第 5 部分)文件面板里文件行的菜单(长按，这里用右键):shell 的当前目录已知时有「放到终端」和「放到当前目录」,后者是一次带 `into` 那个目录的 `term_drop`;目录不知道时只有「放到终端」;文件正在传入时的药丸(item 71):iii 里 Mac 的标签脸位是上传标记(`data-icon="upload"`)、名字去了扩展名、↻ 那个座位是 ×;xiii 手机甲板上一个长名字的文件传入时，脸位是上传标记、标题是去了扩展名的名字从中间对半截断、第三个座位是 ×(`data-term-drop-stop`),按 × 问一次 `term_drop_cancel`,药丸立刻回到会话自己的名字和 ↻,`term_drop` 随后答「已取消」时角上不说话、什么都没打;xiv 暂存途中按 ×:正在上传的那一个传完，不再暂存、不 `term_drop`,已经暂存的交给 `term_drop_cancel` 删掉(在 11cbac58 草稿上红：三个都暂存了、取消不带暂存的路径)) | 不拿 |
| `npm run read` | `check-read.mjs`,对话是不是还读得下去:行长、正文档位、行高、正文墨的对比,两个主题两个姿势两张对话脸(自起 vite + 系统 Chrome) | 不拿 |
| `npm run day` | `check-day.mjs`,会话那条「一天」真的上没上屏,以及它作为控件的样子(2026-09-29 ⑥:胶囊与 2 px 的空、一格不窄于一颗点加它的空、指着的读数与按开的浮层说同一段、手指点得开、一个 Tab 停点、热区不越出自己那一行,宽窄两档)。**同日 ⓪:默认收着**——新的浏览器里没有这一行也不留位;前标签的提示(查看 / 收起 / 没有记录)与 `aria-expanded`;按脸、按名字、⏎ / Space 开合;每一帧采样确认双击名字只改名、带不动;重载后记得;没有记录的一天(把 fixture 的 `tui/quiet` 在线上改成全「未查看」和没有这一项两种)不画、按了也不落;扣住 `day_strips` 时是骨架,和真带同一行同一高;窄档点底栏里前面那枚标签开合(2026-10-02 起;以前是点窄档头里的脸和名字);**落下来浮在会话上**——会话的上沿和高度一个像素不变,这一行上胶囊以外的一点落到会话(同日傍晚,旧代码上红)。其余各节都在按开之后读(自起 vite + bridge + 自备一天的数据) | 不拿(但**要求先构建**:`cargo build -p khor-gui-core --bin bridge -p khor-node --example day_fixture`) |
| `npm run acp` | `check-acp.mjs`,ACP 那一面有没有真的到屏幕上。两批:**协议里带状态的帧**(`tool_call_update` 折进对应那一行、三个状态三种看得出的墨、失败穿 `失败` 自己那支、关掉动效之后事实还在);**新会话(UX-45 起是新标签页的页面)里的注册表与历史对话**(「还没问过 / 它答不了 / 它说它没有 / 它列出来了」是四句不同的话且两两不同、坐着两个轮询周期一次 `agent_ask` 都不许发出去而按一下正好一次、发二进制的那些不长「接进来」、「接进来」和「点开一段」真的把 id 送上了线)(自起 vite + 系统 Chrome) | 不拿 |
| `npm run fast` | `check-fast.mjs`,「快」那三行(`30-behavior.md`):bridge 下挂一块 `statfs` 要 10 秒的**假**卷(`BRIDGE_SLOW_VOLUME_MS`,不碰真盘),机器页 1 秒内列出这台机器、1 秒内出带读数的卡片;离开再回来,**第一帧**就有卡片、读数和进程行(自起 vite + bridge + 系统 Chrome) | 不拿(但**要求先构建**:`cargo build -p khor-gui-core --bin bridge`) |
| `npm run rail` | `check-rail.mjs`,导轨的几何与图标 | 不拿 |
| `npm run native` | `check-native.mjs`,**AppKit 画的东西**:在临时 `KHOR_HOME`(不碰负责人的 store)里起**构建好的** app,用 `khor config set theme` 切深 / 浅 / 跟系统再把窗口切回前台,每一步截窗口量两件事——三颗窗口按钮左缘在 `traffic::X`、中心在第一行中心;左块的材质与板的深浅跟着主题。只在 macOS;不动鼠标,但会把窗口提到前台几秒(2026-09-27 左块变白、按钮跑位之后加的)。**2026-09-28 起多一段键盘**(`NATIVE_ONLY=keys` 单跑、`=theme` 只跑上面那段):按 pid 读菜单栏(五个菜单、编辑菜单那六项、每项写着的快捷键)、用 AXShowMenu 问右键(页面与 R0 标题 0 个菜单,搜索框 1 个)、**按键只投给临时 app 的 pid**(⌘5、⌘N 两下、⌃⇥、⇧⌘[、⌘W、⌘1、没有标签时的 ⌘W),数临时 store 的 `.khor/tabs.json`——一次按键跑两次藏不住;再从菜单栏点一次「新标签页」,正好多一个。同一段还读**菜单栏的名字与语言**:app 菜单的粗体标题是「khor」(不是进程名 khor-gui),编辑、窗口的每一项——AppKit 自己加的那些也算——中文里没有拉丁字母;末了以 `lang en` 重起一次再读,不许有汉字(只读、不按键、不提到前台;在中文桌面上守住「跟 khor 的语言,不只跟系统」)。2026-09-28 旧代码上红过:「khor-gui」、编辑 3 项与窗口 8 项英文。⚠️ 读菜单**不用 System Events**:它按进程名找进程,本机每个 khor-gui 都叫 khor-gui,第一版读到的是负责人自己那个 app 的菜单 | 不拿(但**要求先构建**:`npm run build` 再 `cargo build --profile local -p khor-gui -p khor-cli`) |
| `npm run mobile` | `check-mobile.mjs`,**app 还编得过 iOS**:`cargo check --target aarch64-apple-ios -p khor-gui`,外加 `run` 还带着 `tauri::mobile_entry_point`(没有它库照样链接,手机上的 app 起来什么也不跑)。桌面构建看不见另一半:`desktop` 是 tauri-build 在桌面上设的,`#[cfg(not(desktop))]` 那一半从没被编过——2026-10-02 量到它已经烂了,四个只在桌面的命令照样注册、`run` 照样调只在桌面存在的 `menu`,九个错(旧代码上红,docs/IOS.md)。只在 macOS;要 `rustup target add aarch64-apple-ios` 和构建好的 `dist`;第一次把所有依赖为 iOS 编一遍(几分钟)。**2026-10-02 起多两条静态的**(fix 车道,旧代码上红):④ 同步的 `#[tauri::command]` 都在 `QUICK` 名单上(只碰内存、本机一个文件或 AppKit),会等的(网络、远端机器、进程起来、冷读盘)一律 `async` 经 `off_main`——tauri 把同步命令放在主线程上跑,`term_open` 在那里等一台掉线的机器,整个 app 就停着;名单两个方向都红;⑤ `set_window_theme` 有 iOS 那一半、设 `overrideUserInterfaceStyle`(状态栏的墨跟 app 的主题) | **拿** |
| `npm run palette` | `check-palette.mjs`,两个主题下的可读性；`PALETTE_ONLY=rows` 只测基础调色板与会话行，跳过终端和头像像素扫描 | 不拿 |
| `npm run tabs` | `check-tabs.mjs`,浏览外壳的契约面 + 标签,**真连**(两个 home 走真 UDP) | 不拿 |
| `npm run smoke` | `smoke.mjs`,整条验收,**真连**(三个 home) | 不拿 |
| `npm run web-face` | `web-face.mjs`,浏览器脸,**真连**(真的 `khor serve`) | 不拿 |
| `npm run gates` | `check-gates.mjs`,守门自己够不够得着、写没写下来 | 不拿 |
| `npm run bytes` | `check-bytes.mjs`,源码里有没有**字面**控制字符(写成转义的不算) | 不拿 |
| `npm run boot` | `check-boot.mjs`,第一帧的五个颜色和窗口的两块底是不是都来自 `--background` / `--card`——白屏一闪是主题还没到就画了 | 不拿 |
| `npm run shim` | `check-shim.mjs`,借网页面里那两个漏真实 IP 的 API 还在不在(静态:把注入脚本放进一个 `vm` 里跑) | 不拿 |
| `npm run wk` | `check-wk.mjs`,同一段注入脚本在**真 WebKit** 里到得了哪几个 realm(起一个 `WKWebView`,同一页加载两遍、一遍不带 shim)。**macOS 专有**,要 `swift` 和一个窗口服务器;别的平台上它说一句「这儿量不到」就停,不算通过 | 不拿 |
| `npm run replies` | `check-replies.mjs`,终端对查询的应答(DA、OSC 颜色、DECRQSS、`CSI ? u`……)和按键、粘贴、鼠标报告分得开(`src/lib/term-replies.ts`;重放时只静音应答,一个键也不吞)。静态,也在 `npm run test` 里 | 不拿 |
| `npm run attachments` | `check-attachments-model.mjs`，生产附件选择和暂存逻辑：跨来源父子包含、Windows 路径、目录层次与空目录、暂存归属、取消；正式界面另跑 `FACE_ONLY=term npm run face`（读取不发送、只读原文、多机器选择、目的地、目录上传、取消和失败重试），甲板与键行回归 `FACE_ONLY=bar npm run face` | 不拿 |
| `npm run keys` | `check-keys.mjs`,程序(或替它的 tmux)要了扩展键之后，带修饰的键发什么(`src/lib/term-kitty.ts`,item 39):什么都没要时交给 xterm，只有 ⇧Enter 发 `ESC CR`(item 59,即 Meta+Enter);xterm 的 modifyOtherKeys(`CSI > 4 ; 2 m`,tmux 对带 `extkeys` 的外层终端要的就是它)下 ⇧Enter 发 `CSI 27 ; 2 ; 13 ~`,⌃/⌥ 组合同理，第 1 级不动有自己控制字符的组合,`CSI > 4 m` 和 RIS 关掉;kitty 标志(`CSI > 1 u`)在时 kitty 优先发 `CSI 13 ; 2 u`。按键条的「换行」键(⇧⏎,item 40)在每种状态下发的都和 ⇧Enter 一样，并且按住会连发。⌃ 和 ⌥(item 69):按一下等一个键，连按两下锁住、之后每个键都带它，再按一下松开，第二下来晚了就是松开、什么都不发。Node 里跑真 xterm,不开浏览器；也在 `npm run test` 里。穿过真 tmux 的那一半是 `cargo test -p khor-node --test tmux_extkeys` | 不拿 |
| `npm run dial` | `check-dial.mjs`,我的 khor 表盘的算法(`src/lib/dial.ts`、`src/lib/latency.ts`,UX-26):每毫秒落在哪个半径(0 在内圈、30 和 120 是两个圈、≥300 和不通在外圈,对数刻度、单调)、名字按角度落在脸的哪一侧(背着圆心,正侧方落到脸下)、路从第一张脸边外 3 到第二张脸边内 3、四档延迟(30 / 120 的边、四支墨是四个 token)、按名字均匀排开且记住的角度保留、两台机器之间认什么路(本机的线两头同读、别的机器读它们写的、老机器没写是未知不是不通)。Node 里跑,不开浏览器;也在 `npm run test` 里 | 不拿 |
| `npm run tokens` | `check-tokens.mjs`,token 数的缩写(`src/lib/tokens.ts`,UX-50,负责人 2026-10-08):千进位,K / M / B,哪种语言都一样——不用字节的 G,中文也不换成万 / 亿;用量半页(轴、合计、表、悬停)和输入框的上下文读数都从这一个函数走,不许各自再写一个。Node 里跑;也在 `npm run test` 里 | 不拿 |
| `npm run tap` | `check-tap.mjs`,自动模式下点哪里会弹键盘(`src/lib/term-marks.ts` `inInput`,item 43),Node 里跑真 xterm 的缓冲区:Claude Code 的输入框(`─` 上下沿夹着 `>` 那行)算框加上下各两行,再多不算——框下面空着也不算到屏底;光标裸露一行时算那行加上下各一行;没标记的 shell 在提示符上时算那行、上一行和下面的空白(item 28 的裁定);tmux 状态行在最后一行不碍事;点一下要键盘做什么(`src/lib/term-tap.ts` `tapAsks`,item 49):自动模式点输入区弹、点别处收,屏幕模式哪里都弹从不收,按钮模式不管;屏幕上没有光标时自动模式是「收着就升、起着就落」(item 74),光标的显隐按 xterm 解析到的 DECTCEM 读(`watchesCursor`),RIS、DECSTR 之后算显示 | 不拿 |
| `npm run snapshot` | `check-snapshot.mjs`,主机留的屏幕快照(`crates/node/src/host.rs` `Model::snapshot`,item 46,`khor-host/3`)喂给真 xterm 之后,和把原始字节喂给另一个 xterm 一模一样:每行文字、每格颜色与属性、每格的 OSC 8 链接(item 66)、折行标记、光标、主屏/备用屏、鼠标与粘贴模式;素材在 `scripts/fixtures/snapshot/`(`<名>.raw` 程序的字节,`<名>.snap` 主机的快照,由 `cargo test -p khor-node --lib writes_snapshot_fixtures -- --ignored` 写出) | 不拿 |
| `npm run peek` | `check-peek.mjs`,一块输出算不算「数据」(`src/lib/term-peek.ts`,roadmap 3.4):真的表格、JSON 读成数据,真的普通输出读成文字。静态,也在 `npm run test` 里 | 不拿 |
| `npm run atlas` | `check-atlas.mjs`,xterm WebGL 的字形图集(item 24):先证明探针看得见故障(不带模块时一个终端清图集会改掉另一个的画面),再量一个终端清图集不动邻居、晚到的字体画出来的 CJK 和新开的终端一致、丢了 WebGL 上下文立刻换 DOM 渲染、回来画得一样(真 xterm,系统 Chrome 带 WebGL) | 不拿 |
| `npm run links` | `check-links.mjs`,终端输出里的路径和网址(`src/lib/term-links.ts`,item 53 补充):网格折行的路径从任一行都是一个链接、行号列号还在;折行的网址点第二行也是整个网址;任何文字的名字(`报告/图.png`)和带空格的引号名字都算路径;机器上没有的不算(真 xterm,node;`npm test` 里有) | 不拿 |
| `npm run select` | `check-select.mjs`,手机上的选字是 iOS 自己的(`src/lib/term-select.ts`,item 54):盖在网格上的透明文字层，三种字号、带中文宽字和折行，每个格子的左、上、宽都和 xterm 画的格子差不到半像素，缩放重排后也一样;菜单的「拷贝」取格子里的字(折行接成一行、行尾空白去掉),不取层的行;键盘收着时终端静止不拿焦点(WebKit 在手指落下时就决定长按是选字还是进输入框),硬件键盘的字和 Enter 照样到终端;选区在时 Esc 结束且不发、打字结束且发;键盘起着时选字不放焦点、键盘不落(item 73):按住 400 ms 在 xterm 的格子里选中手下的词、两个把手，接着拖就延长，拖尾把手改终点，抬手把格子里的字和菜单的位置告诉面板，点选区再出菜单，点选区外或 Esc 结束，都不动焦点、不发字，双击三击也一样(旧代码上红：按住就放开了焦点);双击取词、三击取折行的整行;块的提示行长按出块菜单、不可选(真 xterm,系统 Chrome;`npm test` 里有) | 不拿 |
| `npm run scroller` | `check-scroller.mjs`,终端历史走页面自己的滚动(`src/lib/term-scroller.ts`,items 29、30、36、49、50、52、61):框像键盘升起那样一步步变矮之后还在底部;Ink 式的 40 行画面以 10 Hz 重画时手指往上拖,抬手前滚动位置只有手指动过,抬手后只放一次;程序拿着鼠标时 300 px 的拖动是 18 格滚轮;收键盘的下滑(`src/lib/term-swipe.ts`,item 49)是任何往下的一划——出了 10 px 死区、竖多于横就收,屏幕高处 20 px 也收,再慢、中途停过也收,只收一次;死区以内、横多于竖、往上、先按住成了选择、键盘没起、从吧台起手的都不收;手指收起卡座(`swipeFolds`,item 50):往下 60 px 收一次,往上 30 px 展开,折返从手指最远处算;太短、横着、先按住、从吧台起手、键盘起着的都不报;每一划写一行给诊断的「终端」行(item 61 a):方向、距离、抬手时的速度、报了什么或被哪条规则挡下、卡座前后的样子,点按不写;一甩就收(`FLICK` 0.2 px/ms,item 61 b):照模拟器实际送来的触摸流(60 Hz,26–93 ms 里 15–30 px)合成的甩动收起卡座、往上甩展开,450 ms 才走 30 px 的慢拖、抬手前停了 150 ms 的、横着的、键盘起着的都不报;「新输出」只数行(`src/lib/term-gain.ts`,item 52):翻在历史里时,原地重画的提示符、缩放后的重画、光标开关、标题、清屏都不算,真的多出三行才算,历史满了在顶上裁行时也算(真 xterm,系统 Chrome) | 不拿 |
| `npm run softdel` | `check-soft-delete.mjs`,iOS 系统键盘的删除键按住会一直删(`src/lib/term-soft-delete.ts`,item 35):把模拟器录下的事件(`scripts/fixtures/ios-delete.json`)重放进真 xterm,按住 40 下发 40 个 DEL、加速到删词发 ⌃W、拼音上屏不变、硬件键盘照旧、占位字符一个也不进终端;不带模块时按住只删一个(旧代码上红);以及 item 37:拼音拼到一半键盘收起(没有 compositionend)时组字被丢掉、一个字母也不进终端、下一个键干净(不带模块时下一个键会带出半截拼音)(系统 Chrome) | 不拿 |

**最后那批「真连」的和上面几道之间有条分界,值得写下来**:有入口的前几道
都是「一台机器上自己跑得起来的」,真连那三道要**真的起网**——所以跑它们
之前先问一句机器上有没有别人在跑。

**墓碑:上一版这里写的是「最后三行没有 `npm run` 入口」。** 那是真的,而且
它比「文档说二」更糟一档:这份文档第四节自己写着「守门在
`scripts/check-tabs.mjs` 第 0 节」,而 `npm run tabs` 答的是
`npm error Missing script: "tabs"`——**一个被文档称作守门、而 `npm run`
起不动的东西,读起来就像它不存在**,连「我漏跑了」的机会都不给。
2026-08-27 傍晚补齐了三个入口,**并且加了 `npm run gates` 来守它**:
下一个人加第十个脚本而不给入口,那道守门会红。

### 跑哪几道:按你动了什么,不按印象

- **动了 `src/` 里任何一行** → `npm run test`(px / icons / tsc)。它是唯一
  一条「不管改了什么都要跑」的,**也是唯一一条要 cargo 锁的**——见下面那个数;
- **动了字号/字重/圆角/间距/热区,或它们的 token** → 加 `npm run type`(静态)
  和 `npm run face`(渲染)。**两条都要**:一条证明没人写字面量,另一条证明
  规则真落到了屏上,少任何一条另一条都只是一半;
- **动了颜色或 `tokens.css` 的调色板** → 加 `npm run palette`;动的是底(`canvas` / `side` / `card` 一张底)、`popover`、`hairline`、`glass` 时再加 `npm run rail`(图标条的字形量在玻璃上)、`npm run read`(对话的墨量在底上)和 `npm run face` 的 4I(真浮层比板高 4、板四周与板同色)——2026-10-02「一张底加灰线」;
- **往源码里写过任何非文本的字符**(分隔符、哨兵、终端序列)→ 加 `npm run bytes`。
  它守的不是「程序里能不能有这个字符」——`lib/model-band` 里那个分隔符运行时
  就是它,写成转义,完全正确。**它守的是这份源码还搜不搜得动**:
  2026-09-08 `UsageHalf.tsx` 里四个字面 NUL 让 `file` 判它是 `data`,而带 `-I`
  的 `grep` 静默跳过整个文件,于是那一晚三条「grep 没找到」的结论全是猜的。
  **「我没找到」和「我没看」是同一个输出**,而这条把它们分开。
- **动了会话那条「一天」(`DayStrip`、`use-day-strips`、`use-day-shown`,标签条的 `again`、`EditableName` 的 `useSinglePress`),或它背后的 `khor_node::day`** → 加 `npm run day`。
- **动了 `cagent`/`codexagent` 往外发的 `SessionUpdate`,或 `ChatView` 那个 fold** → 加 `npm run acp`。
  它守的是**协议的状态有没有到屏幕上**:今天 khor 自己的垫片是唯一的来源,接了注册表里的 agent 之后,
  同样的字段会从线上真的来——那时候「掉在 `default` 分支里」就不是「还没做」,是**收到了然后扔掉**。
  它守的是别的门**结构上看不见**的那一样:那条带子有没有真的画在屏上。
  它立于 2026-08-30——那条带子发出去过一版**一格都没画**,而当时
  `cargo test`、`tsc`、`check-px`、`check-icons` 全绿、`pageerror` 为空。
  没有任何一道门退化了,它们从来就量不到这件事;
- **动了新标签页的页面(`views/NewSessionPage`,原来的新会话对话框)、`khor_gui_core::agents`、或注册表快照** → 也加 `npm run acp`
  (2026-08-31 起它多了第二批)。这一批守的不是「画没画」,是**四种答案有没有被压平**:
  「还没问过」「它答不了」「它说它没有」「它列出来了」在屏幕上必须是四句不同的话,
  而一个空列表会把前三种画成同一个样子——人的下一步却是去问 / 别再问 / 开新的,
  三个方向。**「两两不同」那半才是扛事的那半**:三个分支印同一句话,逐句检查全过。
  它还守着那条最贵的:**问一次要起一个真进程(对真 `@claude-acp` 实测 1.5 秒,
  npm 包没缓存时 5.64 秒),读记下来的同一个答案 0.017 秒**,
  所以坐着两个轮询周期必须一次 `agent_ask` 都发不出去;
- **动了 `apps/gui/core/src/page_shim.js`、`open_page`,或任何一端新建被浏览
  页面的 webview** → 加 `npm run shim`。它守的是**一件不会自己喊的事**:
  借网页面里 `WebTransport` 和 WebAuthn 走的是这台设备自己的网,不是借来
  的那条——**不报错、页面照常用**,唯一看得出来的地方是别人日志里的一个
  地址。所以这道门坏了不会有任何症状。它带一条对照臂(`credentials.get`
  的 `password` 分支必须放行),否则「拒绝一切」和「拒绝那条漏」在另外三条
  上出的是同一片绿。
  **两道门分工不同,都要跑**:`shim` 是静态的,答「规则写对了」;`wk` 起一个
  真 `WKWebView`,答「WebKit 拿这段脚本会怎样」——删得掉吗、进得了哪几个
  realm。后者带一条不带 shim 的对照臂,没有它,「WebTransport 没了」和
  「这个 WebKit 本来就没有」出的是同一片绿。
  **两道都答不了的**:khor 自己那个 page webview 里 WebKit 有没有真把脚本
  交出去。`wk` 建的是它自己的 `WKWebView`,不是 tauri 建的那个;tauri 那条
  路只核到源码级。真答它要起 app,而这仓所有浏览器守门跑的都是网页脸,
  够不着原生子 webview。
- **动了列表的筛选**(`src/lib/list-filter.ts`、`lib/session-filter.ts`、`DevicesList` / `WorkspacesList` / `QuotaLanding` 的轴、`PaneBar` 的筛选菜单、`Omnibox` 的小标签) → `FACE_ONLY=filter npm run face`,再跑一遍整个 `npm run face`(会话那几节和额度的 8a 筛选在走查里,不在这一段);
- **动了额度页**(`views/QuotaLanding`、`views/UsageHalf`、`lib/model-band`、`--model-*` / `--chart-median`) → `FACE_ONLY=quota npm run face`(UX-53 的三档两主题),再跑 `FACE_ONLY=walk npm run face`(走查那一大段,额度原来的几节都在里面)和 `FACE_ONLY=corners`(corners c),节点那一半(`khor_node::usage` 与账本)→ `cargo test -p khor-node --lib usage` + `cargo test -p khor-node --test usage`;
- **动了导轨** → 加 `npm run rail`;
- **动了第一行**(`--pane-head`、`--tab-lip`、`src-tauri/src/traffic.rs`、标签条的签与脚、R0 的标题) → `npm run boot` + `FACE_ONLY=row npm run face`,构建后加 `npm run native`;
- **动了 R0 的第二行或名字旁边的字**(`PageSubtitle` / `PageTitleAside` 的座位、各页写进去的事实) → `FACE_ONLY=row npm run face`(行距与中线),再跑整个 `npm run face`(4l 读工作区那两处的字,额度那一节在 R0 里读套餐与读数年龄);摆放看 `30-behavior` 的真 app 清单;
- **动了会话的形态**(`crates/node/src/live.rs` 的座次与 `seat`、`adaptor::claude::hook` 的 `KHOR_SESSION` 路由) → `cargo test -p khor-node`(三条「形态随进程变」的假进程树测试,docs/SESSION.md),侧边栏怎么跟着变看 `30-behavior` 的真 app 清单;
- **动了主题、窗口外观、`src-tauri` 里的窗口或标题栏** → 构建后加 `npm run native`;
- **动了 `src-tauri/src/lib.rs` 的命令表或 `run`、任何 `#[cfg(desktop)]`、`src-tauri/Cargo.toml`,或 `khor-node` 里按平台分叉的地方** → 加 `npm run mobile`(拿锁);新命令只在桌面有的,旁边给一个 `#[cfg(not(desktop))]` 的另一半;
- **动了「能不能托管」读到的地方**(`DeviceRow.hosts`、`src/lib/here.ts`、读它的入口:新会话、机器页的按钮与进程表、钩子、VS Code、拉过来、导入、安全页的钥匙、设置的打开、锁屏;节点里的 `refuse_unless_hosting` 与 `Discovery` 关死) → `FACE_ONLY=phone npm run face` + `cargo test -p khor-node --lib -- cannot_host refuses_a_session checklist_is` + `cargo test -p khor-node --test no_host`,再加 `npm run mobile`;摆放看 `30-behavior` 的真 app 清单(2026-10-02 那一段);
- **动了加入网络或扫码**(`src/views/PairDialogs.tsx` 的 `JoinDialog` / `ScanLayer`、`api.ts` 的 `scanTicket` 一族、`app.css` 的 `data-scanning`、`scanner_here`、`src-tauri/capabilities/scan.json`、Info.plist 的相机那句) → `FACE_ONLY=scan npm run face` + `npm run mobile`,再跑整个 `npm run face`;真机扫码看 `30-behavior` 的真 app 清单(2026-10-02 第三步之二);
- **动了前后台**(`khor_node::link` 的 `resume` / `serve_resumable` / `fresh_ears`、`ipc::Op::Rejoin`、`khor_gui_core::rejoin` 与终端、对话的读线程、`src-tauri` 的 `watch_the_front` / `lifecycle_act`、`use-woken`) → `cargo test -p khor-node --test resume` + `cargo test -p khor-gui-core --lib` + `npm run mobile`,动了 ipc 再加 `cargo test -p khor-cli`;模拟器里切后台再回来量一次(docs/IOS.md 第三步之二);
- **动了宽档的标签条或新标签页**(`browse/TabStrip` 与 `MidName`、`browse/Tabs`、`hooks/use-session-tabs` 的 `new-tab:` 与 `become`、`App` 的 `openNewTab` / `leaveNewTab` 与每一条进新建会话的路、`views/NewSessionPage`、`FaceMark` 的文件脸、`--tab-w-even` / `--tab-w-floor` / `--container-tab-*` / `--new-tile`) → `FACE_ONLY=tabs npm run face`(UX-45:等宽与最窄、×、＋ 与新标签页、取消回到原处、路带来的预填、按着关时不动、文件的脸、`MidName`),加 `FACE_ONLY=side`(附栏头的位子条、宽档 side e)和 `FACE_ONLY=phone`(新标签页上的机器与类型);改了页面的注册表加 `npm run acp`;摆放看 `30-behavior` 的真 app 清单(2026-10-07 UX-45 那一段);
- **动了存储**(`views/StorePage`、`DirPicker` 的 `fixed`、`DeviceRow` 的 `receive_*`、`App` 的 `moreStore` 与附栏的 `store` 位子、`khor_node::road` 的 `receive_place` / `follow_receive_dir`、`TransferKind::close`、`khor_node::watch` 的 `unreported`、`IconStore`) → `FACE_ONLY=store npm run face`(宽、窄两档的页、换机器、删除那一问、改位置、附栏的位子、g 只在变了的时候重读、h 下拉刷新),加 `FACE_ONLY=side`(更多菜单、side l 的存储);动了节点再跑 `cargo test -p khor-node --lib -- road:: closing_a_transfer` + `cargo test -p khor-node --test transfer` + `cargo test -p khor-sync --lib receive`;摆放看 `30-behavior` 的真 app 清单(2026-10-07 存储那一段);
- **动了对话里的文件**(`views/MachineChat` 的 `FileChip`、`views/chat-files` 的 `SendInto` / `usePendingFiles`、`ChatView` 的「存到」、`api.ts` 的 `uploadFile` / `chatPut` / `chatPutPaths` / `retryTransfer` / `stopTransfer`、`khor_sync::chat` 的 `into` 与 `Fate`、`khor_node::link` 的 `take_arrived` / `run_take` / `take` / `stop_take`、`khor_gui_core` 的 `into_arg` / `chat_put*` / `deliver_blocking`、CLI 的 `send --into` / `retry`) → `FACE_ONLY=mchat npm run face`;动了节点或记录再跑 `cargo test -p khor-sync --lib chat` + `cargo test -p khor-node --lib` + `cargo test -p khor-node --test transfer` + `cargo test -p khor-cli --test drop_road --test pull_road --test chat_upload` + `cargo test -p khor-gui-core`;smoke 25j(右下角)走的是这条路:往一个还没有的目录发、中断、建目录后 `khor retry`;摆放看 `30-behavior` 的真 app 清单(2026-10-08 那一段);
- **动了广播**(`khor_node::wide`、`khor_sync::chat` 的 `WIDE` / `board_dir` / `set_fate_of` / `home`、`khor_node::link` 的 `known_wide` 与 `with_doc!` 的 `wide/` 那一臂、`take_local` 里广播的那几行、`road::sent_key`、`list::GROUP_KHOR`、`khor_gui_core` 的 `wide_log` / `tell` / `send_path`、`views/MachineChat` 的 `wide`、`BrandIcons` 的 `broadcast`) → `FACE_ONLY=bcast npm run face`;动了节点或记录再跑 `cargo test -p khor-node --test broadcast` + `cargo test -p khor-sync --lib chat` + `cargo test -p khor-node --lib` + `cargo test -p khor-node --test transfer`;动了对话卡片再加 `FACE_ONLY=mchat`;摆放看 `30-behavior` 的真 app 清单(2026-10-08 广播那一段);
- **动了会话的附栏**(`views/SessionSide`、`SessionPlaceSheet`、`SideRecent`、`SideWorkspace`、`components/SideFind`、`BrowseLanding` 的 `bare`、`FilePage` 的 `docked`、`hooks/use-session-side` 的位子与 `keepSidePlace`、`App` 的 `resumeSide`) → `FACE_ONLY=side npm run face`(宽档那一栏;side l 是 UX-47 的「再开回到离开时的地方」)和 `FACE_ONLY=bar npm run face`(手机的面板,bar p),再跑 `npm run day`(⓪ 读「过去24小时」);动了附栏或工作区会话栏的开合与窗口(`App` 的 `sideHold`、`api.ts` 的 `widenWindow` / `giveBackWindow`、`src-tauri/src/room.rs`) → 加 `cargo test -p khor-gui --lib room`,负责人空闲时 `NATIVE_ONLY=room npm run native`;摆放看 `30-behavior` 的真 app 清单(2026-10-07 UX-40 那一段);
- **动了窄档的会话页或触屏的东西**(`components/SessionBar`、`views/TermKeys`、`TabStrip` 的 `at`、`lib/viewport`、`index.html` 的 viewport、`--key-row` / `--safe-*`、`TerminalPane` 的按键与拖动、对话输入框的座位) → `FACE_ONLY=bar npm run face`,再跑整个 `npm run face`(§12 窄档、4J (b)、8b、第十四轮 (e)、列表 (6) 都读窄档的会话页)和 `npm run day`(⓪g 按底栏的标签);动了 `src-tauri` 或 Rust 再加 `npm run mobile`;摆放看 `30-behavior` 的真 app 清单(2026-10-02 第三步 A 那一段),手机上的事只有模拟器或真机看得到;
- **加了、改了 `src-tauri/src/lib.rs` 里的命令,或命令背后会不会等**(网络、远端机器、起进程、冷读盘) → `npm run mobile`(第 4 条读同步命令的普查:会等的写成 `async` 经 `off_main`,不等的加进 `QUICK`);动了 `khor_gui_core::term::term_open` / `chat::chat_open` 的锁或 `landing` → `cargo test -p khor-gui-core --lib`;动了 `host::connect_at` → `cargo test -p khor-node --lib handshake`;卡不卡在模拟器里量(rAF 的空档,docs/IOS.md「真机报的五处」);
- **动了远端怎么够到一行的终端**(`Request::Reach`、`LiveKind::terminal_address`、`Node::attach_tmux` / `session_dir` / `is_hosted` 在不能托管的机器上) → `cargo test -p khor-cli --test far_view`,再加 `cargo test -p khor-node --test no_host`;
- **动了列表的选中(`SideList` 的 `data-selected`、`FileTree` 的打开文件)、会话行的第二行、窄档的安全区(`data-safe-top`、`--safe-*` 落在谁身上)** → `FACE_ONLY=iphone npm run face`,再跑整个 `npm run face`;状态栏的墨(`set_window_theme` 的 iOS 那一半)→ `npm run mobile`,看模拟器截图;
- **动了快捷键(`src/lib/shortcuts.ts`)、菜单栏(`src-tauri/src/menu.rs`)、`[menu]` 的字、选字或光标的规矩** → `FACE_ONLY=native npm run face`,构建后加 `NATIVE_ONLY=keys npm run native`(它按键,只投给临时 app);
- **动了一个时间、延迟、金额或机器连接的说法**(`src/words.ts` 的 `ago` / `agoOf` / `until` / `lasting` / `latency` / `money` / `moneyIn` / `connectionKey` / `downWord`,或 `zh.toml` 里带时间、`ms`、连接状态的句子) → `npm run words`;动了行上的词再跑 `FACE_ONLY=corners`、`FACE_ONLY=entity` 和 `FACE_ONLY=filter`(机器列表的「连接方式」);
- **动了药丸上的通知（`components/PillNotice`、`lib/pill-notices`、`SessionBar` 的甲板、`TabStrip` 的当前标签，或终端 / 对话里出通知的地方）** → `FACE_ONLY=notice npm run face`,再跑 `FACE_ONLY=bar` 和 `FACE_ONLY=term`;
- **动了对话的气泡与正文（`MachineChat` 的 `data-msg-text`、`ChatView` 的 `data-chat-column` 与工具行标题）、「…」页的事实行（`MoreFact`）、列表行的灰（`SideList` 的 `dim`、`MeshNow`）或一处接缝（`components/Fault` 与 `App` / `main.tsx` / `ui/dialog` / `SideHead` 里用它的地方）** → `FACE_ONLY=afix npm run face`;动了 `MoreFact` 再跑整个 `npm run face`(4I 量设置行的右缘和行高);动了 `dim` 再跑 `FACE_ONLY=corners` 和 `FACE_ONLY=entity`;
- **动了任何列表一行的四个角(`SideList` 的 `Corners` 与 `data-corner-keep`、机器 / 会话终端行 / 额度 / 工作区 / 最近 / 浏览器的 `sub` 与 `corners`、`DoingLine`、`DeviceRow.since_ms`、`WorkspaceRow.git`)** → `FACE_ONLY=corners npm run face`(UX-48:每个列表宽窄两档读左下、右上、右下,离线、空闲的 shell、游离 HEAD、普通文件夹;UX-50:机器名后的延迟小一号、`secondary`,左下最多三项(CPU、内存、有显卡是 GPU 否则磁盘)标签 / 值两支墨、`gap-row` 隔开不带「·」,默认列宽三项都在,两种机器各把一行逐 2 px 收窄量出先去第三项、再内存、CPU 不走、不裁一个数;UX-51 的 corners d:工作区的分支在名字后面、次级墨小一号、窄档从中间裁,右上逐个符号的 token 和颜色、五个操作词、干净的不画、⇡,亮暗两个主题在底和选中行上 ≥ 4.5),再跑 `FACE_ONLY=entity`、`side`、`walk`(4f、4o 读工作区行);动了 `khor_node::git::segment`、`GitSegment` 或三支 `git-remote` / `git-untracked` / `git-stash` → `cargo test -p khor-node --test workspace_facts`(操作词、没有上游的 ⇡)、`cargo test -p khor-node --lib workspace::`(普通文件夹的新建 / 修改)和 `npm run boot`;
- **动了终端行的第二行(`components/DoingLine`、`Doing.last_line`、`commands::Tail`)或活着的 shell 的失败(`live::failed_command`)** → `FACE_ONLY=corners npm run face`(corners b 的八行),`cargo test -p khor-node --lib commands::` 和 `--lib live::tests::a_shell_whose_last_command_failed`;
- **动了终端的历史命令(`views/TermHistory`、`TerminalPane` 的 `__history`、`Doing.pane_unmarked`)** → `FACE_ONLY=term npm run face`(history a、b 两例:人自己的 tmux 前面那格不发命令标记时只有启用标记的那一行和复制,没有列表和搜索;发标记或不知道时照旧搜索,SESS-BRIEF 3,2026-10-07);
- **动了终端的查找条(`lib/term-find`、`TerminalPane` 的 `look` / `toggle`)或会话的铃声标记(`lib/bell-marks`、`components/BellMark`、`Doing.bell_ms`)** → `FACE_ONLY=term npm run face`(term find:区分大小写与正则两个开关、没写完的正则是没有匹配项、按设备记住、开关不抢光标,item 70;term bell:不在屏上的会话响铃后行上有徽标的点,在屏上的没有,第一次看到之前的铃不算,item 67);
- **动了 `src-tauri/Info.plist`、`src/main.rs` 里嵌它那一行、或 `menu::speak_at_launch`** → 构建后 `NATIVE_ONLY=keys npm run native`;往 Info.plist 里加 `CFBundleIdentifier` 之前先读 `30-behavior.md`「菜单栏」——它会把页面的 localStorage 挪到一个空目录;
- **动了 `src/browse/`** → 加 `npm run tabs`;
- **动了协议、后端、或 `khor serve`** → `npm run smoke` / `npm run web-face`,
  而这两条要起真网,先问一句机器上有没有别人在跑;
- **往 `scripts/` 里加了东西,或改了 `package.json` 的 scripts** →
  `npm run gates`。

**这张表不再只靠人记得。** `npm run gates` 守住两件事:`scripts/` 里每个
`.mjs` 都有入口起得来(或者被一个有入口的 import 到),以及**每个入口都在
这一节里被点过名**——所以加一道守门而不写进这张表,会红。

**它仍然挡不住的**:这一节说的话**是不是真的**。`gates` 认的是
「`npm run face` 这个串出现过」,不是「关于 face 那句描述没过期」;
**一句写反的描述照样过关**。那还是只能靠读。

### 「我这条很轻」在有锁的世界里不是一个量纲

2026-08-27 由 `lane-ui2` 实测那条**最轻的生成命令**(数字从它的输出里抄,
不是我跑的——为了核一个 42 秒的锁再去占一次那把锁,正是这一节要防的事):

```
cargo run -q -p khor-catalog --bin gen_ts
0.06s user  0.04s system  0% cpu  42.246 total
```

**自己只烧 0.10 秒 CPU(0.06 user + 0.04 sys),墙上 42.246 秒——42.1 秒
在等锁和 setup。** 比这仓原来记的那个样本
(`cargo test -p mandala-core --lib landing`,1m36s / 4% CPU)更极端。

**它直接决定上面那条「任何改动都跑 test」怎么读**:`pretest` 是
`npm run gen`,`gen` 是两条 `cargo run`——所以 `npm run test` **每一次都去排
那把锁的队**,而队伍有多长跟你改了多少行毫无关系。要只跑静态那几条又不排队,
就绕开 npm 直接 `node scripts/check-px.mjs`(这也正是本节那次事故之后
我复跑用的走法)。

**判词**:轻的是你的活,贵的是那把锁。**开跑之前先问一句机器上有没有别人在
编译**——你抢不到锁的时候,别人也抢不到。

### 字型那两道各守哪半

上面那张表说的是**有哪些、怎么跑**;这一节往下说的是**各自守什么、挡不住
什么**,而它只讲 `type` 和 `face` 这一对——这份文档管的是设计系统,别的几道
(px / icons / rail / palette / tabs)的判据写在各自那个脚本的头上。

- `npm run type`(`scripts/check-type.mjs`,静态):源码里 token 之外的
  字号/字重/圆角字面量、数字名字重类、tempo 双胞胎漂移 → 红。
  **挡不住**:规则写了但没落到屏上(层叠被盖、字体缺重、purge 误删)。
- `npm run face`(`scripts/check-face.mjs`,渲染):真页面上量——
  token 类逐个 computed 核值,注入几行数据后列表标题必须是 title 重,
  全脸 weight 分布必须含 {400,500,600},opt-in 了 `tabular-nums` 的
  元素必须真的拿到,**整张脸上每一个可点元素**的热区在窄宽两档姿势下都
  不低于 `--touch`(阈值从页面上读回,不在守门里抄第二份),**每一个按下去有
  反应的东西都在 DOM 里说了自己是目标**,以及**一屏够密时必须
  有阅读顺序**:主页那两半各自量,行数过两打的那一半里,占比最大的那个
  字号不许超过 85%、最大字号要比它大 1.25 倍以上。**挡不住**:没 opt-in
  的读数(它只验声明了的),以及注入样本与 SessionRow 形状漂移——那会以
  「行画不出」的红出现,是响的那种漂移。

  最后那条是 2026-08-27 补的,补它的理由值得记:**原来那条「全脸 weight
  分布必须含 {400,500,600}」是真的、也真的会红,但它守的是「这一页上某处
  有层级」,不是「每一屏都有层级」**。于是主页可以摆着 537 个文本节点、
  其中 535 个同一个字号,而守门全绿——用户的原话是「一堆字表达什么不清楚,
  看着费劲」,说的正是它刚刚放行的那一屏。这是这个仓的假绿家族里的一支:
  **断言成立,只是成立在别处。**

  补的时候又栽了一次同样的坑:第一版在整个主页上量,而另一半里一个
  17px 的机器名就把旁边那堵 30 行同号的墙盖过去了,照样绿。所以判据是
  **按半区量**,而且半区是**按结构找**(`> div > div`)不是按名字列——
  按名字列的守门只守得住有人记得写进去的那几块;结构变了它找不到半区,
  会以「探针死了」红出来,不会静静放行。

字型那两道都在出生时证明过会红(静态半:首跑抓到三处存量违规;渲染半:
把 title 调回 400,两条断言当场咬住)。改守门的人要重做这一步——
拆掉不算,要造一个它声称能抓的错(见 memory: 新守门怎么证明它会红)。

### 守门的作用域,必须和它的名字读起来的范围一样大

上面密度那一条记的是「断言成立,只是成立在别处」。**同一个形状 2026-08-27
当天又犯了一次,这一次犯在热区上**:那一节量的是 `[data-rail-item]`
——**六颗按钮**——而它在输出里、在文档里、在人脑子里都叫「热区守门」。
于是搜索框 18 高、每行的置顶 30×30、机器卡三个动词 24 高、日账七行 29 高,
全程绿灯。**断言是真的,它守的是导轨。**

两次犯的是同一个错,所以它是一族不是一次意外。**这一族的认法**:把守门的
名字和它的选择器并排读一遍——「每一屏都有层级」对上的是「这一页某处有
三档」,「整张脸的热区」对上的是「导轨的六颗」。**对不上的那一半,就是它
放行的那一片。**

改法有三条,都是这次撞出来的:

1. **按结构走遍,别按名字列。** 热区那一节现在走导轨能到的每一个落点
   (会话/机器/文件/浏览器/主页/设置,外加详情与机器卡),在两档姿势下量
   **每一个可点元素**;新加一个落点,它自己就进了名单——靠人记得加进
   名单的守门,只守得住加进去的那几个。
2. **例外要写下理由,不许静静过滤。** 热区那一节有四条例外(没排版的、
   `visibility:hidden` 的、被弹窗 `aria-hidden` 挡在后面的、
   `pointer-events:none` 的焦点哨兵),每条都在守门里写着「手指为什么落
   不到这儿」,而且每一站都把例外的条数打出来。**一个安静的过滤器,正是
   作用域缩回一条的走法**——输出里没有任何一句会告诉你有一类控件不再被
   判了。
3. **走不到要说「探针死了」,不许当成「这儿没问题」。** 每一站带一个锚点;
   锚点不在,守门红的是「这一站没画出来——是走法坏了,不是控件坏了」。
   这条当场就救了一次:机器那一站的锚点原本写的是星图,而窄档那一屏只有
   列表没有星图,守门立刻报了 `probe dead: narrow · devices + map never
   drew`,而不是安安静静地把上一屏又量了一遍。**「找不到」和「没有问题」
   长得一模一样,这是这个仓栽过好几次的坑**(`.nav-badge` 改名之后
   `querySelectorAll` 诚实地返回空数组)。

**它挡不住的**:
- **压在别人底下的目标**:够大、但被盖住,量出来照样合格;
- **裸 `<div onClick>`**——它确实不在这一道里,而它现在**在下一道**(见下
  一节)。**上一版这一行写的是「只打印不断言」**,那是一笔挂账,
  2026-08-27 结掉了。

**墓碑:上一版这里还有第三条,「设置弹窗的身子只判得了弹窗的框」。**
那条是真的,而且它按这份文档自己的规矩把洞写进了那一站的名字里——**写下来
仍然是走到门口就报「这家没问题」**。补一个 `face_choices` 替身之后,那一站
从 1 个可点元素涨到 19 个,当场红出五个 30×30 的色板井
(`views/FaceSettings.tsx` 那颗当时还是 `size-ctl-md` 的
`<input type=color>`,**已补到 44,见下面那条判据的最后一行**)。
**「把洞写在名字里」比数出零违规诚实,但它不是终点**:一个诚实的名字挡住的
是误读,挡不住的是那 18 个从来没被判过的控件。

补的时候第二次用上了「锚点选只有那一屏画得出来的东西」那条:锚点选的是
**最后一根轴里的一个选项**(`[data-face-axis="shape"] [data-face-option]`),
不是弹窗的框。把替身的 `shapes` 清空验过——身子照样画出两根轴和五个色板井,
**14 个控件照样量得出零违规**,而守门报的是
`probe dead: narrow · settings never drew`。**「画了一半」和「画好了」在
数字上分不出来,分得出的是锚点选在哪儿。**

### 例外和豁免不是一回事,合到一处就会烂成一张「这些都没问题」

那四条例外说的都是**手指落不到这儿**(没排版、`visibility:hidden`、被弹窗
挡住、`pointer-events:none`)。2026-08-27 傍晚来了第一条**性质相反**的:
分栏拖把 `data-list-seam` 补上 `tabindex` 之后进了热区判断,量出来 8×900
——**手指落得到,它就是不够大,而地板对它是错的判据**。

**这两种东西必须分开放。** 混在一张单子上,它会慢慢被读成一张「这些都没
问题」的名册,而下一个想加第五条例外的人翻开它时,那张单子**已经读起来像
是宽容的**了。所以守门里是两份:`excuse()`(落不到)和 `UNDER_FLOOR`
(落得到、豁免)。后者和上一节的普查同一个棘轮形状:记名、每跑一次逐条
打印、两个方向都会红——**名单外新来一个不够 44 的是红,名单里的哪天够了
44 也是红**(那时候删行)。

**豁免的理由必须是判据自己的条款,不能是「反正没人碰它」。** 领队最初给的
理由是「宽档独有、永远不是手指目标」,**这条被两件事实推翻**:拖把用的是
`onPointerDown` + `setPointerCapture`,**pointer 事件就是触摸事件**,今天真
拖得动;而守门那一节自己的注释写着它为什么要量宽档——「a desktop is where
a trackpad's imprecision and **the web face on an iPad** both live」,
**「宽档没有手指」正是这道守门当初明确否掉的假设**。

站得住的理由是另一条:**地板管的是尺寸可以自由长大的东西,而这一个的尺寸
不自由**——它是两个窗格之间的缝,每长一像素都是从列表右缘和详情左缘拿走
的,而那儿正是人真的在按的那些行。WCAG 2.5.5 自带的 Essential 例外说的就是
这个,不是在这儿现挖一个口子。

**而豁免必须把「它买不到什么」一起写进去。** 只写「地板对它是错的判据」,
这行字会被读成「这儿没问题」,下一个碰到这个控件的人就信了。所以名单里
每条都带一句:一块宽档触摸屏上手指今天真的够得着这条 8 像素的缝、也真的
难拖,**那是没人设计过解法的债,不是没有问题**;解法是给触摸另配抓手,
不是把缝加宽。

**顺带一条更普遍的**:`tabindex` 让这条缝**进了热区判断**,而它的触摸处境
一点没变——变的是守门看不看得见它。**`PRESSABLE` 是「这是不是个目标」的
代理,`[tabindex]` 又是那个代理里的代理**;一个 a11y 改进因此凭空造出一条
热区红,是代理和判据错位的正常后果,不是谁做错了。**别为了消掉这种红去改
选择器**——把 `[role="separator"]` 从 `PRESSABLE` 里摘掉是个安静的全局过滤器,
以后任何 separator 都白得,而输出里没有一句话会告诉你这儿有个 8 像素的东西。

### 能按的东西必须在 DOM 里说自己能按

**这不是热区,别挂在热区名下。** 热区问的是目标有多大,这一条问的是**除了
鼠标以外还有谁知道它是个目标**。一个裸 `<div onClick>` 只有指针够得着:
它拿不到焦点、键盘走不到、读屏不念——而且**上一节根本量不到它**,因为
`PRESSABLE` 没有任何办法看见它。上一节作用域的每一个洞都长在这一条底下,
所以它值一条断言,不是一句备注。

**判据读的是 handler 本身,不是 `cursor`。** 断言这一条读的是 React 挂在元素上的 props
(`__reactProps$*` 里的 onClick / onMouseDown / onPointerDown / onDoubleClick)
——**handler 是人写上去的,没有装饰性的办法得到一个**。`cursor` 当年只是个
猜「能不能按」的代理,每一站印一个「没有角色却是 `cursor: pointer`」的个数、
永远不断言(一行 hover 样式就能把它变成假红)。

**2026-09-28 起那个计数改了意思,并且断言。** 规矩变成「手指只属于正文里的
链接」(`30-behavior.md` 选字与手):控件一律是箭头,于是 `cursor` 跟「能不能按」
再没有关系,它自己就是一条规矩——每一站、两种姿势,链接(`a[href]`、
`[role=link]`、`data-md-link`、`data-link`)以外任何元素算出 `pointer` 都红,
只报每块手形区域的最上面那个元素,并报出写在哪个文件。落地那天旧代码上红了
77 处,一处一处删掉 `cursor-pointer` 之后绿。

**它是棘轮,不是白纸。** 落地当天跑下来全脸有三个,所以「一律为零」会当场
红、然后当天被人关掉。三个写进守门里的名单,每条带**它的代价**;名单两个
方向都会红:**名单外多一个是红**(有人新写了一个),**名单里的哪天不再出现
也是红**(债还了就删那行,或者走法坏了没走到它)。**一份不会过期的名单,
才是「写下来的例外」和「过滤器」的区别**——和上面那四条例外同一把尺。

名单落地时有三条,**同一天傍晚还掉两条,现在只剩一条**:

| 谁 | 为什么还是裸的 |
|---|---|
| `data-omnibox` | **转发器,不是目标**:handler 只在 `e.target === e.currentTarget`(条子之间那块空白)时触发,只做一件事——把焦点交给它身子里那个 `<input>`。目标在 DOM 里,就是那个 input;这一层只是把它的可按面积扩到字段的内边距上 |

还掉的那两条值得记,**因为棘轮的反方向是靠它们第一次跑起来的**:
`data-session-title`(双击改名是唯一路径)拿到了 `role="button"` + `tabindex`
+ F2/Enter;`data-list-seam`(键盘够不到分栏)拿到了 `tabindex` + 方向键梯子。
两条都不再是裸的,于是守门当场报了两句 `census stale: … carries no bare
press handler on any stop`——**那不是坏消息,那是「债还了,删行」的信号**,
而它是名单里那两行**唯一**会被删掉的原因。**注意它有歧义,而歧义是故意的**:
同一句话也可能是「走法坏了,没走到那一屏」。所以看到它先核一句那一站的
`says-so` 行还在不在,再删。

**它挡不住的**:用 `addEventListener` 挂上去的 handler(`ColorWell` 的
`change` 正是这么挂的)——那种读 DOM 的办法一律看不见。所以这一条配了活
探针:每一站先断言**「有角色的元素上确实找得到 handler」**,找不到就报探针
死了。React 哪天换掉那个 prop 键名,两个计数一起归零、每一站都报一张干净的
成绩单——那正是 `.nav-badge` 那个坑:**空结果读起来是「这东西不存在」,
意思却是「我找错地方了」**。

### 热区判据:扩的是热区,不是图形

判据本身在 `tokens.css` 的 `--touch` 注释里(数和依据都在那儿,这里不抄
第二份),这里只记**怎么穿**,因为这一批四个文件各穿了一遍:

- **在调用点上加 `min-h-touch` / `min-w-touch`,不要去改 `ui/button.tsx`
  的尺寸档。** `size="sm"`(24)是**长相**,是「贴着正文的行内控件」那一档
  (`--ctl-*`);热区是**另一根轴**,tokens.css 明写着它不是第四个 `--ctl-*`。
  改档会把全仓每一颗 `sm` 按钮一起撑高,落进别人正开着的文件里
  ——和 `cargo fmt` 是同一个陷阱。
- **机制**:`min-height` 压得过 `height`,所以调用点加得上;而 `cn()`
  拿不掉 `h-ctl-sm`(tailwind-merge 不认识 `ctl-sm` 是高度,
  `ui/button.tsx` 上有这道疤)。**从调用点只能把地板抬高,不能把盒子改小**
  ——正好是我们要的那个方向。
- **按下/选中的底色必须画在扩后的盒上**。所以这一批加的都是长在按钮自己
  身上的 `min-h-*`,没有一处是「图形小、热区大」的隐形盒子:手指找得到而
  眼睛没被告知过的目标,和热区不够是同一个毛病的两面。
- **横竖两根轴都要看**。「更早」量出来 41×27,**两边都不够**,所以它同时
  吃 `min-w-touch`;只盯高度会漏掉这一种。
- **别指望「热区探出图形」那条路**——它在滚动容器里根本不通:
  `overflow-x-auto` 的容器**两个轴都裁**,比容器高的子元素不会溢出到容器
  外面,只会被裁掉、或者给容器造出一条纵向滚动条(浏览外壳那条车道
  2026-08-27 撞出来的)。地址栏那条横向滚动的面包屑正是这种容器。所以
  上一条(底色画在扩后的盒上)不只是审美判断:**那条替代路本来就走不通**。
- **条形的行(会话行、日账行)是重灾区**:一列 29 高的行,按偏一点就落在
  邻居身上,而它不报错——它执行的是隔壁那一行的动作。
- **有一种控件的图形就是目标,那一种扩的就是图形**。色板井
  (`FaceSettings` 的 `ColorWell`)是今天唯一一个:它没有图标——**井就是
  那块颜色**,浏览器把颜色铺满整个元素,手指瞄的就是它。给它 30 的图形配
  44 的热区,恰恰是上面那条判据从另一面失败(眼睛被告知的是一个尺寸,手指
  摸到的是另一个),而且它连「按下的记号」都没处画:按下去就交给系统取色器了。
  **认法是问一句「这里有没有一个图标,和它的目标是两样东西」**——没有,
  就没有「扩热区不扩图形」这回事。理由写在 `ColorWell` 自己头上。

**它挡不住的**:一个**新写的**控件当然不会自动带上这两个类——这条判据
本身拦不住任何人忘记它,拦住忘记的是守门,不是这段话。所以这两节是一对:
**判据说该多大,守门说谁没做到**,少了任何一半,另一半都只是句口号。

### 概览用画的,细节用写的

同一份读数在这个仓里出现在三处:机器卡(环 + 字节数)、主页星图的每个
落点、主页「机器」那一列。**只有第一处该出数字。**

判据不是「哪个好看」,是**这一屏在被问什么**:总览问的是「谁在忙、谁不
对劲」,一条 bar 不用读就答了;「到底多少」是另一个问题,它有一个一按就
到的落点(机器卡),而在总览里出数字的代价是每台机器多三段等宽等灰的字。
量过的代价:星图上 16 个文本节点全是 12.5px/400,名字、年龄、三个读数
一起到,没有先后。

所以:**总览画 bar(`components/Gauge`),卡里写数**。bar 的 label 仍然
把百分比说出来(`aria-label`/`title`),所以读屏和指针都拿得到——去掉的是
「画出来」,不是「说得出」。

**它挡不住的**:一屏上全是 bar 也会变成另一种墙(六台机器十八条 bar),
那要靠「一个块一个 title」的老判据,不是这一条。

### 一档信息不值得说的时候,它就该不说

`ageIfOld`:读数年龄在还能用秒数的时候一律不画。**阈值不是新判断**,
是 `span()` 阶梯自己的第一级——星图上原来写着「0 秒前的读数」,那读起来
像坏了,而不像新鲜。凡是「这个值总是有,但大部分时候没有信息量」的字段
都适用这条:先问它**从什么时候起开始携带信息**,那个界通常已经在别的
地方被定义过了,不用现编一个。

## 三、加新档的路

先问三句,答不上就不加:
1. 现有档为什么装不下——「差 2px」不是理由,「这一档的语义它不是」才是;
2. 新档的语义名是什么——名字里带数字或带「大/小」的,还没想清语义;
3. 谁是第一个真实用户——没有落点的档是给未来的自己挖的坑。

答上了:值和理由写进 tokens.css(带「它挡不住什么」),tailwind v4 的
`--text-*`/`--font-weight-*` 命名空间自动生成 utility,字型那两道不用改
(它们读 token 不背数值)。


`npm run appearance` 的终端透明检查还覆盖程序 RGB/ANSI/反色背景、文字与 0% 选区可见性、1x/2x、实际上下文丢失的 DOM 备用、GPU 恢复及结束快照。它量每块区域一次的背景合成，不依赖整终端 opacity。

`npm run ime` 在系统 Chrome 中重放真实 TerminalPane 的输入事件链，并检查发往 `term_key` 的字节：未提交的拼音不发送、原生组字编辑保持生效、候选提交一次；覆盖软删除、Kitty、Option、迟到标点和收键盘取消。它不启动真实会话、不拿 cargo 锁；iPhone 原生输入法的事件来源与键盘行为仍需真机确认。

药丸手势的单独回归：`FACE_ONLY=bar FACE_BAND=1 npm run face`，运行 bar u 的真实触摸拖动、阈值、连续换页、取消、重渲染和中断收尾检查；完整甲板与键行仍运行 `FACE_ONLY=bar npm run face`。真 iPhone 验收包括键盘弹出时连续滑动、滑动中切后台再回来、系统中断后再次滑动。
