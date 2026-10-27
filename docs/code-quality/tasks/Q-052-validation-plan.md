# Q052 当前终端协议：验证 plan07（已准备，未启动）

Owner：node_test_review。2026-10-27。本卡及专用runner目录仍归作者；生产/测试源candidate02、root的实施/独审卡不动。不能自审实现。plan01及plan02原件完整保留，仅本卡转向当前plan07；plan02的dep-info ROOT substring是已确认setup误门，不执行。

**当前执行范围以末节plan07为准：仅剩9build/20父，06前端/CLI两批与3父不重跑。下方完整23父表保留总验收范围。**

## 当前冻结前提

- 树 `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-store-owner-recovery`，HEAD `13a9459f6c2dfd8d12912e6e2325b58de6e06999`，branch `fix/quality-terminal-current`。candidate02仍12变化路径，实际11个当前Rust源+删除旧ignored探针；本轮没有新增源码改动。
- root已独审candidate02唯一新增delta：host_shell_tmux helper增加 `-f /dev/null` 两arg；实际两个typed tmux启动原来已带该参数。父/oracle/所有生产源相对01不变。两份30文件一致、1921全输入再次逐SHA匹配当前。manifest `2688c8e5f2d1dbfc6a516791a51d3625d2029b96d912aebfb83faa7c30fe1e4c`；patch `74745a2101a5e520305608ef01134b1c94cb1c23a673e8e2a395217b653f2e96`。
- 本轮只有计划/runner准备与AST/SHA读取，没有Cargo/native/test/tmux/lsof/gen运行，不作平台/App验收或编译绿声明。必须root独审runner后另授唯一槽才能启动。

## 完整23父名单（07只续剩余20）

23 个不同普通父，9 个新定向父包含在内；没有 ignored worker 另计。12 种错误首帧输入仍只算所属一个 CLI 父。旧 ignored 手工旧 host 探针被替换，不算通过。

| 运行组 | 实际 target / 过滤 | 普通父数 | 边界 |
| --- | --- | ---: | --- |
| cli-current | `attach_version` | 2 | 实际 khor CLI→detached current host→真实 PTY；拒绝错误首帧后当前控制连接仍可用；Forget 真历史。 |
| core-current | `terminal_current` | 2 | 实际 term_open/history/rendererReady/Forget/key/resize API→自有 TCP peer；同 socket；不是 PTY 或 App。 |
| host-handshake | `khor_node` / `host::handshake_tests::` | 5 | 真实 TCP；current Version5 first、错误 Welcome/cookie、真实15s silence、150ms whole-frame四输入。 |
| core-io | `khor_gui_core` / `term::io_tests::` | 3 | 原3个 wire/order/queue oracle，仅fixture删version字段。 |
| gui-greeting | `gui_greeting` | 4 | 共享 Welcome helper 的原4 TCP父，含真实15s silence。 |
| snapshot-history | `host_snapshot` | 2 | 实际 host+sh PTY，model snapshot/history/modes 与原延迟门保留。 |
| replay | `host_replay` | 1 | 实际 host+sh PTY，ring trimmed 后模式仍正确。 |
| phone-ready | `open_from_phone` / `a_shell_opened_from_a_phone_is_on_the_phones_list_when_the_open_answers` | 1 | 两个私有 Node homes+hermetic relay+自 reexec host+PTY；Ready前不startup、Ready/reconnect只startup一次。custom harness只有此1父。 |
| private-tmux | `host_shell_tmux` | 2 | 真实PTY内私有 tmux -L khor-test-57/58-PID；私有 TMUX_TMPDIR；Drop只kill自己的-L。不接触用户 tmux。raw出现skipped即停，不计绿。 |
| cli-native-store | `open` / `a_private_host_does_not_inherit_an_unrelated_native_agent_store` | 1 | 只跑已机械修改的1个 CLI open父；实际 host/PTY，独立vendor home，不启动vendor App。 |

逐父精确名单（运行必须与原始 log 的名字和 passed/failed/ignored 汇总相符；custom phone harness核自己的实际1父输出）：

- **cli-current**：`the_real_host_rejects_missing_empty_old_future_and_wrong_first_ops`, `a_current_client_clears_the_host_history_for_later_attachers`。
- **core-current**：`core_rejects_noncurrent_welcomes_without_registering_an_attachment`, `core_sends_history_and_only_renderer_ready_then_forgets_and_continues_on_one_socket`。
- **host-handshake**：`host::handshake_tests::a_current_client_sends_version_five_before_any_input`, `host::handshake_tests::empty_old_future_unknown_and_gui_welcomes_are_protocol_errors`, `host::handshake_tests::a_cookie_refusal_keeps_the_hosts_reason`, `host::handshake_tests::a_host_that_never_welcomes_is_given_up_on`, `host::handshake_tests::the_first_frame_deadline_covers_silence_partial_prefix_partial_body_and_drips`。
- **core-io**：`term::io_tests::input_resize_and_clear_preserve_the_actual_host_protocol_order`, `term::io_tests::a_cancelled_queued_paste_writes_no_terminal_input`, `term::io_tests::queued_input_cannot_cross_a_closed_and_reopened_terminal_id`。
- **gui-greeting**：`current_gui_greeting_keeps_a_raw_stream_without_terminal_negotiation`, `empty_unknown_terminal_and_other_gui_greetings_are_rejected`, `a_refused_gui_handshake_preserves_the_hosts_reason`, `a_silent_gui_host_obeys_the_shared_handshake_deadline`。
- **snapshot-history**：`a_current_attacher_gets_the_snapshot_not_the_ring`, `a_re_attach_keeps_the_history_the_face_asked_for`。
- **replay**：`a_replay_trimmed_past_the_modes_still_starts_in_them`。
- **phone-ready**：`a_shell_opened_from_a_phone_is_on_the_phones_list_when_the_open_answers`。
- **private-tmux**：`a_tmux_typed_in_a_shell_is_not_handed_out_black_after_a_narrowing`, `a_shells_own_tmux_client_is_found_by_its_terminal`。
- **cli-native-store**：`a_private_host_does_not_inherit_an_unrelated_native_agent_store`。

`native_handoff` 仅编译当前已机械适配调用；它运行依赖显式真实 vendor conversation，默认跳过不能记绿，当前不授权运行。不包括用户 App、实际 Tauri→WebView 终端链、物理 iPhone/IME、Windows；这些仍需后续验收。


## 原完整运行集/分批策略（07仅取剩余）分批运行与回收

plan01先构建11个test exe再运行的峰值方案停用；不覆盖原证据。plan06串行11批，每批只构建1个必要test target，立即跑它适用父并核raw，再退役该exe。最多保留当前一个test exe以及CLI两批必须共享的normal CLI。

| 批次 | package / target | 运行父 | 运行后处理 |
| --- | --- | ---: | --- |
| build-cli-current | `khor-cli / attach_version` | 2 | 完成该组后精确退役test exe；normal CLI保留到下一CLI组完成 |
| build-cli-native-store | `khor-cli / open` | 1 | 完成该组后精确退役test exe；此后可退役normal CLI |
| build-core-current | `khor-gui-core / terminal_current` | 2 | 完成该组后精确退役test exe；不保留无用exe峰值 |
| build-core-io | `khor-gui-core / khor_gui_core` | 3 | 完成该组后精确退役test exe；不保留无用exe峰值 |
| build-host-handshake | `khor-node / khor_node` | 5 | 完成该组后精确退役test exe；不保留无用exe峰值 |
| build-gui-greeting | `khor-node / gui_greeting` | 4 | 完成该组后精确退役test exe；不保留无用exe峰值 |
| build-snapshot-history | `khor-node / host_snapshot` | 2 | 完成该组后精确退役test exe；不保留无用exe峰值 |
| build-replay | `khor-node / host_replay` | 1 | 完成该组后精确退役test exe；不保留无用exe峰值 |
| build-phone-ready | `khor-node / open_from_phone` | 1 | 完成该组后精确退役test exe；不保留无用exe峰值 |
| build-private-tmux | `khor-node / host_shell_tmux` | 2 | 完成该组后精确退役test exe；不保留无用exe峰值 |
| build-native-handoff | `khor-node / native_handoff` | 0 | 仅编译，保存来源后精确退役；不运行 |

每批构建实际命令：`cargo test --locked --offline -j 1 -p PACKAGE (--lib | --test TARGET) --no-run --message-format=json-render-diagnostics`，运行直接用该JSON产物实际路径；不nested Cargo，不另做重复normal check。首CLI批必须实际重编当前normal Node/commands/CLI；首core批必须实际重编当前normal gui-core。其余已有正常库可沿用本run同1921源来源，不把fresh=true假冒跨树绿。

normal CLI由两个CLI integration的静态 `CARGO_BIN_EXE_khor`引用；第一组后保留，第二组build/run及其owned host全退后才允许回收。phone fixture只reexec自身actual test binary，必须等父/host/PTY全部退后才回收。core若Cargo顺带重编本树bridge/gen_bindings，只记录fresh=false的本run实际exe并在本批之后精确回收，不执行它们、不当gen验收，也不碰旧fresh缓存或任何rlib。

回收仅限本run Cargo JSON成功生成、已记录且不再为后续父使用的实际可重建exe；完全不扫target前缀或旧缓存。每次核完整source仍匹配、所有本run组和观测到的owner已退、目标regular/no symlink、同uid/nlink1、SHA/size/mtime/dev/inode均为原产物；精确lsof必须exit1且stdout/stderr空。将原log、compiler JSON/depinfo、source/binary身份及拟回收receipt先双盘逐SHA复制，再复核目标inode+SHA并仅unlink此一exe。保retired ledger及删除后absent证明；后续binary稳定检查区分active hash不变和retired按账absent，不把正常退役写成覆盖不稳。

失败父/失败构建不进入retire；保原raw和实际失败exe缓存供root诊断，必要完整内盘copy由root在进一步回收前核准处理。成功可重建exe无需把11份约GB级binary都完整复制；双盘保存完整SHA/size/mtime/uid/inode/来源原log。lsof警告/非零异常/活owner/未知目标一律停止，不扩大删除。

## 当前源码与artifact来源证明

- 启动先核HEAD/branch、candidate02外内30文件、1921全输入、预设manifest/patch SHA；source前/每phase/后全等。
- 删除plan01全第一方Rust mtime遍历。只在对应必要build之前刷新数据中列出的byte-stable target source；前三个当前normal依赖只各刷新一次：Node host.rs、commands lib.rs、core term.rs。两个CLI/相关core/Node integration只触该test文件；Node libtest沿最初host刷新，不再次触生产源。core unit仅额外刷新term_io_tests.rs。每批记录明确名字，刷新前后SHA必须同1921冻结源，不触manifest/依赖/lock或无关crate。
- required test artifact必须fresh=false、manifest_path与target.src_path精确本树；保存由actual exe推导的实际dep-info `.d`；按make规则只读冒号右侧源依赖（左侧output/phony不计），解析反斜杠空格/冒号/hash/backslash、$$、续行及注释；相对源按实际Cargo cwd ROOT归一化，要求目标source_path精确在该源集合且SHA=frozen。不要求depfile文本包含绝对ROOT。normal首批artifact同样精确本树manifest/source路径与fresh=false。所有原始Cargo JSON保留，当前代码frame与实际路径共同证明来源。
- 这里的mtime只为唯一槽获准后的当前源重编；本准备没有执行touch。不会利用跨树stale产物的文件名相同当当前来源。

## 环境、guard与边界

- Cargo和**全部runtime父**均保留调用环境原HOME；不拿HOME作scratch变量，不覆CARGO_HOME/RUSTUP_HOME。fixture根按原显式root/KHOR_HOME机制落短自有TMP。`TMUX_TMPDIR`私有、移除继承TMUX/session；candidate02全helper/typed tmux带-f /dev/null，不读取用户tmux配置、不访问用户server。
- STEP300/TOTAL900/MIN5368709120，实际AST及引用核验，jobs1/offline/locked/incremental0，共享target `/Volumes/disk0/Khor/caches/rust/khor-quality-target`；独占scratch `/Volumes/disk0/Khor/caches/tmp/t52v7`。每250ms动态内外空间/时限；遇guard只停自有组，有限TERM/KILL/wait/drain；forced drain不计绿。预算不因分批而放宽，23父未减少。
- 启动再核无cargo/rustc/gen_bindings在跑；无槽不启动。首个编译/fixture/业务/资源异常分类保raw即停，不自动修源/重跑/清旧cache。App/物理iPhone/Windows未知保持。
- fixture负责实际detached host/private tmux回收，runner采样本PGID、后代、私有root命令/精确本父tmux tag的PID+start身份，逐phase和最终核退；仅杀自身PGID。活detached owner或未知socket/symlink保scratch停报，不以父exit0伪称资源已齐。完整列举仅regular/dir且无owner才清精确scratch。
- 双盘receipt在退休前与最终均核等；正常cache退役单独账，不改变历史source/binary证据。全部23父源码/断言不改，原unknown/list/ignored计数语义不放宽。

## 已准备工件

外盘 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q052/terminal-validation-plan07`；内盘 `/Users/zgh/Library/Caches/khor-code-quality/recovery/Q052/terminal-validation-plan07`，17文件逐SHA。

- runner SHA `7e78e1a16273db4da6ea4b0be0d9db9a62fd568cdac96db312e61f9f16617a9e`。
- data SHA `d81e16663f60bed57e3b7773e566357f158e2ddaf7be541a3564a352c57402d4`。
- preparation-audit：仅ast.parse，300/900/5GiB实际赋值、23唯一父与plan01同名、无HOME赋值/无全源刷新，1921当前=candidate02；未import/执行runner。
- 后续执行预留 `Q052/terminal-green07`；启动参数 `--execute-authorized` 只是防误触，不能替代root独审及唯一槽授权。当前无执行回执。

## plan03窄修复核

plan02 runner原SHA `d106c7a8c9646877f30bbaeb8684b718c864263d7de58524bcfed70ac2d9f24e` 保留不覆盖。plan03只增加dep-info源解析和替换该setup门；11批/23父/精确退休/HOME/300-900-5GiB门不变。`delta-from-plan02.patch` SHA `8b7d77f7feb9b66b633177aae5e8f13a26b7a9f7e8a70cc6b38181f991f005eb`。纯解析helper的六个转义/相对路径/continuation/phony/comment用例已执行，另只读现有Node `.d`确认workspace-relative来源可解析；它不证明当前共享binary来自Q052，仍须未来实际Cargo JSON strict current+fresh=false。没有import runner/执行Cargo、原生或测试binary。

## plan04已观察detached组成员门

plan03原件完整保留（runner `913ed80d63c7c12e0089af474eb6d356ebb7ecabb83c519df751e0f547a94ee6`）。其顶层组已用killpg0检查全组，但detached owner仅旧PID/start可能漏leader退后新生generic child；plan04仅owned_live增加已观察group leader的PGID→当前全成员门。任何成员仍活、或当前leader start改变而身份不明，均保留并报非绿，阻止后续父/精确退休/TMP清理。未新增kill权限，仍只自动停本轮直接Popen PGID；不会凭历史PID盲杀其它进程。精确source1921/23父/11批/退休与HOME及dep-info门未动。仅纯字典矩阵验证leader退generic活/无关组/leader活/全退/身份变化5case，无ps、kill、Cargo或真实测试运行。新delta SHA `eaecdea7448ef21f70ee0671cd42ab4f7014ca2f2e676ff367adafb3aa8a43e9`。

## plan05 catalog实际当前producer门

plan04原件保持（runner `845aed18287a32bd818078fa8d7c1b9450a8905cc478e3a79af38e21a29da5b0`）。依赖比较小表/JSON已存506个精确输入，42个sync全同；排已逐批当前重编的Node/CLI/commands/GUI根包后，只catalog en/zh TOML不同，共同文本值全同。Q052-only为msg.host-gui-protocol，N1-only为两个msg.sync-partly-pending键，必须查完整集合。

首CLI build前只额外mtime-refresh这两个TOML（byte SHA不变），不触build.rs或全依赖。首CLI normal artifacts新增要求current khor-catalog manifest/src_path且fresh=false；按该package_id精确关联build-script-executed，保存actual out_dir、catalog.rs全文/SHA/size、当前TOML/build.rs/Cargo/lib源SHA=frozen、buildscript实际artifact/文件SHA。buildscript binary可沿用同源build.rs，但这次必须重新运行源生成且normal catalog fresh=false。全部十section生成函数集合（含5个word helper）与msg/cli TEMPLATES canonical keys精确等于Q052 TOML，既不可含N1 Pending也不可缺host-gui-protocol。

准备仅执行隔离纯parser读取既有20份generated Rust，正确接受一份既有Q052 keys、拒绝真实N1旧集合对Q052使用，同时N1自身集合正对照通过；不import runner，不执行build.rs/Cargo/gen，不把任何旧binary宣称本次当前产物。此门还需root独审和实际构建raw。source1921、11批23父、HOME、300/900/5GiB和精确退休/PGID门不变。GUI资产只保既有hash，不声称同源App。新delta SHA `20a18a9b6fdad50d9715aefc0d9bacd3d1b43d3679d36fa395f07d12d4ce378e`。

## plan05实际首次执行：前提失败，0/23父，槽已归还

root明确授槽后实际执行原冻结runner（SHA0dc55…85a4b），没有改源/放宽门。`Q052/terminal-green05`外内23文件逐SHA一致；runner session55941，Cargo PID/PGID97656。首build-cli-current实际exit101，52.12s；总52.991s。guard=null，最低外14,891,540,480B/内11,677,245,440B，无磁盘/时限/IO触线。

真实失败是khor-web build.rs:50要求本树apps/gui/dist/index.html但文件缺失；属于构建前提/setup失败，不是Rust类型错误或终端业务红。未形成actual CLI/test executable，23父全部未运行，后10批未开始，未回收任何exe。final.json里空binary集合的stable=true不能当actual binary证据。原raw/error/final不回填、不覆盖。

部分正常库actual JSON fresh=false包含catalog/sync/net/Node/commands；只读事后另存partial-current-artifacts和partial-catalog-producer-audit，明确标whole_CLI_build_failed。actual catalog out_dir来自本次build-script-executed；generated catalog.rs SHA81236c364d71dfdd03e95df4dbcecbf813660570d9c8262bc7fce317cc377903，完整10section函数及wire模板keys匹配Q052（含host-gui-protocol，不含2Pending），TOML/build.rs等源SHA=frozen。这不是整个CLI/test通过，更不是GUI/App验收。

1921 source before=after=current candidate02；原PGID后验ESRCH、全部owned_live为空、精确scratch只有空tmp/tmux目录且已清零移除；无cargo/rustc/gen残留。源、raw、partial证据、cleanup、双盘独立记录齐。未自行补dist/copy旧资产/npm/gen、未修源或重跑；唯一槽正式归还root。全Q052仍open。root随后要求仅准备plan06真实本树前端build前提，另审后再授槽。

## plan06本树真实frontend前提（未运行）

plan05失败raw/final/23双盘及旧runner原件全部保持；新06只加资产构建前提与actual web embed来源门，11次必要Cargo build/23Rust父不变。先执行本树命令 `npm --offline --ignore-scripts --script-shell=/bin/sh --prefix apps/gui run build`，package build本体严格固定为 `tsc --noEmit && vite build`。本机npm11.17源码run.js123–140确认ignore-scripts仍执行指定main event，但不追加pre/post；不运行prebuild的Cargo gen、不执行npm install/ci，不copy其它树或placeholder dist。Vite配置只react/tailwind/woff2转换，build不启动devserver；现dist不存在，若启动前未知dist出现停报，不先删。

只读核node_modules可用（沿现shared symlink）、TypeScript5.9.3/Vite7.3.6与本树lock匹配、npm/node/实际脚本SHA。571个本树src/public/config/package输入含155个gen TS已冻结，build前后与最终均核源/类型文件未变；不运行新bindings producer、不称同源App验收。npm cache仅指自有scratch内，HOME不覆。build输出dist是新outputs，另记完整文件SHA/size、actual HTML和script/modulepreload JS引用；必须有真实/assets/index-*.js并存在本树dist。dist在此所有者树保留供khor-web，不混入1921input漂移，也不删除或部署App。

首CLI构建还要求本树normal khor-web fresh=false，按精确package_id关联真实build-script-executed/out_dir，保存assets.rs全文/SHA；UI_REVISION必须等于khor-ui-assets:+实际index JS path，include_bytes的web路径集合必须覆盖精确dist树，每条disk路径归一化后都在当前树dist并逐SHA相等，不可用别树旧assets.rs。catalog05门保持。Rust raw/actualbinary/retired账与frontend前提分开，frontend/tsc失败只算setup，不把它当23父红/绿。

AST仍300/900/5GiB，所有原PGID/退休/双盘门不变。本轮仅准备及隔离纯HTML/Rust include解析表达式检查，没有npm/Cargo/gen/浏览器/服务运行，没有写dist或改1921源码。新delta SHA `de881032883c8d04ea92c83b662b586c2d2ea8a295bbcff39179d6213aabd31b`；frontend-prerequisites.json包含当前工具/输入精确来源，须root另审授槽。

## plan06实际执行：3/23父通过，normal CLI身份退休门停（整run非绿）

原9ab99…05cd runner实际按授权运行；`Q052/terminal-green06`外内49文件逐SHA齐。前提npm exit0/12.59s、首CLI build exit0/53.59s、cli-current两父exit0/20.24s、CLI open build exit0/1.75s、cli-native-store一父exit0/1.22s。三不同普通父确切passed/0failed/0ignored，原raw和父名单保留；没有业务target失败。还有9必要Cargo批（含native compile-only）及20父未运行，不把build数和运行组数混算。

随后原retire门尝试normal CLI时身份assert失败，未进入其lsof或unlink。旧inode36029806，当前36030017；device16777244、bytes257289528、mtime1793062898514974999、uid501/nlink1及SHA2e95a41ed2c56afd7a40c4dfbabe579cf803979ef0ce1898a39f892265580946全部相同，只有inode变。第二CLI build的原JSON再次给同本树normal khor artifact/fresh=true；可能与Cargo重新发布bin相容，但没有FS trace，暂不硬断原因。所有权门严格停止是正确行为；需要root审新控制如何在每次已知Cargo发布后登记新的身份，不能静默放宽或回填旧snapshot。此normalCLI缓存保持未删，事后独立lsof精确exit1/stdoutstderr空；两个已完test exe依原门已退休并有原双盘pre-unlink receipt。原binaries.json、error/final/retired账均未改写。

本次真实dist277文件，HTML/index JS为/assets/index-CRE8tIZZ.js；normal khor-web actual fresh=false且actual out_dir/assets.rs include路径与本树dist全集合逐SHA匹配，UI_REVISION=khor-ui-assets:/assets/index-CRE8tIZZ.js。catalog实际当前producer/key门通过。571 frontend输入/155genTS及完整1921 source before/after/current均稳，dist最终SHA也同producer；这是Rust CLI build资产前提，不是新bindings或App行为验收。

总92.761s，guard=null，最低外14,856,286,208B/内11,623,297,024B。五组PGID6661/6884/7732/8096/8133均后验ESRCH，所有owned_live为空，无cargo/rustc/gen残留；精确自有TMP已清。没有源修订、宽门、补跑或额外生成。整体仍failed/stopped；author-poststop-audit另存事实，不回填原failure。唯一槽正式归还root，后续只按新的独审/授权行动。plan06源及15文件冻结原件保留。

## plan07仅续剩余9build/20父（未运行）

06已有3个不同CLI父绿及两个实际CLI构建不重跑；不再npm/前端构建。new07 imports immutable green06全部49文件SHA证据（外内相同），源before/after/current1921、前端571/155gen、277实际dist/UI_REVISION与catalog/web06真实生成producer SHA全部核同后才能执行；已有CLI来自06真实fresh=false normal编译内容，不以第二次Cargo fresh=true单独当同源依据。

初始normalCLI要求第一/第二actual Cargo row唯一且main/src_path、manifest、executable精确本树，fresh分别false/true；当前SHA/size/mtime/dev/uid/nlink等于first，inode精确等于06poststop已观察36030017，另记newidentity与原因未知，不回写06。新ledger明确使用当前inode而非复制旧inode。此CLI保留至全部剩余Node/host/native编译完成才按当前身份/lsof0/所有owner退+原SHA/source证明精确退休，不删陌生或正在使用的产物。

每次后续Cargo后如再次声明同normal CLI路径，只有其actual row严格当前root main/manifest/executable且原内容/size/mtime/dev/uid/nlink全等、1921source稳定时才允许记录新inode身份，保原origin和publication ledger；没有该row而inode变化即停。内容SHA门始终不变；不会把未知inode变化称数据损坏或凭freshtrue放行。

剩余批次/父严格为06列表去除前两CLI批与三已绿父：core-current2、core-io3、host-handshake5、GUIgreeting4、snapshot/history2、replay1、phoneReady1、tmux2=20；native_handoff仅第9批编译不运行。另17文件dual冻结保原06/05失败原件，AST300/900/5GiB、HOME、privateTMP、exact exe退休、PGID全成员门不变。启动参数仍需root独审+唯一槽；本轮仅准备和receipt算术，不import/执行runner，不触发lsof/Cargo/npm/测试。新delta SHA `b666b5001a4399593303e1d03a49f9b04be2903cef0d1ae91c92d01465fc64ab`。

## plan07实际续跑：core清除后内容断言失败，停报并归槽

root按07冻结runner授予唯一槽后执行原`--execute-authorized`，未重跑06三绿/前端/CLI两build。`Q052/terminal-green07`外内37文件逐SHA相同；runner SHA7e78e1a16273db4da6ea4b0be0d9db9a62fd568cdac96db312e61f9f16617a9e保持。首build-core-current真实exit0/19.72s，actual current terminal_current executable fresh=false，manifest/src/depinfo源SHA精确本树。首core-current父组真实exit101/1.25s：拒绝非current Welcome父通过；`core_sends_history_and_only_renderer_ready_then_forgets_and_continues_on_one_socket`在terminal_current.rs:201断言清除后的after不能含before-clear失败，线程随后读帧EOF。原raw/error/final保留。当前只定实际业务oracle断言失败，产品与fixture具体因果仍待归因，不把EOF或任意Err当目标红。

本次2不同普通父=1通过/1失败；合并06既有3绿为5/23已运行、4通过/1失败，另18未运行；9剩余构建中只完成首批，后8批和native_handoff compile-only未开始。没有自行放宽、改源或补跑。failed test exe、incidental bridge/gen_bindings和继承normalCLI全保留；本run零exe退休。gen_bindings仅Cargo incidental构建，未执行生成器。

总23.517s，无guard/forced drain，最低外14,576,373,760B/内11,622,977,536B。PGID19309/19603后验ESRCH，owned_live=[]，精确TMP仅空tmp/tmux目录并已移除；无cargo/rustc/gen残留。完整1921 source before=after=current，571前端输入含155genTS与actual06的277dist/UI_REVISION全部稳定。4个实际记录binary当前SHA/size/mtime/dev/inode/uid/nlink逐项同记录、精确lsof均exit1空输出。后验事实另存author-poststop-audit，非独立审查；原control/日志/失败收据不回填。唯一Cargo/native槽正式归还root；全Q052仍open，下一行为须另授权。

## plan08 source03完整当前验证准备（未启动）

Root独立审已接收source03（terminal-candidate03-root-review）。旧05/06/07失败和source02历史绿保持；Host helper已变，08不继承任何旧行为绿。仅准备新08：11个必要单target no-run构建，28不同普通父=原23+新IO4+实际Host Model clear旧父1。真实源码确认selector为`host::last_line_tests::the_model_forgets_what_was_cleared`，Node lib同exe先握手5再`--exact`该1父，全部成功后才退休。IO `term::io_tests::`实际7父，原TCP两父完整保留；native_handoff只compile，不运行/不计父。无Cargo/native/npm/gen启动，等待root另审和唯一槽授权。

原06不可变49双盘来源只为571frontend/155gen/actual277dist/UI_REVISION字节与历史producer证明；08复制旧catalog/web收据均加origin06前缀，不将其当本次current producer，不导入旧CLI。当前source03 normal CLI/Node/commands/Core/Sync/catalog/web要求actual fresh=false、精确当前manifest/src；首CLI批只额外refresh两catalog TOML、sync lib、web build.rs、host.rs、commands lib、CLI main与目标test，其余批只必要目标源。N1最新actual control04/libtest-build source frame1084已实比506依赖输入：Sync42及其他462同，差异仅2 catalog TOML。旧main graph与之前candidate对比均不冒当前事实。

每个关键normal artifact另存真实dep-info（相对Make RHS仍依ROOT解析）、critical current source路径与frozen SHA、所有in-tree tracked依赖SHA及actual文件全SHA/size/mtime；最终normal库hash门。catalog实际out_dir/catalog.rs完整十section函数+wire模板key集必须精确本树TOML，不能接纳N1 Pending superset；web本次current producer assets.rs/UI_REVISION/include全部路径+SHA必须精确本树dist。CLI当前内容baseline来自08真正fresh=false编译；之后沿07实际Cargo出版row+同内容/size/mtime/dev/uid/nlink/source才登记inode变化并留ledger，不使用旧06CLI行为。CLI保留到全部Node/host/native结束；正常精确退休的CLI用退休前完整SHA证据+预期absence核，不能当normal库漂移。

HOME、私有TMP、bounded PGID全成员/finite drain、guard非绿、source/binary/cleanup门与双盘pre-unlink账保持；STEP300/TOTAL900/MIN5368709120 AST。每批成功才精确退休本run实际产物，失败exe保留，不清旧cache/前缀/用户目录。当前source03完整1921、candidate27双盘、实际571/155/277与origin49核齐，prep仅isolated原depinfo/catalog纯parser(AST等于已审07)检查，实际06catalog正例/actualN1对Q052反例均符合，0个业务运行。

plan08冻结runner SHA `763c3f17df944348d7b7b97a701d6a6d5e8222054b0f5d09b88e864c2942caee`；data SHA `3dc1072aade47d823739d8e9ca312d088b3c7bb0992f7749d90241e66183e2f4`。新目录terminal-validation-plan08保历史07原件，含source03 currentframe、依赖实比/actualN1 receipts SHA、origin49、AST父名单/各纯parser历史及新准备审计。准备不代表已获执行槽；本卡仍作者持运行收据写权。

## plan08实际：首构建成功，normal CLI顶层dep-info控制误门停（0/28）

root明确授槽后执行原763c…caee runner；首build-cli-current Cargo PID/PGID45208，实际exit0/62.31s，原JSON build-finished success=true。随后新增normal dependency来源门取`debug/khor.d`，其RHS没有本树crates/cli/src/main.rs而assert失败；整体非绿，28父全部未运行，余10build/native compile未开始，零exe退休。不是编译错误或产品业务红，原control/原error/final一律保留，不自行放宽、补跑或改源。

只读后验确认：actual normal CLI row唯一、fresh=false、manifest/src精确本树；Cargo test生成本次normalbin但没有更新旧顶层Cargo汇总debug/khor.d，该file仍指旧R1树，mtime1793044463925302374。实际本次rustc hashed binary/deps为`debug/deps/khor-31b03f1ae134de07`及同名.d（mtime1793066826690138291），RHS相对ROOT是当前crates/cli/src/main.rs；normal发布exe与该hashed exe全SHA/size相同，SHAe0c36b62fe8303648b2098ff70e57b9b8ccd40109776b1e462421ff78af7b97d。仅后验记录，不回填成原phase gate通过。两种原depfile另存author-stale-top-cli.d与author-actual-rustc-cli.d；正常exe和failed-run test exe均保留，没有扩大清理。

原normal-dependencies已记录catalog/Sync/Node/commands，CLI门失败发生在其producer最终门之前。事后只读另存actual current catalog/web生成原件并核完整catalog函数/template keys精确Q052（含host-gui-protocol、不含N1 Pending）；actual assets.rs UI_REVISION和全部277 include路径/SHA精确本树06dist。此为author-partial生产来源检查，非独立审查或28父业务绿。后续若修控制，应按actual normalbin对应的hashed exe/depfile精确配对来源，不直接信旧top-level .d或只删当前main断言。

总63.571s，guard=null/forced_drain=false；最低外14,493,638,656B/内11,271,458,816B。PGID45208后验ESRCH、owned_live空，无cargo/rustc/gen残留，私有scratch已清。1921 source before=after=current source03；571前端含155gen、277actual06dist/UI稳定，记录实际test exe SHA/size/mtime稳。normal CLI/hashed pair/实际test exe精确lsof全exit1空。`Q052/terminal-green08`外内44文件逐SHA一致，author-poststop-audit SHA7233181c2eb9671a35cd1956bf15dcd18cc0efe1f3150b133e0b4ce328e54536。唯一Cargo/native槽正式归还root；source03和原08计划继续冻结，下一行动另授。

## plan09最小resume准备（未启动，保持08控制失败原件）

root确认08是normal CLI top-level Cargo .d误门，授权仅准备09；source03/1921实施源不改。09第一个阶段明确`reuse08-build-cli-current`：继承08唯一实际exit0构建，其44文件外内SHA、1921before/after/current、原CLI e0c36…7b97d与hashed pair/test exe及四normal库原账/生成源全部当下核同。原PGID45208只留历史producer receipt，不进新owned/result PID账，也不伪新编译或重复第一Cargo。之后实际运行所有28父，另10必要构建；原旧4pass/08零父一律不继承为当前业务绿。

新增normal CLI配对门同时用于初次继承08及后续实际Cargo publication：仅在debug/deps/khor-<16hex>普通非symlink exe中找与actual published CLI全SHA/size/mtime严格相同的唯一文件；其配对rustc.d LHS必须同时声明该exe与该.d，每条RHS归当前ROOT并逐SHA匹配完整frozen source，包含actual row target main，原manifest/src/executable严格门保留。不取debug/khor.d，不靠freshmetadata或仅搜RHS。全部library .d规则、content/size/mtime/dev/uid/nlink同源门、可核实际row后的inode ledger和精确退休门保持。normalCLI/rustc companion保到其依赖父结束，不回填08、更不往sharedtarget拷旧内容。

正常生成来源在09实际重验，明确producer execution run08、actually_executed_this_run=false：catalog fullkeys/outdir和web assets.rs/UI/277 include全部当前。原四normal库全SHA/mtime/bytes同08原账；web normal archive全SHA首次snapshot标09 preparation，以08 actual freshfalse current row、mtime在原owned62s build窗口、1921来源稳定和root协调无其他native writer为依据，绝不称08原control已存过其archive SHA。若任一snapshot/producer变，停报重新计划。

准备期仅isolated read-only控制检查：实际08缓存配对/LHS/RHS成功；同size/mtime错内容、同内容错mtime、重复匹配、正确RHS但错误/缺LHS、外树RHS均拒绝；Make escaped LHS解析对照通过，0业务父。整个import06资产→reuse08入口也以Popen/native-run强制拒绝的隔离namespace只读执行，允许的唯一外部命令是git ls-files；六normal根与actual test/完整生成源检查齐，未spawn/退休/所有权导入。初次隔离fixture漏资产import而停，补回与真实main相同调用序后通过，记录在check内；没有改runtime控制规避失败。

source03 candidate27、08原44、current1921、571/155/277稳定，out/scratch原absent，当前无compiler。STEP300/TOTAL900/MIN5368709120 AST、10新Cargo+1历史build、11runtimegroups/28唯一父（IO7/Node握手5+Host exact1）、native_handoff compileonly齐。HOME/TMP/PGID全成员/有限drain/双盘/失败原件/precise retirement全部保持。原05/06/07/08和旧计划不动，未取得启动许可。

09 runner SHA `667f47a4716d539e2863e1b6808575be7b71afd555c5222e973832fd1f854d94`；data SHA `2cace764e8d1dad9d946fbe84b926fd321c992ce2417f4fa3dc1ea3bcd1c7f65`；reuse08 snapshot SHA `8cf46e19439eedef60904911e53d194c5b8a2a77fece28ba8c979410656a4c70`。新双盘terminal-validation-plan09冻结供root短审；本卡继续保运行收据写权。

## plan09实际：28/28业务父通过，最终保留两stale socket而整体cleanup非绿

root明确授槽后执行原667f…4d94 runner，先严格复核08原44双盘/1921来源及24实际artifact当前身份，诚实写reuse08-build receipt，无新第一Cargo/无旧行为绿继承、旧45208不进owned。随后21个真实phase=10新必要Cargo+11runtimegroups均exit0、guard=null、forced_drain=false、所有phase owned_live空。28不同普通父全部实际通过，包括原07第201行失败TCP父完整body、新IO4全部、原IO3、Node握手5+sameexe exact Model clear1、GUIgreeting4、snapshot/history2、replay1、真实phoneReady1、私有tmux2、真实CLI3。native_handoff仅编译成功1.53s，不计业务父；App/物理设备未验。

整体仍exit1，原final error=null但cleanup_completed=false、scratch_absent=false，不能称整run绿。总224.167s，最低外14,580,387,840B/内11,196,653,568B。原runner正确拒清自有TMP的特殊条目：`/Volumes/disk0/Khor/caches/tmp/t52v9/tmux/tmux-501/khor-test-57-61015`与`khor-test-58-61015`两UNIX socket，mode0140660、uid501/gid20/nlink1，device16777244，inode分别36033715/36033684。其它5条目均目录，无regular/unknown/symlink。该61015为最后私有tmux父实际process group，原test server名严格带该PID；没有扫描/清其它用户socket。

原09runner产物149文件，补author-poststop-audit和author-socket-state-audit后151文件外内逐SHA齐。21实际PGID全部后验ESRCH，所有原observed PID/start均不在，owned_live=[]，无cargo/rustc/gen。两socket及其目录精确lsof均exit1/stdoutstderr空；root另授权只读AF_UNIX connect，两者各实际errno61=ECONNREFUSED，前后inode/mode/uid/mtime不变，可证stale，仍原样保留未unlink。author-poststop-audit SHA32d21bba778e240cc9132295f561612a06f7ed0c112214e13d6e039ffa635ad4；socket audit SHA46b2e00e0d0d94c33c85091f3e94925569d5f08fe8c5bbe7b4e48feae3740f51。原final/cleanupfailure不回填。

完整source03 current1921=before=after；571frontend含155gen及actual06dist277/UI稳定。7个关键normal根的库/伴随CLI内容证明稳定；按原精确门已退休14个可重建exe（11test含08继承且本次真实执行的首test、2本run incidental、最后normalCLI），每个完整身份/全SHA/lsof0/所有owner退、双盘pre-unlink原账均齐。normalCLI hashed companion仍保且全SHA相同，最终normal库检查通过。没有新gen/前端构建/改源/扩大cleanup。

只读定位fixture：`host_shell_tmux.rs:44–50`的Server Drop仅`tmux(-L self.0, kill-server)`，未处理自身socket；两父末尾也各kill-server，然后Drop重复kill，无unlink路径。故这里是fixture退出清理缺口，非产品行为红。建议后续最窄修仅该fixture：固定当前-L私有name与实际socket路径/归属，在有限server退出后，严格只对该精确ordinary UNIX socket且UID/identity一致、connect=ECONNREFUSED、无owner才unlink；活/未知/symlink保留并让explicit success-path cleanup失败可见，Drop只守panic收尾。两父业务oracle不动。若root另授source04与专门控制，可继承未改26父并只重跑原tmux2证明TMP干净，不需全28重复。

本次唯一Cargo/native槽正式归还root。source03、原09计划/原05–09失败与业务收据继续冻结，无source04/后续执行自授。Q052整体收口待root独立证据审/精确清理决定与fixture修订。


## source04 / plan10 实际收口与集成（root，2026-10-27）

source04仅host_shell_tmux.rs fixture变动，22文件双盘冻结；root实际读delta并核生产及其余1920输入相对03完全相同。私有PID标签、初始路径不存在、server回报socket与身份、父目录归属/类型、-N/-S精确控制、有限退出及不活跃探测；只清自身原身份socket，未知/活跃保留。旧业务oracle不改。

plan10 runner `dc92fb250c3cc7caa405e4ed893db1fbfa2125a8398a815a2521f524c50342da`，300/420s、5GiB。绑定实际R1 A8成功normal producer后，一次必要host_shell_tmux构建，精确刷新5输入，当前Node/Sync/catalog fresh=false、实际depinfo/fullkeys等通过；不重跑旧26、不npm/gen/CLI build。实际build PID89120 exit0/74.5767s；runtime PID90059 exit0/6.4232s、两原父2/0/0，无skip。final 83.0786s、source1921稳定、actual normal/binary/frontendgen stable、allgroups absent、owned[]、scratch_absent与cleanup_completed都true、error null，最低外14,910,332,928B/内10,966,614,016B。root另核两PID ESRCH、42原件双盘一致与原始raw/final/results。后续作者no-compiler观察遇到root新授N1编译发生在Q052已交接后，不回写原run或归为其失败。

root接受source04+实际10；作者仅12源码路径提交95428b25，wave1 cherry-pick为8d1033bb，无冲突；集成12路径完整bytes与source04逐SHA相同且tree clean。集成定向native与最终App仍待，不重复把历史28当当前整包App验证。两卡写权已交root，作者无新native任务。
