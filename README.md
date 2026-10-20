# NAS Sync

NAS Sync is the Tauri desktop client that keeps folders of a computer in sync with folders of
the NAS, through the `nas-sync-manager` application installed on the NAS. The desktop targets are
**Linux and Windows**. The interface supports English and Italian, with light and dark themes.

## Features

- A connection wizard with a NAS address entered manually or discovered on the local network,
  a test of the link that shows the fingerprint of the NAS authority to confirm, and the account
  credentials, used once to obtain a certificate of the computer.
- Sync rules between a folder of the computer and a folder of a data disk of the NAS, two-way,
  computer to NAS or NAS to computer; pause, resume, edit and removal.
- Per-sync and overall states covering synced, syncing, paused, and disconnected operation.
- The history of the updated files, kept on the computer.
- A tray icon with the state, a menu to pause or resume, open the client or quit, and a summary
  window with the state and the last updated files.
- Options for the NAS address, disconnection, notifications, startup with the system, and
  bandwidth limits.

## Getting started

### Requirements

| Requirement      | Details                                                                              |
| ---------------- | ------------------------------------------------------------------------------------ |
| Node.js          | A version supported by `package.json`; CI uses Node.js 24.                           |
| Package manager  | pnpm for local development, or npm with the committed npm lockfile.                  |
| Rust             | Version 1.89 or later.                                                               |
| Native libraries | The Tauri build dependencies of the system, and `libayatana-appindicator3` on Linux. |
| NAS software     | A gateway with the client routes and `nas-sync-manager` installed on the NAS.        |
| Network          | HTTPS to the gateway and TCP 22000 to the NAS on the local network.                  |

```sh
pnpm install
pnpm run tauri:dev
```

The first `tauri:dev` or `tauri:build` downloads the official Syncthing release of the platform
and checks its SHA-256 (`scripts/build/engine.mjs`).

### Build commands

| Command               | Result                                                             |
| --------------------- | ------------------------------------------------------------------ |
| `npm run build`       | Type-check the project and compile the frontend into `dist/tauri`. |
| `npm run tauri:build` | Build the native application and its desktop packages.             |

## Using the application

1. Type the address of the NAS, or select **Search** to discover NAS devices automatically,
   then select **Verify**. A detected NAS that still needs initial setup cannot be selected
   until its setup is completed in the NAS web interface.
2. Compare the fingerprint with the one of the authority in the certificates of Security Center
   on the NAS, and confirm it.
3. Sign in with the NAS account and choose the name of the computer. The password is not kept.
4. Select **New sync**, choose a folder of the computer, a folder of a NAS disk and a mode.

Closing the window keeps the client in the tray. On Windows a click on the tray icon opens the
summary and a double click opens the client; on Linux the summary is in the tray menu.

## NAS requirements

Pairing needs the account password and, while mTLS is on in Security Center, is refused because the
computer has no certificate yet. The computer appears in the devices of Security Center, where
revoking or removing it also ends its syncs on the NAS. With network discovery off, the NAS is
reached by its address. See the [NAS integration guide](NAS-INTEGRATION.md).

## Documentation

| Document                                                   | Audience and scope                                          |
| ---------------------------------------------------------- | ----------------------------------------------------------- |
| [Development guide](DEVELOPMENT.md)                        | Architecture, commands, engine, storage and verification.   |
| [NAS integration guide](NAS-INTEGRATION.md)                | Certificate, gateway routes, sync application and firewall. |
| [Changelog](CHANGELOG.txt)                                 | Release notes consumed by the release pipeline.             |
| [GLib compatibility patch](backend/vendor/glib/PATCHES.md) | The vendored Linux dependency fix and its regression test.  |

## Validation scope

Automated checks cover the interface, simulated commands, the certificate and gateway exchanges
against a local HTTPS server, the engine API against a simulated Syncthing and, on demand, the
real Syncthing binary. The real transfer of files with a NAS, Windows, the tray of the desktop
environments, the protected storage of the systems and the installers still need a test on the
machines.
