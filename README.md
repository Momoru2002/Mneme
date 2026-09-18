# Mneme

**A local-first knowledge vault.** Keep your notes as plain Markdown files on
disk, with full-text search, templates, multi-user access control, and a
tamper-evident audit log — all on your machine, no server, no network by default.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
![Platforms](https://img.shields.io/badge/platform-macOS%20%7C%20Linux-lightgrey)
[![CI](https://github.com/Momoru2002/Mneme/actions/workflows/ci.yml/badge.svg)](https://github.com/Momoru2002/Mneme/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Momoru2002/Mneme?include_prereleases&sort=semver)](https://github.com/Momoru2002/Mneme/releases)


## What is Mneme?

Mneme (named after the Greek Titan of memory — root of the word "mnemonic")
is a desktop app that turns any folder of Markdown files — what it calls a
**Well** — into a fast, searchable, access-controlled knowledge base. You point
it at a directory of `.md` files and keep writing in plain Markdown; Mneme adds
search, templates, user roles, and an audit trail on top, without ever moving
your files off your computer.

Everything runs inside a single application: a native window rendering a React
interface, backed by a compiled Rust core that owns your data. There is no
background server to manage, no account to sign up for, and no outbound network
connection unless you explicitly turn one on.

## Features

- **Local-first, plain files** — your content is ordinary `.md` files in a folder
  you choose. Open them in any other editor any time; Mneme never locks them away.
- **Full-text search** — instantly find notes across the active Well.
- **Templates** — scaffold new notes from reusable templates.
- **Multi-user access control** — role-based permissions gate every change, so a
  shared Well can have readers, editors, and admins.
- **Tamper-evident audit log** — every change is recorded in an append-only log you
  can review.
- **Private by default** — no network access until you opt in.
- **Web access (opt-in)** — start a local companion server to open your Well in a
  browser tab, with an option to allow other devices on the same network (e.g.
  your phone) to connect too, gated by a session token.
- **Cross-platform** — native builds for macOS and Linux (Windows support is in
  progress).

## Download

Grab the latest build for your OS from the
[**Releases**](https://github.com/Momoru2002/Mneme/releases) page:

| OS      | File                        |
| ------- | --------------------------- |
| macOS   | `.dmg` (universal)          |
| Linux   | `.AppImage` or `.deb`       |
| Windows | _coming soon_               |

### Install notes (unsigned builds)

Releases are not yet code-signed, so your OS may warn on first launch:

- **macOS** — right-click the app → **Open**, then confirm. If it is still
  blocked, run:
  ```sh
  xattr -dr com.apple.quarantine /Applications/Mneme.app
  ```
- **Linux** — make the AppImage executable: `chmod +x Mneme_*.AppImage`.

## Getting started

On first launch Mneme shows a one-time setup screen to create your admin account,
then opens to **Add your first Well**. Choose a folder of Markdown files (or an
empty folder to start fresh) and you are ready to write, search, and organize.

## How it works

Mneme is built as a single desktop process:

```
┌────────────────────────────────────────────────────┐
│ Desktop shell (Rust core, single process)           │
│  ┌────────────────────────────────────────────────┐ │
│  │ UI — React 19 + Vite + Tailwind                  │ │
│  └───────────────────────┬────────────────────────┘ │
│                          │ in-process calls          │
│  ┌───────────────────────┴────────────────────────┐ │
│  │ Rust core                                        │ │
│  │   auth · wells · files · folders · search ·      │ │
│  │   templates · settings · users · audit           │ │
│  │   (every change checked against your role)       │ │
│  └───────────────────────┬────────────────────────┘ │
│  ┌───────────────────────┴────────────────────────┐ │
│  │ Embedded SQLite index   (~/.mneme/mneme.db)      │ │
│  └────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────┘
```

Your Markdown stays in your Well folder; a private SQLite database under `~/.mneme`
holds the search index, accounts, and settings.

### Built with security in mind

- The UI never opens a socket or a port — it talks to the core through in-process
  calls, so there is no local web server to attack.
- Passwords are hashed with **argon2id**; sessions live only in memory and end when
  you close the app.
- `~/.mneme` is created private (`0700`) and the database file is forced to `0600`.
  Mneme refuses to run if that folder lives inside cloud storage, which would
  corrupt the database.
- Every file operation is confined to the active Well's folder — path traversal
  (`..`), absolute-path escapes, and symlink tricks are all rejected.

## Build from source

### Prerequisites

- **Node.js ≥ 22** and **pnpm ≥ 9** (pinned in `package.json`).
- **Rust 1.92.0** (pinned via `rust-toolchain.toml`) plus the
  [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS.

### Develop

```sh
pnpm install
pnpm --filter @mneme/desktop dev
```

### Build

```sh
pnpm --filter @mneme/desktop build   # produces the installer(s) for your OS
```

### Test & lint

```sh
# Rust core:
cd apps/desktop/src-tauri && cargo test && cargo clippy --all-targets --all-features

# Web UI:
pnpm --filter @mneme/web test
pnpm --filter @mneme/web typecheck

# Whole repo:
pnpm lint
```

## Contributing

Issues and pull requests are welcome. Before opening a PR, please run `pnpm lint`
and the tests above. See [`docs/RELEASING.md`](docs/RELEASING.md) for how releases
are cut.

## Contributors

- [Momoru2002](https://github.com/Momoru2002) — maintainer

## Attribution

Mneme started as a rename/fork of [**Mimir**](https://github.com/ecowangsa/mimir)
by [ecowangsa](https://github.com/ecowangsa), reused here under the terms of its
[MIT license](LICENSE). All credit for the original architecture, Rust core, and
UI goes to the upstream project; this fork carries forward its own changes
(currently: opt-in LAN access for web mode) under the Mneme name.

## License

[MIT](LICENSE) © 2026 momoru2002
