# DPIMech — Plan

Lightweight, native, cross-platform DPI bypass manager. Windows first, then Linux, then macOS.

## Decisions

| Topic | Decision |
|---|---|
| Language / UI | Rust + Slint (native rendering, Fluent style, ~15–30 MB RAM) |
| Platforms | Phase 1: Windows · Phase 2: Linux · Phase 3: macOS |
| Engines | ByeDPI, zapret (winws/nfqws/tpws), GoodbyeDPI, SpoofDPI; the UI only shows engines the current OS supports |
| Routing modes | Per-app, system-wide, local proxy only |
| Privileges | Privileged background service + unprivileged GUI talking over IPC |
| Updates | Notify → show changelog → one-click download/install (auto-install optional in settings) |
| Strategy test targets | Built-in domain packs (Discord, YouTube, …) + user domains |
| App picker | Running processes + installed apps (with icons, searchable) + manual name/path + browse |
| Profiles | Multiple profiles can run simultaneously, with conflict detection |
| Language | English only for now, but every string goes through `@tr()` (gettext) so more languages are drop-in |

## Engine support matrix

| Engine | Windows | Linux | macOS | Mode | Upstream |
|---|---|---|---|---|---|
| ByeDPI (`ciadpi`) | ✅ | ✅ | ✅ | SOCKS5 proxy | hufrea/byedpi |
| zapret `winws` | ✅ | – | – | system-wide (WinDivert) | bol-van/zapret-win-bundle |
| zapret `nfqws` | – | ✅ | – | system-wide (NFQUEUE) | bol-van/zapret |
| zapret `tpws` | – | ✅ | ✅ | transparent proxy / SOCKS | bol-van/zapret |
| GoodbyeDPI | ✅ | – | – | system-wide (WinDivert) | ValdikSS/GoodbyeDPI |
| SpoofDPI | – | ✅ | ✅ | HTTP proxy | xvzc/SpoofDPI |
| ProxiFyre (helper) | ✅ | – | – | per-app SOCKS redirect | wiresock/proxifyre |

## Architecture

```
┌──────────── GUI (user, Slint) ────────────┐        ┌──────── Service (SYSTEM/root) ────────┐
│ Dashboard · Profiles · Strategy Lab ·     │  IPC   │ Profile runner  · Engine supervisor    │
│ Engines · Logs · Settings · Tray          │◄──────►│ Routing (ProxiFyre / WinDivert / nft)  │
└───────────────────────────────────────────┘ JSON-  │ Downloader/updater · Strategy tester   │
                                              RPC    │ Log bus (stream to GUI)                │
                                                     └────────────────────────────────────────┘
```

- **IPC:** Windows named pipe, Linux/macOS Unix socket; line-delimited JSON-RPC with an event stream (status, logs, test progress).
- **Service:** Windows Service (installed once, one UAC prompt) · systemd unit · launchd daemon (`SMAppService`).
- The GUI can be closed completely; profiles keep running. Tray-only mode keeps RAM minimal.

### Cargo workspace (target layout — current layout is in AGENTS.md)

```
crates/
  core/        # shared types: Profile, Strategy, EngineKind, IPC protocol, config (TOML)
  engines/     # Engine trait + one module per engine (args builder, capabilities, health check)
  routing/     # per-app / system-wide / proxy backends, cfg(target_os) per platform
  updater/     # GitHub Releases client, checksum verify, extract, version store
  tester/      # strategy test runner + scoring
  service/     # daemon binary (Windows service / systemd / launchd)
  gui/         # Slint app + tray
assets/strategies/  # built-in strategy packs & domain packs (also fetched from our GitHub repo)
i18n/               # gettext .po files (en only initially)
```

### Engine trait (sketch)

```rust
trait Engine {
    fn kind(&self) -> EngineKind;
    fn supported_os(&self) -> &[Os];
    fn modes(&self) -> &[RoutingMode];              // Proxy / SystemWide / PerApp
    fn build_command(&self, s: &Strategy, ctx: &RunCtx) -> Command;
    fn conflicts_with(&self, other: &dyn Engine) -> bool; // e.g. winws vs GoodbyeDPI (both WinDivert)
}
```

## Features

### Profiles (the main screen)
- Each profile = engine + strategy + routing mode + (apps | domains) + options.
- Dashboard shows profile **cards** with a big toggle, status dot, engine badge, apps' icons, live traffic/uptime.
- "New profile" wizard: 1) what to unblock (pick a domain pack / apps) → 2) engine (only compatible ones, with recommendation) → 3) strategy (pick from lab results or list) → 4) routing mode.
- Conflict detection before start: two WinDivert engines on overlapping traffic, port collisions, same app in two per-app profiles → clear message + suggested fix.
- Global hotkey and tray menu to toggle each profile.

### Engines & updates
- Engines page: installed version, latest version, size, source link, "Update"/"Install"/"Remove".
- Downloads from GitHub Releases, verifies SHA-256 (from release assets or our manifest), extracts to `%ProgramData%\dpimech\engines\<engine>\<version>` and keeps the previous version for rollback.
- Background check interval configurable (default daily); toast notification + changelog.
- App self-update via the same mechanism.
- Note: WinDivert-based tools are often flagged by Defender — show a clear explanation and an optional "add exclusion" button.

### Strategy Lab (the auto-tester)
- Strategy lists per engine are pulled from our GitHub repo (`strategies/<engine>.json`, versioned), plus user-defined ones.
- User picks domain packs (Discord, YouTube, Instagram, Roblox, …) and/or custom domains.
- Proxy engines (ByeDPI, SpoofDPI, tpws): run N strategies **in parallel** on different local ports, probe each domain through the proxy (TLS handshake + HTTP status + timing).
- System-wide engines (winws, GoodbyeDPI, nfqws): run **sequentially**, restricted to the test domains via hostlist so the rest of the system is unaffected.
- Result shown in plain language: `✅ 8/8 sites · 120 ms avg` / `⚠️ 5/8 (YouTube failed)` / `❌`, sorted by score (success rate, then latency, then stability across repeats).
- Advanced drawer shows raw args + per-domain detail; "Save as profile" / "Apply to profile" in one click.
- Optional "quick check" of the active profile from the dashboard.

### Routing
- **Per-app:** Windows → ProxiFyre managed by the service (config generated from the profile). Linux → cgroup v2 + nftables mark → proxy. macOS → limited (phase 3 research).
- **System-wide:** WinDivert engines on Windows, nfqws + nftables on Linux, pf + tpws on macOS.
- **Local proxy only:** just expose `127.0.0.1:<port>` and show copy-ready settings.

### Settings
- Start with OS, start minimized, auto-start selected profiles, minimize to tray on close.
- Update check interval / auto-install.
- Log level, log retention; export diagnostics bundle.
- Theme: follow system / light / dark.

## Phases

1. **Foundation (Windows):** workspace, config, IPC, Windows service + installer, Slint shell with Fluent theme, tray, i18n plumbing.
2. **ByeDPI + ProxiFyre MVP:** engine download, profiles (single → multiple), app picker, logs. *(Replaces current ByeDPI Manager usage.)*
3. **zapret winws + GoodbyeDPI:** system-wide mode, conflict detection.
4. **Strategy Lab:** strategy repo format, parallel/sequential tester, scoring, save-to-profile; ISP detection and country/ISP presets (Türkiye first).
4b. **Setup wizard + Easy mode:** first launch asks "do you know what DPI/engines are?". Easy mode: pick (or auto-detect) ISP → automatically find which of the common sites are blocked → choose "only these apps / whole computer / proxy" → the wizard installs the needed engines in order, runs the Lab and creates a ready profile. Easy mode shows a reduced UI (big on/off, fewer menus); Advanced mode is today's UI. Switchable in Settings.
5. **Updates & polish:** update notifications, rollback, self-update, Defender guidance, MSI/installer.
6. **Linux:** systemd service, nfqws/tpws/ByeDPI/SpoofDPI, nftables & cgroup routing, AppImage/deb.
7. **macOS:** launchd daemon, tpws/ByeDPI/SpoofDPI, pf, signed .dmg. *(Foundation done; the rest is
   on hold by the owner's decision, 2026-09-30.)*

## Priorities (set 2026-09-30)

**Priority 1 — quick, visible work** (done 2026-09-30, see PROGRESS.md)
1. Rename to DPIMech, with migration of dpimngr 0.1.x installs.
2. Pointer cursor; "avg." ping label; refresh icon button; latency in the profile editor.
3. Wizard progress in percent with a live log panel; daily log files with a chosen folder.
4. About section and credits in Settings.
5. Game (anti-cheat) warning and a warning when another DPI tool is running.
6. README split per OS; "not a VPN"; "AI-assisted, tested by hand".

**Priority 2 — shortcuts and quick launch** (done 2026-09-30, see PROGRESS.md)
- Profile shortcut from the card menu / editor: desktop or menu entry, icon = DPIMech logo + the app's icon;
  it turns the profile on, then opens the app, with a small progress window.
- Recreating replaces the old shortcut; deleting a profile removes its shortcuts; Settings → "Remove all".

**Priority 3 — not decided yet.** Candidates, for the owner to pick from:
- ~~Discord on Linux~~ → works with ByeDPI since 0.2.4 (stricter checks, DECISIONS #41).
- ~~SpoofDPI~~ → Linux and macOS since 0.2.3 (DECISIONS #39).
- ~~Strategy repository~~ → `strategies/default.json` in this repository, fetched from `main` by the
  service (0.2.5, DECISIONS #42). Domain packs stay in the code for now.
- ~~App self-update~~ → since 0.2.3 (DECISIONS #40).
- macOS: whole computer (pf) and per-app routing, signing and notarization — on hold for now.

## Open items
- ~~Final app name~~ → **DPIMech** (2026-09-30; 0.1.x installs migrate automatically, see DECISIONS #28).
- ~~Where our strategy/domain-pack repo lives~~ → no separate repo: `strategies/default.json` here (DECISIONS #42).
- ~~License~~ → **GPL-3.0-or-later** (DECISIONS #33); engines are downloaded at run time, not bundled.
