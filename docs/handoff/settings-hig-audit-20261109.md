# Settings review · 2026-11-09

Scope: existing Khor settings, same production App and design tokens. Main-window integration is the owner's product direction; Apple HIG also permits a dedicated macOS settings window. Publication is on hold.

## Implemented

- Main App, FloatNav, shortcut, macOS menu and editor requests converge on the same settings navigation; native code no longer creates a settings window. Existing main frame is focused without resizing.
- Settings has its own seven-category sidebar/title, without My Khor/storage/diagnostics mixed into that sidebar. Utilities retain their existing routes. No new General category.
- Last category is stored on the viewing device, restored on the next opening, and stale/unavailable storage falls back safely. Explicit deep links retain their category and validated machine target.
- Selected categories expose aria-current. Existing keyboard list navigation, grouped forms, native selects and switches are reused.
- Lab configuration rows now come from the canonical Rust metadata/defaults and real localization catalog, with isolated Lab browser values. No handwritten configuration inventory.

## Findings and recommended order

| Priority | Current evidence | User impact | Next change to review |
| --- | --- | --- | --- |
| P1 | TerminalPage combines local TerminalSection preferences and per-machine AgentRecognition | The user cannot immediately tell which machine a change affects | Separate local viewing options and machine-specific settings; make scope visible beside the control |
| P1 | ConfigSections applies setConfig asynchronously without per-row pending/order feedback; material controls use their own queue/rollback | Slow or failed changes can be difficult to interpret; a race has not yet been reproduced | Exercise delayed/failure cases, then unify feedback and serialization where the tests prove necessary; avoid a toast for every successful toggle |
| P1 | No search of actual setting labels/choices | Users must guess one of seven categories | Search a shared item index; results open and locate the actual row. Filtering only category names is insufficient |
| P1 | AboutPage suppresses failed fetchAbout reads by setting null; opening the storage directory also swallows launch failures | Missing facts or an unresponsive action have no explanation | Keep the available information and show a compact local error/retry for the failed operation |
| P2 | About combines version/updater with storage paths/cache and companion CLI details | Ordinary update tasks are mixed with maintenance information | Keep version and updates prominent, progressively disclose technical details and link existing Storage |
| P2 | Existing form layouts need checks with long translations, narrow widths and larger text | Wrapping, reachable controls and keyboard focus may fail | Test actual component geometry and focus; do not infer a defect solely from source |

## Review boundaries

Do not introduce another settings skin or icon pack, duplicate system accessibility settings, remove agent capability recognition without tracing its backend, or redesign all categories without discussion. These findings are a review plan, not a claim that every proposed change has been implemented.

Frontend Lab can verify application routing, restored category, real forms, selected state and responsive layout. Native menu focusing, window material and platform keyboard behavior still require native verification. Browser viewport assertions alone do not prove native frame behavior.

## Primary references

- [Apple HIG: Settings](https://developer.apple.com/design/human-interface-guidelines/settings): defaults, focused settings, expected navigation and restoring the latest pane.
- [Apple HIG: Search fields](https://developer.apple.com/design/human-interface-guidelines/search-fields): discovering and reaching content, including deeply located preferences.
- [Apple HIG: Selection and input](https://developer.apple.com/design/human-interface-guidelines/selection-and-input): appropriate system controls.

Official pages and their Markdown counterparts reviewed on 2026-11-09. Main-window choice follows user instruction, not a purported HIG requirement.
