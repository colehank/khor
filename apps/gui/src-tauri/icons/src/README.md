# App 图标：定稿、外观与生成

这里是安装后 App 图标的唯一维护入口。设计系统只链接这里；各平台不得自行挑选母图。
机器可读的选择映射是 [`app-icon-appearances.json`](app-icon-appearances.json)，
`scripts/app_icon.py` 读取它，生成 macOS、iOS 及兼容资源，并检查它们没有漂移。

## 当前定稿

| 外观 | 母稿 | 来源 |
| --- | --- | --- |
| 浅色 / 默认 | [`app-icon-light.svg`](app-icon-light.svg) | 用户封存的米白 v14 balanced，绿光响应 0.70；不是后续环境光试稿 |
| 深色 | [`mandala-glass.svg`](mandala-glass.svg) | 原 07；由 `app-icon.json` 与 `dev/icon-lab/renderer.ts` 生成 |

浅色稿是自包含 SVG，保留定稿中的位图玻璃底层和 SVG 光层，不依赖实验目录。
`app-icon-light-approval.json` 保存定稿来源、用户确认与 SHA-256。
浅深两稿独立定稿，不用反色、自动换底色或重建材质来互相推导。
修改浅色稿需重新完成视觉定稿并更新确认记录；仅重写生成清单不能通过定稿校验。
深色的参数与 SVG 一致性校验继续保留。

## 系统如何选择

选择依据是**系统的图标外观**，不是 khor 内部的界面主题。系统设为自动时，图标跟随系统；
用户明确选浅色、深色、着色或透明外观时尊重系统选择，不用运行时 API 强制覆盖。
只把 khor 窗口切成深色不会更换 Dock 或主屏幕图标。

| 安装平台 | 选择与降级 |
| --- | --- |
| macOS 26+ | 同一个 `Icon.icon` 的 Default 用浅色定稿，Dark 用原 07，由系统选择；着色/透明等系统外观交给系统处理，另做实机验收 |
| iOS / iPadOS 18+ | 同一个 `AppIcon.appiconset` 的 Any 用浅色定稿，Dark 用原 07；使用主屏幕的自动/明暗设置；未单独定稿的着色外观由系统生成 |
| 旧 macOS、旧 iOS / iPadOS | 使用浅色默认稿；不声称支持系统未提供的自动明暗切换 |
| Windows / Linux | 当前使用浅色静态 PNG / ICO；桌面启动器图标自动明暗尚未实现，不能把窗口图标切换等同于安装图标切换 |

Apple 的遮罩、外缘及系统特效会随系统版本和外观设置变化；Mobile 与 Desktop 的
**对应外观母图逐字节相同**，不要求系统合成后逐像素相同。
自定义玻璃、光线和底色已在母图中，Icon Composer 图层关闭额外玻璃与阴影，位置和比例固定。

## 调整与生成

深色 [调校台](../../../dev/icon-lab/README.md) 继续读取原 07 参数，不负责修改浅色定稿。
`native.html` 查看两种外观的 macOS / iOS 系统导出；滑块草稿不覆盖安装资源。

在具备 Python 3.11+、Pillow 12.2.0、resvg 0.47.0 和带 Icon Composer 的 Xcode 26 的
macOS 上，重新封装已定稿的两张图：

```sh
python3 -m pip install -r scripts/requirements-app-icon.txt
python3 scripts/app_icon.py generate
```

默认复用已验证的深色 SVG；若深色绘制程序或参数已变，会拒绝复用。
要把新的深色调校参数生成为 SVG，另需 Node 22.19+：

```sh
python3 scripts/app_icon.py generate --preset /path/to/exported.json
python3 scripts/app_icon.py check
```

生成器先在临时目录完成两种外观的栅格化、封装、原生预览及资源验证，然后更新资源。
它不构建 App，也不编译原生 catalog。`check` 是完整只读检查，包含实际执行 TS 绘制程序。
`generated.json` 记录输入、输出 SHA-256 和工具版本；不要手工补某一平台的 PNG。
`Icon.icon/icon.json` 和 iOS `Contents.json` 都由策略生成，不分别手改。

## 平台资源与验收

`Icon.icon/Assets/app-icon.png` 与 `app-icon-dark.png` 为不透明 RGB / sRGB 的 1024 像素母图。
它们分别与 iOS `AppIcon-512@2x.png`、`AppIcon-dark.png` 字节相同。
iOS 采用 Apple 的 Single Size 目录，由 Xcode 为 iPhone / iPad 生成系统所需尺寸。
兼容资源 `icon.png`、`icon.icns`、`icon.ico` 从策略的 `fallback`（浅色）生成，
仅应用原有连续圆角透明蒙版，不额外绘制玻璃外缘。

`App icon` CI 检查 TS 类型/行为、深色参数/SVG、浅色定稿指纹、资源清单/哈希/尺寸/颜色模式、
两种外观的 iOS/macOS 母图一致性，编译 macOS 与 iOS 实际 catalog，并导出两种原生外观。
回归夹具故意移除深色映射、篡改 iOS 深色图或浅色定稿，即使重算清单也必须失败。

安装验收另行执行：macOS Finder、Dock、启动器；iPhone/iPad 主屏幕、搜索及设置；
分别切换自动、固定浅色、固定深色，并测试旧系统的默认稿和系统着色/透明外观。
需检查 App 冷启动、运行中及退出后；仅预览或 catalog 编译通过，不等于安装验收通过。

## 会话与托盘图标

`mandala-16.svg`、`mandala-glyph.svg`、`tray.svg` 是独立的小字形或托盘用途，不读取安装图标策略。
`khor-session-avatar.json` 保存用户确认的会话头像参数，`BrandIcons.IconKhor` 直接读取
`mandala-glyph.svg`，统一新建入口、列表、标签和手机药丸；khor 浮现头像实际标记占比 65%。
会话头像继续使用每台机器自己的底图、墨色和基础衬色，其他会话类型不变。

## Apple 参考

- [使用 Icon Composer 创建图标](https://developer.apple.com/documentation/xcode/creating-your-app-icon-using-icon-composer)
- [配置 Asset Catalog 的 Single Size 与明暗外观](https://developer.apple.com/documentation/xcode/configuring-your-app-icon)
- [iPhone 主屏幕图标外观设置](https://support.apple.com/en-gb/guide/iphone/iph385473442/26/ios/26)
