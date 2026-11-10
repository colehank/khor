# App 图标源码与生成

当前图标：07 对齐。Mobile 与 Desktop 由一份参数和一份绘制程序生成：

- `app-icon.json`：生产参数，调校台的新会话默认值。
- `../../../dev/icon-lab/renderer.ts`：纯 SVG 绘制程序，浏览器与 CLI 共用。
- `mandala-glass.svg`：生成的 1024 × 1024 完整方形图稿，不手工修改。
- `generated.json`：输入、输出的 SHA-256 与生成工具版本，用于检测资源失同步。

`mandala-16.svg`、`mandala-glyph.svg`、`tray.svg` 是独立的小字形/托盘用途，
不作为 App 安装图标的另一个来源。

## 调整与生成

在仓库根目录打开[调校台](../../../dev/icon-lab/README.md)：

```sh
python3 scripts/app_icon.py serve
```

定稿后导出参数 JSON。在具备 Node 22.19+、Python 3.11+、resvg 0.47.0 和
带 Icon Composer 的 Xcode 26 的 macOS 上运行：

```sh
python3 -m pip install -r scripts/requirements-app-icon.txt
python3 scripts/app_icon.py generate --preset /path/to/exported.json
python3 scripts/app_icon.py check
```

不传 `--preset` 时重新生成当前生产参数。生成器先在临时目录完成验证、栅格化、
封装与系统预览导出，全部成功后才写入生产参数及资源；失败保留现有资源。
不运行 App 构建或原生 catalog 编译。Pillow 固定为 12.2.0；resvg 版本不符会报错。
原生预览通过当前 `xcode-select` 所选 Xcode 的 `ictool` 导出。

## 平台资源

`Icon.icon/Assets/app-icon.png` 是不透明 RGB、sRGB 的 1024 像素母图，
与 iOS marketing 的 `AppIcon-512@2x.png` 字节相同。iPhone/iPad 尺寸按
`gen/apple/Assets.xcassets/AppIcon.appiconset/Contents.json` 统一派生。
Icon Composer 的单图层不再附加玻璃材质或阴影，背景与外缘仍由系统决定。

兼容资源 `icon.png`、`icon.icns`、`icon.ico` 从同一图稿生成，只应用原有的连续圆角
透明蒙版，不额外绘制玻璃外缘。不同系统的遮罩、布局和材质处理仍可能产生外观差异。
不要用手工拷贝某张 PNG 的方式只更新一个平台。

`App icon` CI 先对工具运行 strict TypeScript 检查和行为测试，再检查参数与 SVG 一致、生成资源清单/哈希/尺寸/颜色模式、iOS/macOS
母图相同，并在 macOS 26 编译实际 layered catalog。完整安装后的 Finder、Dock、
启动器与 iOS 主屏幕验收仍需后续安装构建；浏览器或 `ictool` 预览不能替代这些验收。
