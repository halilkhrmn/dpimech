# AGENTS.md — DPIMech

Guide for anyone (human or AI agent) working on this repo. **Read this first, then `docs/PROGRESS.md`.**

DPIMech (formerly the working name "dpimngr"; repo: github.com/halilkhrmn/dpimech) is a lightweight, native, cross-platform manager for DPI-bypass engines
(ByeDPI, zapret, GoodbyeDPI, SpoofDPI). Windows first; Linux and macOS later.

## Project docs — keep them current

| File | What it holds | When to update |
|---|---|---|
| `AGENTS.md` | Rules, architecture, commands, file map | When structure, commands or conventions change |
| `docs/PLAN.md` | Product plan, feature scope, phases | When scope or phases change |
| `docs/PROGRESS.md` | Phase checklist + dated work log | **At the end of every work session** |
| `docs/DECISIONS.md` | Numbered decisions with reasons | Whenever a non-obvious choice is made |

Work-log entries: newest on top, `### YYYY-MM-DD — short title`, then bullets for *done*, *verified how*, *open/next*.

## Architecture

```
GUI (dpimech.exe, user)  ──IPC: line-delimited JSON──►  Service (dpimech-service.exe, SYSTEM/root)
Slint UI + tray                                          profiles · engine processes · logs · config
```

- **IPC transport:** named pipe `\\.\pipe\dpimech` (Windows), Unix socket `/run/dpimech/dpimech.sock` (Linux/macOS).
- **Protocol:** `crates/core/src/ipc/mod.rs` — `ClientFrame{id, request}` → `ServerFrame::Reply{id, result}`; server also pushes `ServerFrame::Event`.
- **Service data dir:** `%ProgramData%\dpimech` · `/var/lib/dpimech` · `/Library/Application Support/dpimech`
  (override with `--data-dir` or `DPIMECH_DATA_DIR`). Contains `config.toml`, `engines/<slug>/<binary>`, `logs/`.

## File map

```
crates/core/      shared library (no UI, no OS service code)
  src/model.rs      EngineKind, RoutingMode, Routing, Profile, ProfileStatus, LogLine
  src/config.rs     ServiceConfig (TOML, atomic save)
  src/paths.rs      data-dir layout
  src/packages.rs   downloadable packages catalog (repo, asset matching, extract rules, pinned hashes)
  src/catalog.rs    domain packs (+ probe hosts), standard strategies (strategies/default.json), online sources, ISP table
  src/lab.rs        Strategy Lab request/result types
  src/args.rs       quote-aware argument splitter
  src/argpolicy.rs  engine argument allowlist (getopt-aware; blocks file writes, arbitrary reads, listen overrides)
  src/ipc/          protocol types, framing, transport (pipe/socket), async Client
crates/service/   dpimech-service binary
  src/main.rs       CLI: run | install | uninstall | service
  src/supervisor.rs profile CRUD, engine process lifecycle, conflict detection, package guards
  src/packages.rs   GitHub download, SHA-256 check, extract, installed.json, driver MSI
  src/perapp.rs     shared ProxiFyre router for per-app profiles
  src/nfqueue.rs    Linux: nftables table that queues traffic to nfqws while it runs
  src/perapp_linux.rs  Linux per-app / whole-computer proxy routing (cgroups, nft redirect, SOCKS relay)
  src/job.rs        engines die with the service (Windows job object, Linux PR_SET_PDEATHSIG)
  src/health.rs     SOCKS5 health probe used by the restart watchdog
  src/dnscheck.rs   Lab: compares system DNS with 1.1.1.1 (DoH) to spot DNS blocking
  src/sni.rs        TLS SNI parser: the detailed log names the site of a redirected connection (Linux)
  src/foreign.rs    finds other DPI tools running outside <data>/engines (ForeignTools request)
  src/migrate.rs    one-time move from the old name dpimngr (data dir) during `install`
  src/acl.rs        locks down the data dir (Windows: SIDs, takeown + icacls; Unix: root-owned, no g/o write)
  src/firewall.rs   inbound rule for the installed ProxiFyre.exe (session 0 gets no prompt)
  src/defender.rs   opt-in Windows Security exclusion for the engine folder
  src/launch.rs     engine + strategy → command (placeholders, defaults, argument policy); used by profiles and the Lab
  src/lab.rs        Strategy Lab runner, online strategy lists, ISP lookup
  src/server.rs     IPC accept loop + request dispatch
  src/logs.rs       ring buffer + broadcast of user-visible log lines; debug lines only while a GUI
                    asked for the detailed log (SetDetailedLog)
  src/winsvc.rs     Windows SCM integration
  src/systemd.rs    Linux systemd install / uninstall (unit in /etc/systemd/system)
  src/launchd.rs    macOS launchd install / uninstall (LaunchDaemon, binary in /Library/PrivilegedHelperTools)
crates/gui/       dpimech binary (Slint)
  ui/app.slint      window, sidebar, page switching
  ui/pages.slint    Dashboard, Logs, Settings, placeholders
  ui/editor.slint   profile editor + app picker dialog
  ui/engines.slint  Engines page (install / update / remove)
  ui/components.slint  NavItem, ProfileCard, Badge, EmptyState, PageHeader
  ui/theme.slint    colours, icon glyphs
  src/main.rs       wiring callbacks, tray, close-to-tray
  src/bridge.rs     background tokio thread owning the IPC connection
  src/convert.rs    core model <-> Slint structs
  src/tray.rs       tray icon + menu with per-profile toggles
  src/hotkey.rs     global hotkey (Ctrl+Alt+D)
  src/autostart.rs  "start when I sign in" via HKCU Run (+ --minimized)
  src/labui.rs      Strategy Lab page state; ui/lab.slint
  src/wizard.rs     first-run setup wizard state machine; ui/wizard.slint
  src/shortcut.rs   profile shortcuts: composed icon (logo + app), .lnk / .desktop / .app per OS;
                    shortcuts.toml records them (replace on recreate, remove with the profile / "remove all")
  src/shortcutui.rs "Create shortcut" dialog state; ui/shortcut.slint
  src/launcher.rs   `dpimech --launch <id> [--open <app>]`: progress window, start profile, open app, start the
                    main app in the tray if it is not running; ui/launch.slint
  src/prefs.rs      per-user GUI prefs (easy mode, onboarded) + elevated service install
  src/selfupdate.rs in-app update: service downloads + verifies, Windows installer run silently by the
                    service, AppImage swapped by the GUI; banner "Restart and update"
  src/notify.rs     Windows notifications (own AppUserModelID), status-change detection
  src/single.rs     single GUI instance (Windows mutex + event, Unix socket)
  src/logfile.rs    daily log files (opt-in) and error snapshots written by the GUI (never by the service)
  src/report.rs     "Report a problem": report text, GitHub issue / mailto links; ui/report.slint
  src/i18n.rs       translations for Rust-built text + language choice (tr, ru, fa, ar) and the user's
                    country from the locale; lang/ holds the .po catalogs
  src/whatsnew.rs   "What's new" dialog after an update (ui/whatsnew.slint)
  assets/           logo-source.png (master) → logo.png (UI), dpimech.ico/.png (exe, notifications),
                    tray-32.rgba (tray; status dot drawn at runtime) — regenerate with tools/make_icon.py
  src/apps.rs       process + Start Menu discovery with icons (Win32)
  src/picker.rs     app picker logic; src/state.rs UI-thread state
tools/            ipc.ps1 (raw IPC requests), uia.ps1 (drive the GUI via UI Automation),
                  build-installer.ps1, build-linux-packages.sh, build-macos-app.sh, i18n.py, make_icon.py,
                  copr-webhook.sh (starts a COPR build; used by release.yml and copr.yml),
                  smoke-linux.sh / smoke-windows.ps1 (release smoke tests: install a package on a clean
                  system, check libraries, service and window; release.yml runs them before publishing)
installer/        dpimech.iss (Inno Setup)
packaging/linux/  .desktop entry, service-setup.sh (deb postinst / rpm %post), deb maintainer scripts;
                  package metadata lives in crates/gui/Cargo.toml ([package.metadata.deb / generate-rpm])
packaging/fedora/ dpimech.spec + make-srpm.sh (source RPM with vendored crates) for Fedora COPR;
                  .copr/Makefile is COPR's entry point (see docs/RELEASING.md)
.github/workflows ci.yml (fmt, clippy, tests), release.yml (new version on main or tag v* → GitHub release),
                  pages.yml (site/ → GitHub Pages), copr.yml (manual COPR build / webhook test)
strategies/       default.json: the standard strategies per engine; embedded in the app and fetched from
                  `main` by the service, so editing it reaches users without a release; domains.json: the
                  site packs (with the countries where each is blocked), handled the same way
changelog/        en.md + tr/ru/fa/ar.md: user-facing notes per version, shown once after an update
                  ("What's new", src/whatsnew.rs) and used as the GitHub release text
site/             landing page (plain HTML, no build step; reads the newest release via the GitHub API)
docs/             PLAN, PROGRESS, DECISIONS, RELEASING (release steps, COPR setup)
```

## Commands

Rust is installed via rustup at `%USERPROFILE%\.cargo\bin` (add to `PATH` in fresh shells).

```sh
cargo build --workspace                 # debug build
cargo test --workspace                  # unit tests
cargo build --release -p dpimech-gui    # release GUI (size/RAM checks)

# Development run (two terminals). Put engine binaries in .dev-data/engines/<slug>/
cargo run -p dpimech-service -- run --data-dir .dev-data
cargo run -p dpimech-gui

# Screenshots of a specific page (debug builds only)
DPIMECH_DEBUG_PAGE=engines|logs|editor|picker|shortcut  DPIMECH_DEBUG_PROFILE=<profile id>

# Talk to a running service directly
pwsh tools/ipc.ps1 -Requests '{"type":"list_packages"}' -ListenSeconds 2

# Real Windows service (elevated shell). `install` copies itself to
# C:\Program Files\dpimech, secures C:\ProgramData\dpimech, registers + starts the
# service (auto start, restart on failure); running it again upgrades in place.
target\release\dpimech-service.exe install
"C:\Program Files\dpimech\dpimech-service.exe" uninstall
# The dev service and the real one share the pipe name: only one can run at a time.

# Linux (root): installs to /usr/local/lib/dpimech and enables dpimech.service.
sudo target/release/dpimech-service install
sudo /usr/local/lib/dpimech/dpimech-service uninstall
# Dev run next to an installed service: DPIMECH_SOCKET=/tmp/dpimech.sock

# Linux packages into target/linux/ (.deb, .rpm, AppImage); the release workflow runs the same script.
cargo install cargo-deb cargo-generate-rpm   # once
./tools/build-linux-packages.sh

# Release smoke test of a package (root; the release workflow runs these in clean containers / on Windows)
sudo tools/smoke-linux.sh appimage|deb|rpm target/linux/<file>
pwsh tools/smoke-windows.ps1 target/installer/dpimech-setup-<version>.exe

# Fedora source RPM (what COPR builds): newest v* tag, or DPIMECH_REF=<ref>
sh packaging/fedora/make-srpm.sh target/srpm
```

## Conventions

- **Commits:** no AI/Claude attribution anywhere (no `Co-Authored-By` trailers, no "generated with" lines). Author is the repo owner.
- **Branches:** `main` is protected: work on a branch and open a pull request; never push to `main`.
  Merging a PR that raises the version publishes a release (docs/RELEASING.md).
- **Language:** code, comments, UI strings and docs in English. Every user-visible string is translatable:
  `@tr(...)` in Slint, `tr("...")` / `trf!("… {} …", x)` in Rust (`src/i18n.rs`; texts that arrive in variables go
  into `EXTRA_TEXTS`). After changing UI text run `python tools/i18n.py` and translate the new entries in
  `crates/gui/lang/{tr,ru}/LC_MESSAGES/dpimech-gui.po` (tests fail on missing placeholders or EXTRA_TEXTS).
- **Style:** `cargo fmt`, no warnings. Comments explain *why*, not *what*. Match the surrounding code.
- **Layering:** `core` must not depend on UI or OS-service crates. OS-specific code lives behind `cfg(...)` in the smallest possible module.
- **UI:** Slint with the `fluent` style and the **software renderer** (GPU renderer costs ~90 MB extra RAM, see DECISIONS #4). Use `Palette.*` / `Theme.*` colours, never hardcoded ones in pages. Icons come from `Icons` in `theme.slint`.
- **Model updates:** profile cards change in place (`set_row_data`) while the same profiles are listed, so an open card
  menu survives status updates; `changed item` in `ProfileCard` puts the switch back on the service's state. A new
  `VecModel` only when profiles are added, removed or reordered. Logs use an incremental `VecModel`.
- **Budget:** GUI release build ≤ 40 MB working set when visible; service ≤ 20 MB idle. Re-measure after UI-heavy changes.

## Security rules (the service runs as SYSTEM/root)

- Engines are launched **only** from `<data>/engines/<slug>/`, never from a path supplied by a client.
- The pipe DACL lets authenticated local users talk to the service. Therefore any request that ends up in a privileged process must be validated.
- Engine arguments go through `core::argpolicy` on save **and** at launch. Unknown options are rejected. Add every new engine's options there before enabling custom args for it (e.g. zapret `--debug=@file` writes files).
- The service binary lives in Program Files and the data dir is reset to SYSTEM/Admins-only write on every service start (`acl.rs`). Never launch anything from a user-writable path.
- Use SIDs, not group names, in ACL/firewall code: names are localised.
- Downloaded engines must be checksum-verified before being placed in the data dir.
