# Mneme

**A local-first knowledge vault.** Keep your notes as plain Markdown files on
disk, with full-text search, templates, and an app-lock password — all on your
machine, no server, no network by default.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
![Platforms](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows%20(beta)-lightgrey)
[![CI](https://github.com/Momoru2002/Mneme/actions/workflows/ci.yml/badge.svg)](https://github.com/Momoru2002/Mneme/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Momoru2002/Mneme?include_prereleases&sort=semver)](https://github.com/Momoru2002/Mneme/releases)

## What is Mneme?

Mneme (named after the Greek Titan of memory — root of the word "mnemonic")
is a desktop app that turns any folder of Markdown files — what it calls a
**Well** — into a fast, searchable knowledge base, blending Obsidian's
plain-file philosophy with a Mimir-style single-source-of-truth vault.
You point it at a directory of `.md` files and keep writing in plain Markdown;
Mneme adds search, templates, and an app-lock password on top, without ever
moving your files off your computer.

Everything runs inside a single application: a native window rendering a React
interface, backed by a compiled Rust core that owns your data. There is no
background server to manage, no account to sign up for, and no outbound network
connection unless you explicitly turn one on.

## Features

- **Local-first, plain files** — your content is ordinary `.md` files in a folder
  you choose. Open them in any other editor any time; Mneme never locks them away.
- **Full-text search** — instantly find notes across the active Well.
- **Templates** — scaffold new notes from reusable templates.
- **App lock** — a single local password, required on every launch (and honored by
  Web Access too — a locked app refuses requests even with a valid link/QR code).
  Hashed with argon2id; the password itself is never stored, only its hash, and
  the unlocked state lives only in memory for that run of the app.
- **Private by default** — no network access until you opt in.
- **Web access (opt-in)** — start a local companion server to open your Well in a
  browser tab, with an option to allow other devices on the same network (e.g.
  your phone) to connect too, gated by a session token.
- **MCP server for AI assistants** — connect Claude Desktop, Claude Code, or any
  other [MCP](https://modelcontextprotocol.io)-capable assistant directly to a
  Well over stdio: it can search, read, and write your notes as a second brain.
  See [Connecting an AI assistant](#connecting-an-ai-assistant-mcp) below.
- **Multi-device via your own sync tool** — no built-in sync engine (on purpose —
  see below), but a Well is just a folder of plain `.md` files, so putting one
  in Dropbox/iCloud/Syncthing/etc. and adding it as a Well on another device
  works today. See [Using the same Well on multiple
  devices](#using-the-same-well-on-multiple-devices) below.
- **Cross-platform** — native builds for macOS, Linux, and Windows (Windows
  support is new — see the note below).

## Download

Grab the latest build for your OS from the
[**Releases**](https://github.com/Momoru2002/Mneme/releases) page:

| OS      | File                        |
| ------- | --------------------------- |
| macOS   | `.dmg` (universal)          |
| Linux   | `.AppImage` or `.deb`       |
| Windows | `.msi` or `.exe` (NSIS)     |

> **Windows status:** the Windows build was only just added and hasn't yet had
> a release cut with real installer testing on Windows hardware — CI (see the
> badge above) compiles and runs the test suite on `windows-latest` on every
> commit, so a red badge means something is actually broken, but "CI is green"
> and "someone has clicked through the installer on a real Windows machine"
> aren't the same claim yet. If you hit a Windows-specific issue, please open
> one. One known, deliberate difference from macOS/Linux: file/directory
> permissions can't be locked down to 0600/0700-equivalent on Windows the way
> they are on Unix (there's no direct analog); Mneme instead relies on your
> Windows user profile's own default access restrictions. See the doc comment
> in `apps/desktop/src-tauri/src/perms.rs` for the full rationale.

### Install notes (unsigned builds)

Releases are not yet code-signed, so your OS may warn on first launch:

- **macOS** — right-click the app → **Open**, then confirm. If it is still
  blocked, run:
  ```sh
  xattr -dr com.apple.quarantine /Applications/Mneme.app
  ```
- **Linux** — make the AppImage executable: `chmod +x Mneme_*.AppImage`.
- **Windows** — SmartScreen will likely warn on first launch since the binary
  is unsigned: click **More info** → **Run anyway**.

## Getting started

On first launch Mneme asks you to create an app-lock password (see
[App lock](#features) above), then opens to **Add your first Well**. Choose a
folder of Markdown files (or an empty folder to start fresh) and you are ready
to write, search, and organize.

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
│  │   templates · settings                           │ │
│  └───────────────────────┬────────────────────────┘ │
│  ┌───────────────────────┴────────────────────────┐ │
│  │ Embedded SQLite index   (~/.mneme/mneme.db)      │ │
│  └────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────┘
```

Your Markdown stays in your Well folder; a private SQLite database under `~/.mneme`
holds the search index, your app-lock password hash, and settings.

### Built with security in mind

- The UI talks to the core through in-process calls — no local server is
  running unless you explicitly turn on Web Access (see Features above), which
  binds to loopback only unless you opt into LAN mode.
- **App lock**: one local password, required on every launch and re-required
  after "Lock now" — see [Features](#features) above. Hashed with **argon2id**;
  the plaintext password is never stored, only its hash, and the unlocked
  state lives only in memory for that run of the app. This gates every command
  that touches note content, including Web Access requests — a locked app
  refuses those too, even with an otherwise-valid session token.
- `~/.mneme` is created private (`0700` dir / `0600` DB file on macOS and Linux;
  Windows relies on your user profile's own access restrictions instead — see
  the Windows note above). Mneme refuses to run if that folder lives inside
  cloud storage, which would corrupt the database.
- Every file operation is confined to the active Well's folder — path traversal
  (`..`), absolute-path escapes, and symlink tricks are all rejected.

**Honesty note:** the database schema still has `users`/`role` and `audit_log`
tables left over from an earlier multi-user design that was removed (see
`rbac.rs`'s doc comment in the source) — they exist, but nothing in the app
currently writes meaningful audit entries or enforces per-role permissions.
Earlier versions of this README claimed both as working features; they
weren't, and this section now only describes what actually runs.

## Connecting an AI assistant (MCP)

Mneme ships a small companion binary, `mneme-mcp`, that speaks the
[Model Context Protocol](https://modelcontextprotocol.io) over stdio — the
same standard Claude Desktop, Claude Code, and a growing set of other AI
tools use to connect to local data. It's a third transport onto the exact
same core the desktop app and Web Access use (see the diagram above), not a
separate copy of your notes: it reads and writes the same
`~/.mneme/mneme.db` and the same Well folders on disk, live.

**Tools it exposes:** `list_wells`, `search_notes`, `read_note`, `list_notes`,
`create_note`, `update_note`, `append_to_note`, `delete_note`, `create_folder`,
`list_templates`. `well_id` is optional on every tool that needs one — it
defaults to whichever Well is active in the desktop app, so a single-Well user
never has to pass it. Ask your assistant to list its available tools once
connected to confirm.

### Setup

1. **Find the server.** `mneme-mcp` is bundled with the desktop installers and
   sits next to the Mneme executable. The easiest way to get its exact path
   (plus copy-paste snippets for the clients below) is **Settings → AI
   Assistants** inside the app. Building from source instead?
   ```sh
   cd apps/desktop/src-tauri
   cargo build --release --bin mneme-mcp
   ```
   The binary lands at `target/release/mneme-mcp` (`.exe` on Windows).
   *(Linux AppImage users: the AppImage mounts itself somewhere new on every
   launch, so its bundled path isn't stable — use the `.deb`/`.rpm`, or copy
   `mneme-mcp` to a permanent location first.)*
2. Point your MCP client at that path:

   **Claude Code** — one command, no file editing:
   ```sh
   claude mcp add --transport stdio mneme --scope user -- /absolute/path/to/mneme-mcp
   ```
   Verify with `claude mcp list`, then `/mcp` inside a `claude` session to
   confirm it's connected.

   **Claude Desktop** — edit its config file, then fully quit and reopen the
   app (closing the window is not enough — quit from the system tray/menu
   bar icon):
   ```json
   {
     "mcpServers": {
       "mneme": {
         "command": "/absolute/path/to/mneme-mcp"
       }
     }
   }
   ```
   If you already have other keys in that file (some builds also store UI
   preferences there), add `"mcpServers"` alongside them — don't replace the
   whole file. Open the config from **Settings → Developer → Edit Config**
   inside the app rather than hunting for the file yourself where possible.
   On Windows, if that button doesn't find it, Claude Desktop's newer builds
   are installed as an MSIX package and the real file lives at
   `%LOCALAPPDATA%\Packages\<Claude package folder>\LocalCache\Roaming\Claude\claude_desktop_config.json`
   — not the plain `%APPDATA%\Claude\` path older docs mention.
3. Run Mneme's desktop app as usual — `mneme-mcp` reads/writes the same
   database, so notes the assistant creates show up there immediately, and
   vice versa. The desktop app does not need to be running for `mneme-mcp`
   itself to work (SQLite doesn't need a "server" to be up), but you'll want
   it open to actually see what the assistant is doing.

**If a client reports 0 tools or can't connect:** `mneme-mcp` is spawned with
no visible terminal, so there's usually no error message anywhere in the
client UI. Check `~/.mneme/mneme-mcp.log` — every startup, every tool call,
and any fatal error (e.g. it couldn't open Mneme's database) is appended
there with a timestamp, which is otherwise the only way to see what actually
happened.

Since this connects over **stdio only** (no network port), it only works for
an AI client running on the same machine — this is a deliberate scope
decision, not a current limitation to be lifted later; see [Web
access](#features) above if you specifically want a different device to
reach a Well.

## Using the same Well on multiple devices

Mneme has no built-in sync engine, and that's deliberate — the original
project this was forked from tried a git-based sync feature and dropped it.
Hand-rolled sync of a mutable SQLite index alongside plain files is exactly
the kind of thing that turns a small bug into corrupted or lost notes, and
that risk isn't worth it when a much simpler option already works:

**Put your Well's folder inside whatever file-sync tool you already use** —
Dropbox, iCloud Drive, OneDrive, Syncthing, a self-hosted Nextcloud, etc. A
Well is just a folder of plain `.md` files; nothing about it requires being
in any particular location, and Mneme always reads a Well's files and search
results live off disk (no separate index that could go stale), so changes
synced in from another device show up as soon as you look. Add the same
folder as a Well on each device (pointing at wherever your sync tool puts it
locally) and you're done — the actual syncing is handled by tools with years
of production hardening behind them, not by Mneme.

(The ONE place Mneme does refuse cloud storage is `~/.mneme` — its own
private SQLite database, which is not inside your Well and which you'd never
want to sync anyway. That restriction has nothing to do with your notes.)

**The trade-off to know about:** if you edit the *same* note on two devices
before they've synced with each other, you'll get an ordinary sync
conflict from whichever tool you're using (e.g. a "conflicted copy" file from
Dropbox) — Mneme has no special merge logic for that, and neither does
Obsidian or any other plain-file notes app in this situation. In practice
this only comes up if you're actively editing the same file on two machines
at once, which is uncommon for a single-user notes vault.

## Build from source

### Prerequisites

- **Node.js ≥ 22** and **pnpm ≥ 9** (pinned in `package.json`).
- **Rust 1.92.0** (pinned via `rust-toolchain.toml`) plus the
  [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS.

### Develop

```sh
pnpm install
bash scripts/prepare-sidecar.sh --placeholder   # once: Tauri needs the mneme-mcp sidecar file to exist
pnpm --filter @mneme/desktop dev
```

### Build

```sh
bash scripts/prepare-sidecar.sh --host        # builds mneme-mcp and stages it for the installer
pnpm --filter @mneme/desktop build            # produces the installer(s) for your OS
```

### Test & lint

```sh
# Rust core (run `bash scripts/prepare-sidecar.sh --placeholder` once first):
cd apps/desktop/src-tauri && cargo test --workspace && cargo clippy --workspace --all-targets --all-features

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

## License

[MIT](LICENSE) © 2026 ecowangsa
