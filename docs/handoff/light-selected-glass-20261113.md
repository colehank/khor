# Light selected glass — 2026-11-13

In translucent light desktop windows, navigation and active tabs inherited the solid theme's opaque white `glass-sel`. The scoped tint now transmits the backdrop, keeping at least80% white behind the existing olive glyphs and increasing with window opacity. Solid windows, mobile and dark selection retain their original values. Shadows, rims and foreground opacity are unchanged.

The App Lab also seeds its requested theme into its isolated fixture before App's first settings read. Previously, persisted light settings could race a fresh dark preview. Production settings behavior is unchanged.

Validation:

- Red83e917ab, run43372318296: both engines confirmed opaque navigation/tab fill in four transparent light configurations; retained WebKit theme-reload race evidence.
- Green68f2ceff, run43373078985:14 Chromium/WebKit configurations,28 actual production pill samples,8 pixel pairs; backdrop channel response14.93–24.31. Solid/dark controls pass.
- Unit/gates/mobile static, original appearance renderer gate, direct App and live Storybook/Vite hot updates pass. This is browser/renderer evidence, not AppKit compositor acceptance.
- Verified artifact14013985370, SHA256 `60699cc60f7077383935e5211278b684e3dd7e7cd5a03e6310846332b890c438`; evidence `/Volumes/disk0/Khor/evidence/lab-workbench-20261113/glass-fixed-68f2`.

Review entrance: http://127.0.0.1:23961/workbench/?path=/story/khor-app--gui (open this entrance again to acquire the newest pinned version). Choose light theme and frost. The user defers the Air's native opaque-bar check to the next installed version. No release or main merge in this follow-up.
