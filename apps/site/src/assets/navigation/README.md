# Website navigation icons

Apple SF Symbols, exported through the macOS `NSImage` system symbol API:

- `docs.svg`: `book.closed`
- `download.svg`: `arrow.down`
- `appearance.svg`: `circle.lefthalf.filled`
- `language.svg`: `globe`

Copyright Apple Inc. These system-provided images are covered by Apple's terms,
not the historical Phosphor MIT license retained in `LICENSE`:
https://images.apple.com/legal/sla/docs/xcode.pdf (System-Provided Images).

The transparent renders are SVG alpha masks using `currentColor`. Painted bounds
are measured and cropped to the same optical size; navigation uses a 22px SVG box
and a 44px hit target. The GitHub brand mark remains in `../github.svg`.

Optional regeneration on macOS:
`node apps/site/scripts/render-network-symbols.mjs --navigation`.
Rendering is offscreen. Normal static builds need neither AppKit nor symbol fonts.
