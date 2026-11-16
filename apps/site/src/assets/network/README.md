# Network type symbols

Approved by the project owner from the local SF Symbols comparison:

- `lan.svg`: Apple SF Symbols `cable.connector`
- `wifi.svg`: Apple SF Symbols `wifi`
- `cellular.svg`: Apple SF Symbols `cellularbars`

Source: the macOS system symbol API (`NSImage`). Copyright Apple Inc.; these are
Apple system-provided images, not MIT-licensed Khor or Phosphor artwork. Terms:
https://images.apple.com/legal/sla/docs/xcode.pdf (System-Provided Images).

The approved 96 × 96 transparent renders are embedded in SVG alpha masks so the
original silhouettes inherit the diagram's current color. All instance IDs are
isolated by `ConceptMark.astro`. No fonts or native APIs are needed at runtime.

Optional regeneration on macOS: `node apps/site/scripts/render-network-symbols.mjs`.
The exported assets are committed; standard static builds do not render symbols.
