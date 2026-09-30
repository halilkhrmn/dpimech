# Contributing to DPIMech

Thanks for helping! Bug reports, strategy results from your network, translations and code are
all welcome.

## Ways to help without code

- **Report what works on your connection.** Run the Strategy Lab and open an issue with your
  country, provider (ISP), engine and the top result's arguments. That is how presets get better.
- **Translate.** The UI ships in English, Turkish and Russian. Catalogs live in
  `crates/gui/lang/<lang>/LC_MESSAGES/dpimech-gui.po` (edit with Poedit or any text editor).
  New language: copy `crates/gui/lang/dpimech-gui.pot` to `crates/gui/lang/<code>/LC_MESSAGES/dpimech-gui.po`,
  translate it and add the code to `LANGUAGES` in `crates/gui/src/i18n.rs`.
  After changing UI text, run `python tools/i18n.py` to update every catalog.
- **Report bugs** with the steps to reproduce, your Windows version, and the relevant lines from
  the Logs page.

## Building from source (Windows)

Requirements: [Rust](https://rustup.rs) (stable) and the Visual Studio C++ Build Tools
(`winget install Microsoft.VisualStudio.2022.BuildTools` with the "Desktop development with C++"
workload).

```sh
cargo build --workspace            # debug build
cargo test --workspace             # unit tests
cargo build --release --workspace  # release build → target/release/
```

Two binaries are produced:

| Binary | What it is |
|---|---|
| `dpimech.exe` | The window + tray icon. Runs as the normal user. |
| `dpimech-service.exe` | The background service that downloads and runs engines. |

### Running while developing

Engines that use WinDivert need administrator rights, so they only work through the installed
service. ByeDPI and the Strategy Lab work without it:

```sh
# terminal 1 – a development service with its own data folder and pipe name
set DPIMECH_PIPE=dpimech-dev
cargo run -p dpimech-service -- run --data-dir .dev-data

# terminal 2 – the GUI talking to it
set DPIMECH_PIPE=dpimech-dev
cargo run -p dpimech-gui
```

Installing the real service (elevated shell): `target\release\dpimech-service.exe install`.
It copies itself to `C:\Program Files\dpimech`, locks down `C:\ProgramData\dpimech` and registers
an auto-start service. Run it again to upgrade; `... uninstall` removes it.

Useful tools in `tools/`: `ipc.ps1` sends raw requests to the service, `uia.ps1` clicks through
the GUI via UI Automation. Debug builds also accept `DPIMECH_DEBUG_PAGE=engines|logs|editor|picker`
to open straight on a page.

## Releasing

Releases are built by GitHub Actions (`.github/workflows/release.yml`). Bump `version` in the
root `Cargo.toml`, commit, then push a matching tag:

```sh
git tag v0.1.1
git push origin v0.1.1
```

The workflow checks that the tag matches `Cargo.toml`, builds the release binaries and the
installer, and publishes `dpimech-setup-<version>.exe`, a zip of both binaries and
`SHA256SUMS.txt` with generated release notes. Locally, `tools/build-installer.ps1` produces the
same installer in `target/installer/`.

## Project layout

See [AGENTS.md](AGENTS.md) for the architecture, a file map and conventions, and
[docs/](docs/) for the plan, progress log and design decisions.

## Pull requests

- Keep changes focused; one topic per PR.
- `cargo fmt --all`, `cargo clippy --workspace --all-targets` (no warnings) and
  `cargo test --workspace` must pass.
- Match the surrounding code: comments explain *why*, user-facing strings use `@tr(...)`.
- If you change behaviour, add a short entry to `docs/PROGRESS.md`. If you make a design choice,
  add it to `docs/DECISIONS.md`.

### Security rules (the service runs as SYSTEM)

- New engine options must be added to the allowlist in `crates/core/src/argpolicy.rs`. Options
  that write files, read arbitrary files, or expose a listener must be denied or restricted.
- Engines may only be started from the service's own data folder, never from a path sent by a
  client.
- Downloads must be verified against a SHA-256 digest.
- Use SIDs, not group names, in ACL or firewall code (names are localised).

If you find a security problem, please report it privately instead of opening a public issue.
