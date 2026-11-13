# Selected glass ground parity — 2026-11-13

The user rejected the first80%-white light selection as visually unchanged and requested following the dark implementation. The previous small pixel-response threshold was insufficient to establish the requested appearance.

Transparent desktop windows now share the existing dark mechanism: `glass-sel:var(--background)`, with `background:transparent` on the same root. The single window ground remains responsible for overall opacity; active pills add no extra white plate. Light and dark keep their own rims/shadows. Foreground opacity, solid windows and mobile are unchanged. The moving selection ghost uses the same utility.

Validation:

- Strict red760337d5, run43382886041: five light transparent configurations fail in each engine (alpha.8 or1, expected0). Solid/dark controls pass.
- Implementation1329eeef: all14 configuration/28 pill/8 pixel checks pass; original appearance, unit/gates/mobile static pass. An unhandled WebKit module-import rejection was retained, so this run was not accepted as clean.
- Checker-only eeec07bb waits for actual lazy-module completion before full-document scenario navigation and records failed module URLs; no page-error filtering. Clean run43384535546 passes both engines, with no module failures, plus direct/workbench Vite hot checks. Production bytes are identical to1329eeef.
- Final artifact14017784394, SHA256 `ba85f53c17f5a35ee293d388d2d7f930f9443a16e1a051b6fce22acf6e716bc2`; evidence `/Volumes/disk0/Khor/evidence/lab-workbench-20261113/parity-final-eeec`.

Pinned review: http://127.0.0.1:23961/versions/eeec07bb6e041ac5/assets/workbench/index.html?path=/story/khor-app--gui
Choose light theme and frost. Native Air material acceptance remains deferred to the next installed App. No main merge or release in this correction.
