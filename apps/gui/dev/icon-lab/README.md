# 图标调校台

独立开发工具，不打包进产品。浏览器与资源生成器共用 `renderer.ts`，默认读取
`../../src-tauri/icons/src/app-icon.json`。新浏览器从当前生产参数开始，已有的浏览器调整
继续保存在 `khor.icon-lab.v1`；“恢复当前图标”可回到生产参数。

在仓库根目录运行（Python 3.11+、Node 22.19+，无需 npm 安装或 App 构建）：

```sh
python3 scripts/app_icon.py serve
```

打开 <http://127.0.0.1:23962/>。手机在同一网络时用
`python3 scripts/app_icon.py serve --host 0.0.0.0`，访问电脑的局域网 IP、相同端口。
服务器只提供工具文件、产品 token、当前参数和系统预览，不提供整个仓库或目录列表。
界面直接使用产品 `tokens.css`；服务器仅把 Tailwind theme 容器转换成浏览器可用的变量。

手机：预览固定在上方，下面切换外光、内光、玻璃、背景、形体和保存；横屏时左右排列。
支持触摸滑块、数值输入、方向盘、撤销、暂存对照、参数导入导出、SVG 和 PNG 导出。
浏览器中的圆角是示意；“当前图标的两端系统效果”显示提交参数对应的 Icon Composer 导出，
不会随着尚未保存到仓库的滑块调整变化。浏览器 PNG 与 resvg 的滤镜/渐变采样可能略有差异，
安装资源以统一生成器输出为准。

参数定稿后导出 JSON，再运行[统一生成命令](../../src-tauri/icons/src/README.md)。
浏览器没有写入仓库的接口。调校不直接覆盖生产资源。

## 材质与光

五颗圆珠与内环是不透明材质，菱形板与外环模拟玻璃。白色外光由设置方向照入坛城，
另一侧透出绿色；外光关闭时透出光也关闭。内部总发光共同控制五颗圆珠和内环，
独立于外光。反射和背景晕染是可调的二维 SVG 模拟。

## 类型与运行链路

`app.ts`、`renderer.ts`、`render.ts`、`renderer.test.ts` 是唯一维护源码。
参数键从控件定义推导；JSON 和浏览器存储在 `unknown` 边界校验，UI 状态、预设、
光源结果、渐变与圆珠几何有明确类型。DOM 绑定验证实际元素类型，避免把任意元素断言成输入框。

Node 直接运行可擦除类型的 TS；服务通过 Node 内置 `stripTypeScriptTypes` 派生浏览器
JavaScript 响应，`.ts` 模块以 `text/javascript` 提供，保留模块的 `.ts` 导入路径。
旧 `.mjs` URL 是同一源码的兼容别名。派生 JS 不写入仓库，也没有第二套手写实现。
运行时剥离类型不执行类型检查；`npm run check` 是独立的 strict 类型门，CI 必跑。
它同时启用 `noUncheckedIndexedAccess`、`exactOptionalPropertyTypes` 和 `erasableSyntaxOnly`。
严格检查只安装这个工具的固定 TypeScript/Node 类型依赖，无需整套 GUI 依赖。

类型擦除机制见 [Node 22.19 官方 API](https://nodejs.org/download/release/v22.19.0/docs/api/module.html#modulestriptypescripttypescode-options)。

## 来源与维护

`reference-07.png` 是本轮用户选定的 07 静态视觉参照（此前生图迭代产物），只用于对照，
不参与 SVG 渲染和安装资源生成。当前生产图稿为基于它重新编写的程序化 SVG。
旧参数夹具保留版本 1 的兼容性；旧 `spillAngle` / `directionalSpill` 字段可导入导出，
不再产生固定方向的绿色光。新参数缺省值兼容旧导出文件。

```sh
npm ci --prefix apps/gui/dev/icon-lab
npm run check --prefix apps/gui/dev/icon-lab
npm test --prefix apps/gui/dev/icon-lab
python3 -m unittest discover -s scripts/tests -p 'test_app_icon.py'
python3 scripts/app_icon.py check
```
