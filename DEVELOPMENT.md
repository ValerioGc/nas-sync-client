# Development guide

This guide describes the implementation of NAS Sync. The [README](README.md) describes the user workflow. The [NAS integration
guide](NAS-INTEGRATION.md) covers the NAS side.

## Technology and project boundaries

Vue 3, TypeScript, Vite and SCSS in the frontend; Rust and Tauri 2 in the backend. The project
started as a copy of NAS Finder and keeps its tooling, scripts, lint and style rules. It does not
import code from the NAS projects.

Dependencies beyond NAS Finder include the Tauri `tray-icon` and `image-png` features, the official
plugins `dialog`, `single-instance`, `autostart` and `notification` (used from Rust only),
`reqwest` with rustls, `rustls` and `ring` (already in its tree), `tokio`, and `keyring` (the
Credential Manager on Windows, the Secret Service on Linux). The frontend adds no package.

## Repository layout

| Path                                   | Responsibility                                                        |
| -------------------------------------- | --------------------------------------------------------------------- |
| `src/App.vue`                          | Main window with setup, state, syncs, history, and dialogs.           |
| `src/components/MiniView.vue`          | Summary window of the tray (`index.html#mini`).                       |
| `src/components/ConnectWizard.vue`     | Address, fingerprint and account steps.                               |
| `src/components/`                      | Syncs, rule dialog, NAS folder browser, history, options and common.  |
| `src/composables/useClient.ts`         | State of a window, read at each `client-state` event (30 s fallback). |
| `src/services/client.ts`               | Typed wrappers of the native commands.                                |
| `src/i18n/`                            | English and Italian interface messages.                               |
| `backend/src/i18n/`                    | English and Italian tray labels and notification messages.            |
| `backend/src/client.rs`                | Link, syncs, reconciliation of the engine and overall state.          |
| `backend/src/nas.rs`, `tls.rs`         | Gateway exchanges and trust in the NAS authority.                     |
| `backend/src/engine.rs`                | Syncthing as a supervised child process and its API.                  |
| `backend/src/tray.rs`, `notices.rs`    | Tray icon, menu, summary placement and notifications.                 |
| `backend/src/settings.rs`, `secret.rs` | `client.json` and the private key in the protected storage.           |
| `backend/src/discovery.rs`             | The discovery of NAS Finder, without a saved list.                    |
| `scripts/build/engine.mjs`             | Download and SHA-256 check of the official Syncthing release.         |

## Native command API

| Command                                            | Behavior                                                             |
| -------------------------------------------------- | -------------------------------------------------------------------- |
| `get_state`, `get_history`                         | The snapshot of the client and the newest history entries.           |
| `discover_nas`                                     | NAS Finder discovery on the local network.                           |
| `probe_nas`                                        | Reaches the gateway and returns the authority fingerprint.           |
| `connect_nas`                                      | Certificate issue, pairing and link, with the confirmed fingerprint. |
| `move_nas`, `disconnect_nas`                       | New address of the same authority; unpairing.                        |
| `nas_folders`, `pick_folder`                       | Folders of the NAS disks; folder dialog of the system.               |
| `create_sync`, `update_sync`, `remove_sync`        | Syncs of this computer, through the NAS.                             |
| `set_paused`, `save_preferences`, `set_appearance` | Pause of every sync, options, language and theme.                    |
| `open_main`, `hide_mini`, `quit_app`               | Windows and exit, from the summary.                                  |

The main window may call every command but `hide_mini`; the summary only the state, the history,
the pause and the window commands (`backend/capabilities/`). Errors cross IPC as codes the
frontend translates.

## Localization and dependencies

Keep user-facing translations in `src/i18n/lang/` and `backend/src/i18n/`.
Hardcoded diagnostics, source comments, test descriptions and project documentation use English.
Tests may assert Italian output when checking the Italian translations.

Development dependencies include tools loaded indirectly by the build and test configurations.
`jiti` loads the TypeScript ESLint configuration, `vue-eslint-parser` is a required peer of
`eslint-plugin-vue`, `sass` compiles component styles, and `@vitest/coverage-v8` supplies the
coverage provider. Their absence from application imports does not make them unused.
The Tauri CLI, Allure CLI and `npm-run-all2` are used by package scripts.
Keep both `package-lock.json` and `pnpm-lock.yaml`. CI installs with npm, while local development
also supports pnpm. `npm run lint:lock` checks the npm manifest and lockfile.

## Trust and secrets

The first link accepts the certificate of the gateway only to download `/auth/ca.crt`, checks
that the certificate of the gateway comes from that authority and shows its SHA-256. Every later
link trusts that authority only, whatever name the NAS is reached at. The certificate and the
authority are in `client.json`; the private key is in the protected storage of the system and
never on disk. The account password is used for sign-in and confirmation only, then dropped.

## Engine

`engine.rs` runs the bundled Syncthing with `--home` in the data folder of the application, an API
on `127.0.0.1` at a port chosen at each start, the key in `STGUIAPIKEY` and `STMONITORED=1` (one
process, restarted by the client). The first `config.xml` disables global and local
discovery, relays, NAT traversal, usage and crash reports, and automatic upgrades. The listener
binds only to loopback. The NAS is reached at its address; folders are `sendonly`, `receiveonly` or
`sendreceive`, the opposite of the NAS. The client gives the engine its configuration at each
start and after every change, removing folders and devices nobody asks for.

An engine left running by an application that was killed (a crash, a reload of `tauri dev`) holds
the lock of its home, and a new one would stop at once with "Failed to acquire lock". The API port of
each responsive engine is kept in `api-port`. Before starting, the client asks that port to
shut down (`POST /rest/system/shutdown` with the same key). What the engine writes goes to
`engine.log` in its home, written again at each start. This log provides startup diagnostics when
the client reports that the engine is unavailable. The pairing waits up to 20 seconds for an engine still starting.
The long poll of the events waits 20 seconds, under the 30 of the HTTP client.

The client sends `client-state` to the windows after each reading of the state (every 5 seconds,
or at once after a change); a window reads the state then, and on its own every 30 seconds, and
asks for no history before a NAS is linked.

The engine is given only the syncs whose computer folder was chosen on this computer. The `chosen` map in
`client.json` associates each sync ID with its local folder, written when a sync is created or edited here. A sync
the NAS lists with another folder, or one this computer never chose, shows `unchosen` and stays out
of the engine until it is edited here; the list the NAS gives at the first start after the update is
taken as chosen. `forbidden()` refuses the top of a disk, the home folder whole, the folders of the
system and those of the application. Each folder keeps for 30 days in `.stversions` what the NAS
deleted or replaced (versioning `trashcan`).

The history follows `LocalChangeDetected`, `RemoteChangeDetected` and failed `ItemFinished`
events, at most 300 entries in `history.json`.

## Verification

```sh
npm run format:check
npm run lint
npm run type-check
npm run test:coverage
npm run test:e2e
npm run test:build
npm run release:test:unit
cargo fmt --manifest-path backend/Cargo.toml --check
cargo clippy --manifest-path backend/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path backend/Cargo.toml
```

Frontend coverage requires at least 80% per file. The Rust tests use a local HTTPS server with
a test authority (`backend/tests/tls/fixtures`) and a simulated engine API. The optional real-engine check uses the bundled binary.

```sh
NAS_SYNC_ENGINE=backend/binaries/syncthing-x86_64-unknown-linux-gnu \
  cargo test --manifest-path backend/Cargo.toml -- --ignored real_engine
```

`-- --ignored real_keyring` writes and removes a test entry in the protected storage of the
session. On Linux the Secret Service is reached through libdbus (`libdbus-1-dev` to build).

## Release tooling

The release scripts, the audit and the build workflows are those of NAS Finder. The build
workflow downloads the engine of each platform through `npm run tauri -- build`, and installs
`libayatana-appindicator3-dev` for the tray on Linux.
