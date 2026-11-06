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

The first scene set is `gui`, `codex`, `terminal`, `stream`, `content`, `list`; full uses `live`. Extend `apps/gui/dev/lab/scenes.ts` and the data-only fixture to cover additional scenarios. Enter a scenario through the actual list action. Keep component coordinates, CSS, viewport handling and gesture hosts in production source; never patch them in a scenario. Missing fixture commands fail explicitly.

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

## Remote workflow

Install `.github/workflows/developer-lab.yml` in a private build-control repository, then select it with `--builder-repo owner/repository`. The repository must be private. Snapshots are parentless source envelopes under `source/`, so their workflows cannot run as control-repository workflows.

The frontend job checks generated contract compatibility, reuses compatible trusted generation or regenerates from the submitted source, including when the release artifact has expired, builds the complete App and uploads the usable bundle before browser checks. A full request builds a separate debug Tauri host on a trusted macOS runner. No signing credentials or production store are required. Trusted native builds do not restore shared Rust caches writable by the rented worker. Native results from an untrusted rented worker must not be substituted for these artifacts.

Use the source manifest and browser report together during review. Chromium size/touch emulation verifies browser geometry; it does not replace real iPhone or native WebView acceptance. The manager does not mark `user_accepted` automatically. Merge and release follow the project's normal review process.
