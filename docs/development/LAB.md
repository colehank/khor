# Developer Lab

Lab uses the complete production `index.html → src/main.tsx → App` tree. Its bootstrap chooses a data boundary before importing the normal entry. It does not redraw a phone, composer, navigation deck or safe area. The regular Vite entry and packaged release never import the Lab bootstrap.

Choose the runtime and builder independently:

| Choice | What runs |
| --- | --- |
| `--mode frontend` | Production UI with isolated fixture data, in a browser |
| `--mode full` | Production UI plus a real isolated Rust/Tauri backend |
| `--builder local` | Build on this developer's computer using its installed toolchains |
| `--builder remote` | Submit tracked WIP source to the private build repository; acquire artifacts from trusted GitHub-hosted runners |

The first full host supports macOS arm64. Frontend Lab is portable. Full Lab uses `live` data: create disposable workspaces and sessions in its isolated store. Browser fixtures do not claim to verify native keyboard, file sharing, WebView layering, process persistence or backend behavior.

## Web workbench

Frontend bundles include Storybook 10.6. The manager root `/` or `/workbench/` opens its standard scene list, viewport toolbar and Controls panel. `/app/` remains the direct complete App entry. The workbench embeds this same production entry, pinned to the bundle's source version; it never copies App layouts. The manager is a development tool and is excluded from the release build.

The pinned Storybook 10.6.7 manager has a scoped `flex-shrink: 0` correction on its viewport frame so opening Controls cannot shrink the requested device height; browser checks measure the actual nested App viewport.

Choose Mobile 320/390/430, Android 412, Tablet 768, Desktop 1280/1440 or custom dimensions. The theme/material/background toolbar and Controls adjust desktop opacity, simulated safe areas and mobile keyboard space. Environment changes preserve the current document and draft; changing scene/data deliberately starts that fixture again. Desktop material is a CSS composition over a synthetic wallpaper/grid, using the production ground painter and terminal cutouts. Mobile retains its actual solid ground. This preview cannot reproduce native AppKit blur or diagnose differences between real Macs by itself.

Storybook viewport changes dimensions, not user-agent or input hardware. Mouse input stays mouse input; Playwright device contexts cover real browser touch/UA emulation. The keyboard control reserves space through the production viewport path and labels it as a placeholder; it does not summon an OS keyboard. Use full native Lab for real IME, touch inertia, OS file drops and WebView layering.

Developers running local hot frontend Lab can open the manager in a second process:

```sh
cd apps/gui
KHOR_LAB_ORIGIN=http://127.0.0.1:23910 npm run lab:workbench
```

The manager proxies the existing Lab entry; App changes still use that Lab's normal hot update path. Source-only developers use remotely built immutable workbench assets. Share `/workbench/?path=/story/khor-app--gui` or a pinned `/versions/<version>/assets/workbench/index.html?path=/story/khor-app--gui` for review.

## Edit source with hot updates

`lab dev` runs the actual App through Vite. It is a foreground development process: Ctrl+C stops its own server and native launcher. Its runtime declares `mutable: true`; use `lab start/update` for a fixed source version to review. The two entries use the same production App and fixture modules.

```sh
python3 scripts/dev.py lab dev --builder local --mode frontend --scene gui

# A real desktop backend, separate from the installed App.
python3 scripts/dev.py lab dev --builder local --mode full

# On a development Mac with Xcode, xcodegen, CocoaPods and an iOS runtime.
xcrun simctl list devices available
python3 scripts/dev.py lab dev --builder local --mode full --target ios --device 'iPhone 17' --port 23920
```

Frontend changes update the running source view without packaging it again. Native development uses a disposable source project, a distinct application identifier and an isolated store. Platform generators may update that project; they never rewrite the developer checkout. Restart `lab dev` after Rust, native configuration or dependency changes to prepare the corresponding native inputs; frontend edits reuse the host. iOS uses its own application container. Its native `devUrl` points at the server root because Tauri’s mobile proxy appends resource paths; the browser entry remains `/app/`. HMR connects explicitly to the loopback development port, which works through the simulator’s custom-scheme page. Only an available simulator may be selected by this command; device signing is outside this development path.

The execution computer needs Node/npm and Rust generators. An existing `CARGO_TARGET_DIR` is respected; otherwise development owns a target directory in its cache. A source-only computer keeps using `lab start --builder remote`. Developers can run `lab dev` on their chosen development worker and interact with it through remote desktop. Programs and assets from an untrusted worker must not be installed as trusted artifacts on the source-only computer.

## Commands

From any Khor working tree, use `python3 scripts/dev.py`, or install `khor-dev` with the existing `configure-local --install-command` command:

```sh
# Local builder: Node/npm and the Rust generators must already be available.
python3 scripts/dev.py lab start --mode frontend --builder local --scene gui

# Source-only computer: needs Git, Python 3.12+ and authenticated gh; no Node, Cargo or Xcode.
python3 scripts/dev.py lab start --mode frontend --builder remote --scene gui
python3 scripts/dev.py lab status
python3 scripts/dev.py lab open
python3 scripts/dev.py lab scene --scene terminal
python3 scripts/dev.py lab reset
python3 scripts/dev.py lab update
# If artifact acquisition failed: stop, then acquire the existing run without recompiling.
python3 scripts/dev.py lab start --builder remote --resume
python3 scripts/dev.py lab stop

# Stop before changing runtime/builder on the same port.
python3 scripts/dev.py lab start --mode full --builder remote
```

`start` returns after starting its lightweight manager. Build, acquisition and checks continue asynchronously. `update` snapshots the current tracked source; stage new files first. Untracked files, including local settings and credentials, are excluded. `status` records the base commit, exact tree, snapshot commit, workflow URL, available bundle and check result. Availability, checks and human acceptance are separate fields. A failed check does not pretend the already available bundle has passed.

Refresh the browser after a bundle update or scene reset. For a full Lab, `lab restart` reopens its isolated native view with the active frontend and existing store, reusing its compiled host without a new build. Lab does not reload a page while someone is typing or composing with an input method. `/app/` is the stable latest entry. `snapshot_url` pins an acquired version until cache retention removes it; its assets and source manifest remain tied to that version.

Frontend scenes include `gui`, `codex`, `terminal`, `stream`, `content`, `page`, `list`, `devices`, `workspaces`, `workspace`, `quota` and `browser`; full uses `live`. A frontend link can select its own scene and fixture sizes, independently of the manager's default:

```text
/app/?scene=list&machines=6&items=5
/app/?scene=quota&machines=3
/app/?scene=workspaces&machines=3
/app/?scene=workspace&machines=1
/app/?scene=gui&items=2
```

`machines` accepts 0, 1, 3 or 6 (default 3); `items` accepts 2, 3 or 5 (default 5). The fixture includes distinct machine avatars and connection states, six subscription/balance/budget/error examples, fourteen usage days, four Git or plain-folder workspaces per machine, paginated commit history and documents stored independently by machine and path. Changing scene or sizes resets that browser Lab's isolated fixture state. Full mode does not accept a URL scene override.

Synthetic facts must retain production domain behavior. Machine system marks use typed `vitals.system` facts. Avatars come from `khor_core::avatar` with the default style; directory templates are read and ordered by `khor_node::files::list_dir`. `lab_data` creates its own temporary workspace and never opens a node or a production store. Do not hand-write avatar paint, crop or mark-ink records, or sort a replacement directory array in the frontend fixture.

`scripts/khor_lab/fixture_data.py` validates the source hashes in `data/world.generated.json`. Backend avatar/listing or template changes regenerate it; ordinary UI changes reuse it. Local builders and remote workflows prepare the same data. A source-only developer acquires the generated fixture artifact from the trusted Lab workflow; include the generated JSON with a fixture change. Developers with Rust can regenerate it with `python3 scripts/khor_lab/fixture_data.py .` from the repository root and verify it with `--check`.

Extend `apps/gui/dev/lab/scenes.ts` and the data-only fixture to cover additional scenarios. Enter a scenario through the actual list action. Keep component coordinates, CSS, viewport handling and gesture hosts in production source; never patch them in a scenario. Missing fixture commands fail explicitly.

## Defaults and isolation

`lab.config.json` contains portable project defaults. `.khor-lab.local.json` is ignored and overrides them. Flags take precedence. A source-only developer can choose:

```json
{
  "builder": "remote",
  "cache_root": "/path/to/development-cache"
}
```

This preference applies only to that developer. It does not prevent another developer from running the complete Lab locally.

Each working tree and port owns one loopback manager. Repeated `start` attaches to its status; changing mode/builder requires `stop`. Control operations require a locally stored token. Served paths expose compiled frontend assets, not the token, native archive or backend store. `stop` addresses only the authenticated manager's own process group, never the installed App or existing session hosts.

Native Lab has a separate bundle identifier, `KHOR_HOME`, browser profile and process. It does not install over `/Applications/Khor.app`. The generated/native input contract determines when a native host may be reused. A frontend-only edit can reuse a compatible host while retaining its original compiled-source identity. A backend, runtime or host-packaging change requires rebuilding the host. Cached versions and dependency sets have retention limits; active versions are protected. The native store persists across Lab restarts so persistence can be tested.

Frontend Lab may also be viewed inside Khor's browser. The Lab build replaces only the backend-context boundary: frontend mode always uses its isolated HTTP fixture, even when the containing WebView exposes Tauri globals. It never deletes, replaces or invokes that host runtime. Full mode still requires the isolated native host and uses its native backend. The production build retains its original native-runtime detection.

## Remote workflow

Install `.github/workflows/developer-lab.yml` in a private build-control repository, then select it with `--builder-repo owner/repository`. The repository must be private. Snapshots are parentless source envelopes under `source/`, so their workflows cannot run as control-repository workflows.

The frontend job checks generated contract compatibility, reuses compatible trusted generation or regenerates from the submitted source, including when the release artifact has expired, builds the complete App and uploads the usable bundle before browser checks. A full request builds a separate debug Tauri host on a trusted macOS runner. No signing credentials or production store are required. Trusted native builds do not restore shared Rust caches writable by the rented worker. Native results from an untrusted rented worker must not be substituted for these artifacts.

Use the source manifest and browser report together during review. Chromium size/touch emulation verifies browser geometry; it does not replace real iPhone or native WebView acceptance. The manager does not mark `user_accepted` automatically. Merge and release follow the project's normal review process.

## Review and merge

Keep the product edit and its Lab on the same feature branch. A reviewed interaction is already the production implementation; it does not need to be copied from a prototype later.

1. Iterate in `lab dev`, running the affected behavior checks.
2. Commit the intended change and acquire an immutable `lab start/update` preview. Record its exact tree, check result and human feedback separately.
3. Open a focused PR. Quality builds the complete App Lab, exercises the shared browser scenarios and verifies CSS hot updates preserve a real composer draft. Source-only pre-commit checks remain lightweight. A newer PR revision cancels its outdated Quality run.
4. Before merging, verify the latest base, required Quality and the relevant native/device checks. Update acceptance after a behavior change. Signing and distribution remain in the tag-driven release workflow.

The private worker accepts `check --task ios-lab-prepare`, `ios-lab` and `ios-keyboard`. Preparation, native installation, actual Appium/XCUITest keyboard verification and human device acceptance are different results. The simulator probes own a dedicated `Khor Developer Lab` device and never reset another simulator or the installed product. The native probe waits for UIKit’s actual address control before tapping it, verifies the system software keyboard and input focus, then edits real source CSS and checks the same document and editor survive the native hot update. It enters the page through the actual browser list; it does not override the App’s narrow cold-start policy. The keyboard tool versions are pinned; the Appium server listens on loopback. Screenshots and JSON reports belong to the exact submitted source; a simulator keyboard result does not claim physical iPhone or Chinese IME acceptance.
