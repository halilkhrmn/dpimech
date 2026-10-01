# Progress

Phases are defined in [PLAN.md](PLAN.md). Tick items as they land; add a dated log entry at the end of each session.

## Phase checklist

### Phase 1 — Foundation (Windows) · *mostly done*
- [x] Cargo workspace (`core`, `service`, `gui`)
- [x] Domain model: engines, routing modes, profiles, statuses (OS-aware engine/mode matrix)
- [x] Config persistence (TOML, atomic save)
- [x] IPC protocol + named-pipe transport with DACL + async client
- [x] Service: profile CRUD, engine process supervision, log bus, conflict detection (port / WinDivert / same app)
- [x] Service: Windows SCM `install` / `uninstall` / `service` (builds; **not yet tested elevated**)
- [x] GUI shell: sidebar, dashboard with profile cards, profile editor, logs, settings, placeholders
- [x] Tray icon (show / quit), close-to-tray
- [x] Software renderer; RAM measured (see DECISIONS #4)
- [x] Engines die with the service (Windows job object, DECISIONS #10)
- [x] Real Windows service: install/upgrade/uninstall tested elevated (Program Files copy, recovery actions)
- [x] Start with Windows / start minimized settings (GUI, HKCU Run)
- [x] Per-engine argument validation (`core::argpolicy`)
- [x] Data dir ACL hardening; firewall rule for ProxiFyre
- [ ] Verify SCM recovery by killing the service (needs an elevated shell)
- [x] App icon in the .exe resources (done in Phase 5)

### Phase 2 — ByeDPI + ProxiFyre MVP · *mostly done*
- [x] Package installer: ByeDPI, ProxiFyre, Windows Packet Filter driver from GitHub Releases, SHA-256 verified, versioned dirs
- [x] Engines page: install / update / remove, "what's new", update badge in the sidebar, daily update check
- [x] Per-app routing via one shared ProxiFyre (config generation, lifecycle, failure → profiles marked failed)
- [x] App picker: running processes + Start Menu apps with icons, search, manual name/path, browse
- [x] Tray menu: per-profile toggles, tooltip with running count
- [x] Global hotkey Ctrl+Alt+D (stop all / restore last set)
- [x] Guards: can't remove a package a running profile uses; reinstall of an in-use version keeps locked files
- [x] Stale "engine not installed" errors clear after install
- [x] **Engine health watchdog**: SOCKS probe + stall markers + crash restart with backoff + scheduled restart, `-c 4096` default for ByeDPI, "TCP only" per-app option, restart count on cards
- [ ] Install the driver from the Engines page as a real (elevated) service — only detection tested

### Phase 3 — zapret winws + GoodbyeDPI (system-wide) · *done*
- [x] Packages: official zapret (Windows binaries + fake payloads), GoodbyeDPI (pinned SHA-256)
- [x] winws / GoodbyeDPI argument allowlists
- [x] System-wide profiles with a domain list → `{hostlist}`; editor "Sites to unblock" with domain packs
- [x] Friendly "needs administrator rights" error in dev mode
- [x] Installed both through the SYSTEM service; winws and GoodbyeDPI profiles unblock discord.com directly
### Phase 4 — Strategy Lab · *done*
- [x] Baseline run, parallel ByeDPI runner (4 engines), sequential WinDivert runner, scoring
- [x] Standard set + online lists (Community list, Turkey ISP presets) with clear labels and origins
- [x] ISP detection (ipwho.is, opt-in) and ★ presets for the detected ISP
- [x] "Use" creates a prefilled profile
- [x] Lab runs for winws (15 strategies) and GoodbyeDPI (32) through the SYSTEM service
### Phase 4b — Setup wizard + Easy mode · *done for ByeDPI paths*
- [x] First-run question ("Just make it work" / "I know what I'm doing"), Easy mode switch + "Open wizard" in Settings (prefs in `%APPDATA%\dpimngr\gui.toml`)
- [x] Wizard: sites → where (apps / whole computer / proxy) → ISP (optional) → install engines in order → Lab → profile saved + started (autostart on, free port picked)
- [x] "Install the service" button (ShellExecute runas → one UAC prompt) in the wizard and Settings
- [x] Easy mode UI: sidebar only Profiles + Settings, "Set up something new" opens the wizard
- [x] Wizard "whole computer" path end to end on the installed build (winws Lab → system-wide profile, discord.com 200 directly)
### Phase 5 — Updates, installer, polish · *done (translations moved later)*
- [x] Connection monitor per profile (interval, restart/warn, loop protection, "Check now", latency on cards)
- [x] Windows notifications under dpimngr's own identity; tray icon colour by state
- [x] App icon embedded in both executables
- [x] Inno Setup installer (install/upgrade/uninstall), `tools/build-installer.ps1`
- [x] GitHub Actions: CI (fmt, clippy, tests) and tag-triggered releases with installer + zip + SHA256SUMS
- [x] dpimngr update check (GitHub releases) + notification; Windows Security exclusion button (opt-in)
- [x] Single GUI instance (second launch shows the running window)
- [x] Translations (tr, ru): 291 strings, bundled .po catalogs shared by Slint and Rust, language from the system or Settings
- [x] Tray right-click menu toggles — profiles switch on and off (owner's hand test, Windows 11, 0.2.x)
- [ ] Windows Packet Filter driver upgrade and Defender exclusion — skipped at the user's request
### Priority 1 (PLAN.md → Priorities) · *done*
- [x] DPIMech rename + migration · [x] UI polish (cursor, avg. ping, refresh icon, editor latency)
- [x] Wizard percent + live log, daily log files · [x] About + Thanks · [x] Anti-cheat and other-tool warnings
- [x] README per OS in en/tr/ru, "not a VPN", "AI-assisted, tested by hand"
- [x] Priority 2: profile shortcuts — card ⋯ / right-click menu and editor button, icon = logo + app icon,
  desktop / menu entries per OS, `dpimech --launch` progress window; replaced on recreate, removed with the
  profile, Settings → "Remove all shortcuts" (Linux tested in the container; Windows by the owner: the
  shortcut turns the profile on and opens the app)

### Diagnostics · *done, waiting for field logs*
- [x] Default log: strategy + port on start, Lab baseline / best strategy / failed sites, routed processes,
  "no process matches", DNS vs 1.1.1.1 warning
- [x] Detailed log (Settings, off by default): per-connection lines with the TLS site name (Linux), per-strategy
  Lab lines; daily log files opt-in; automatic error snapshot when a profile fails
- [x] Discord per-app on Linux resets connections (owner's report) — works with ByeDPI since 0.2.4's stricter checks

### SpoofDPI and in-app updates · *done (0.2.3)*
- [x] SpoofDPI 1.5 on Linux/macOS: package, argument policy, managed SOCKS5 launch, 8 built-in strategies,
  per-app routing by TLS name (DECISIONS #39)
- [x] In-app updates: background download + SHA-256 check by the service, "Restart and update"; Windows
  silent installer, AppImage swap, package managers left in charge (DECISIONS #40)
- [x] Windows silent update by hand (owner: 0.2.3 → 0.2.4 on Windows)

### Releases and distribution · *done*
- [x] GPL-3.0-or-later; repository `halilkhrmn/dpimech` (fresh history, commits under the owner's GitHub identity)
- [x] `main` protected by a ruleset (PRs only); merging a PR that raises the version publishes the release
  (Windows installer + zip, .deb, .rpm, AppImage, .dmg, SHA256SUMS) and creates the tag
- [x] Fedora: COPR `halilkahraman/DPIMech` built from source, started by the release through a webhook
- [x] Landing page on GitHub Pages; download box picks the newest release and the visitor's system

### Phase 6 — Linux · *in progress*
- [x] Service builds and runs on Linux; Unix socket open to local users (0666), second instance refused
- [x] Engines die with the service (`PR_SET_PDEATHSIG`; systemd `KillMode=control-group`), clean stop on SIGTERM
- [x] `install` / `uninstall` via systemd (binary in `/usr/local/lib/dpimngr`, unit in `/etc/systemd/system`)
- [x] Data dir hardening: root-owned, no group/other write, planted symlinks removed
- [x] `.tar.gz` packages (exec bits kept, setuid dropped, links skipped); `<stem>-<arch>` binary names
- [x] Real ByeDPI Linux release installed from GitHub (owner's desktop: "ByeDPI v0.17.3 installed")
- [x] systemd install on a real systemd machine (owner's desktop, 0.2.1 from the packages)
- [x] zapret on Linux: nfqws + tpws from the same package (per-CPU binaries, exec bits), argument allowlists
- [x] nfqws system-wide: `inet dpimngr` nftables table (queue 200, `bypass`, ports from `--filter-tcp/udp`) for as long as the engine runs; stale table removed at start
- [x] tpws as a local SOCKS proxy (127.0.0.1, managed port) with the SOCKS health probe
- [x] Strategy Lab for nfqws: standard winws set without the `--wf-*` filters, sequential runner with the queue rules
- [ ] nfqws with a real kernel queue (this container lacks `nft_queue`; rules only syntax-checked)
- [x] tpws system-wide and per-app (same relay; tpws standard set in the Strategy Lab)
- [x] Per-app routing (cgroup v2 + nftables + transparent→SOCKS relay in the service)
- [x] GUI on Linux: tray on its own GTK thread, single instance (socket in `$XDG_RUNTIME_DIR`), XDG autostart entry, `notify-send`, service install via `pkexec`, `xdg-open` for links; wizard hides per-app and uses nfqws for "whole computer"
- [x] Linux app discovery for per-app routing (`/proc` + `.desktop` files, Flatpak/Snap)
- [x] Packaging: .deb, .rpm, AppImage via `tools/build-linux-packages.sh`, built by the release workflow on Ubuntu 22.04 (Flatpak skipped, DECISIONS #29)
### Phase 7 — macOS · *foundation; the rest on hold (owner's decision, 2026-09-30)*
- [x] Service: launchd install/uninstall (`/Library/PrivilegedHelperTools`, LaunchDaemon, KeepAlive), socket in `/var/run/dpimech`, data dir hardening
- [x] Engines: zapret's universal `binaries/mac64/tpws` as a local SOCKS proxy (ByeDPI: if its release has a macOS build)
- [x] GUI: tray created inside the event loop, LaunchAgent autostart, osascript notifications and admin prompt, macOS folders for prefs/logs, AppleLanguages
- [x] `tools/build-macos-app.sh` (universal .app + .dmg, unsigned), macOS in CI and in the release workflow
- [ ] Hand test on a Mac; code signing / notarization (the .dmg is built by every release)
- [ ] Whole computer (pf + tpws) and per-app routing

## Work log

### 2026-10-01 (38) — 0.2.9: profile shortcuts bring up the tray
- **Owner's report:** opening a profile shortcut starts the profile and the app, but no main window or tray
  appears, so there is nothing to switch the profile off with.
- **Done:** the launcher starts `dpimech --minimized` (the main app, in the tray) when no main instance runs.
  `single::is_running()` checks without making a running instance show its window (Windows: OpenMutexW;
  Unix: connect without a byte). The main app is started from `$APPIMAGE`, not the launcher's AppImage mount,
  which goes away when the launcher exits ("Open DPIMech" on the error page had the same problem).
- **Verified (Linux, Xvfb):** shortcut with no main app: launcher window, then `dpimech --minimized` keeps
  running with no window; shortcut again: no second instance, window stays hidden. fmt, clippy (Linux +
  Windows), tests. Not run on Windows yet.
- **Owner's report:** right-click → "Create shortcut…" on a profile card did nothing (the editor button worked).
  Cause: every status event replaced the whole profile model, destroying the card whose menu was open; the
  menu's activation was lost and the click fell through to the new card (opens the editor). Reproduced under
  Xvfb by changing a profile's status while its menu was open.
- **Done:** `apply_profiles` updates rows in place when the same profiles are listed; `ProfileCard` resets its
  switch from `item.on` on every change (the reason for the old wholesale replacement).
- **Verified (Xvfb):** status change with the menu open, then "Create shortcut…" → dialog opens; switch on via
  click, off via the service → switch shows off; on/off clicks stay consistent.

### 2026-10-01 (37) — 0.2.8: AppImage starts without libxdo
- **Report (CachyOS, AppImage 0.2.7):** `error while loading shared libraries: libxdo.so.3`. The AppImage bundles
  no libraries, and tray-icon's default `libxdo` feature linked it (only for predefined Copy/Paste menu items,
  which the tray does not use).
- **Done:** tray-icon without default features (`gtk` only); libxdo dropped from rpm requires, Fedora spec, CI,
  release workflow and READMEs. The binary now needs only GTK3/glib/fontconfig. libayatana-appindicator is
  loaded at runtime and its binding panics when missing: the tray thread catches that, and without a tray
  closing the window quits and `--minimized` shows the window after 3 s instead of running invisibly.
  The AppImage bundles libayatana-appindicator3 + libayatana-indicator3, libayatana-ido3, libdbusmenu-glib/-gtk3
  (owner: the tray must always be there); the GUI loads them by path only when the system has none, so no
  LD_LIBRARY_PATH leaks into apps it starts. The tray-less fallback stays for systems where even that fails.
- **Verified (bundling):** with all five hidden from the system, the AppImage loads its own copies
  (/proc/<pid>/maps) and the tray comes up; with them installed it uses the system copies.
- **Verified:** built the AppImage, hid libxdo + appindicator from the system: window opens, close quits,
  `--minimized` shows the window; with the libraries back: close keeps it in the tray, `--minimized` stays hidden.
  `readelf -d`: no libxdo. fmt, clippy (Linux + Windows), tests.

### 2026-10-01 (36) — 0.2.7: Fedora build fixed, lockfile checked in CI
- **Problem:** COPR build 11060164 failed while making the source RPM: `cargo vendor --locked` refused the
  v0.2.6 tag, whose Cargo.lock still said 0.2.5 (the lockfile commit landed after PR #9 was merged and tagged).
- **Done:** version 0.2.7 (no app changes) so COPR builds a tag with a matching lockfile; CI runs clippy and
  tests with `--locked`, so a version bump without its Cargo.lock fails the pull request.
- **Verified:** `cargo vendor --locked` succeeds on this commit (the step that failed on COPR).
- **Next:** after the merge, check the COPR build for 0.2.7.

### 2026-10-01 (35) — Linux no longer "preview"
- **Done (owner's call: Linux works on their machines):** READMEs (en/tr/ru) and the site call Linux ready;
  the "only tested in a container" note and the "Upgrading from dpimngr 0.1.x" section are gone.
  macOS stays "early preview". The dpimngr migration code stays in the app.

### 2026-10-01 (34) — screenshots in the READMEs and on the site
- **Done:** four screenshots in `site/screenshots/` (Windows ones from the owner, Linux ones taken under Xvfb
  with a demo data dir), shown in README (en/tr/ru) under the status table and in a "Screenshots" section
  on the site. One copy: the READMEs link into `site/`, which is what Pages publishes.
- **Verified:** site rendered with Chromium at 1100 px and 375 px (no horizontal scroll).

### 2026-10-01 (33) — 0.2.6: shortcuts from older versions, driver update error
- **Owner's reports (Windows, 0.2.4/0.2.5):** Settings said "0" shortcuts while "Discord wDPI" was on the desktop
  (made with 0.2.1, before `shortcuts.toml` existed); the hint showed "(or  )" because the ⋯ glyph is not in
  the text font. Updating Windows Packet Filter from the Engines page failed with msiexec 1603, labelled
  "needs administrator rights?" (wrong: the service is SYSTEM), and the card then showed "Install" — the old
  driver was gone.
- **Done:** shortcuts are also found by scanning the desktop and menu folders (Windows known folders, so a
  OneDrive desktop counts; Linux desktop + applications; macOS Desktop + Applications) for ones whose command
  is `dpimech --launch <profile>`; they count, are replaced on recreate and removed with their profile.
  Hint text without the glyph. Driver install refused while per-app profiles run (ProxiFyre holds the
  driver); msiexec writes a verbose log to `<data>/logs/driver-install.log` and the error shows its first
  "Error NNNN." line; 1603 now says to restart Windows and press Install again.
- **Verified:** unit tests for reading the profile from .lnk/.desktop/.app commands and for the MSI log
  line; Windows cross clippy clean. Not verified on Windows yet: the scan, the driver reinstall.
- **Next (owner):** restart Windows, Engines → Windows Packet Filter → Install; per-app profiles need it.
### 2026-10-01 (33) — Landing page links to the Android version
- **Done:** `site/` gets a Desktop / Android switcher (same one on the Android page,
  <https://halilkhrmn.github.io/dpimech-android/>), an "On a phone?" note, an Android row in the platform
  table and a footer link. Android visitors get the main button pointed at the Android page.
- **Verified (container):** page rendered in Chromium at 1000 px and 390 px: no script errors, no sideways scroll.
- **Next:** the Android repository needs Pages set to "GitHub Actions" once, like this one.

### 2026-09-30 (32) — 0.2.5: standard strategies as a JSON file in the repository
- **Done:** the standard strategies moved from Rust tables to `strategies/default.json` (DECISIONS #42). The app
  embeds it and the service fetches the newest one from `main` with the daily update check and on "Update
  online lists" in the Lab; a file it cannot read (newer format, empty, blank entries) is ignored. The owner chose
  this over a separate strategy repository.
- **Verified (container):** service lists the 12 embedded ByeDPI strategies; a valid file in
  `<data>/strategies/default.json` replaces them (13, new one first); a file with an unknown format falls back
  to the embedded set; the daily check hits the raw URL (404 until this is merged). Unit tests: every engine
  has strategies, ByeDPI/tpws/SpoofDPI strategies pass the argument policy, bad files are rejected.
- **Next:** after the merge, the service log should say "standard strategies updated" only when the file on
  `main` differs from the stored copy.

### 2026-09-30 (31) — In-app update tested by the owner
- **Done:** PR #5 merged, v0.2.4 published (21:32 UTC).
- **Verified (owner):** in-app update from 0.2.3 to 0.2.4 on Windows worked ("Restart and update", silent
  installer run by the service).
- **Verified (owner):** Discord through ByeDPI works on the owner's Linux with 0.2.4.

### 2026-09-30 (30) — 0.2.4: stricter connection checks
- **Owner's report (Linux, 0.2.2):** Discord through ByeDPI stuck on start ("recv: Connection reset by peer")
  although the Lab had picked a strategy; zapret worked. Stricter checks, DECISIONS #41:
  a site counts as open only when its page starts loading (up to 64 KB read), not on response headers;
  the Lab gives its 5 best strategies 3 more rounds one at a time and only a strategy that never missed is
  "confirmed" (ranked first; the wizard warns when nothing was); a profile's sites are checked 4 s after
  start and a failure is logged at once with the sites that failed (card shows advice when most failed).
- **Verified (container):** real ByeDPI Lab, 8 strategies on discord.com + github.com: 5 candidates re-run,
  8/8 each, confirmed; profile with two unreachable check sites logged "only 1/3 site(s) open through ByeDPI
  — failed: …" 4 s after start. Unit tests for merging and candidate choice.
- **Note:** PR #4 was merged seconds before this was pushed, so 0.2.3 went out without it.

### 2026-09-30 (29) — 0.2.3: SpoofDPI, in-app updates, two fixes
- **Done:** SpoofDPI engine (DECISIONS #39) incl. relay option to connect by TLS name; in-app updates
  (DECISIONS #40); engine output stripped of terminal colour codes; Windows Security hint hidden off Windows
  (the owner saw it on Linux); fix: stopping and quickly starting a profile stopped the new run too (the old
  run's clean-up removed the new run's entry) — each run now has its own number.
- **Verified (container):** real SpoofDPI 1.5.4 as a local-proxy profile (traffic flows; its colour codes no
  longer reach the log), Strategy Lab ran its strategies (DoH is blocked in this sandbox, system DNS passed),
  quick stop+start keeps the profile running; update flow as 0.2.1 against the real v0.2.2 release: service
  downloaded `DPIMech-0.2.2-x86_64.AppImage`, SHA-256 equal to GitHub's digest, banner "DPIMech 0.2.2 is ready
  to install", "Restart and update" swapped the AppImage, the old window exited and exactly one new 0.2.2
  window started. Windows cross clippy clean; unit tests for SpoofDPI assets/policy/strategies, SOCKS
  requests by name, colour stripping.
- **Not verified:** the Windows silent install + relaunch (needs a real Windows and a newer release).
- **Owner's Discord log:** still from a pre-0.2.2 build (no "strategy:" / "best strategy" lines).

### 2026-09-30 (28) — Windows hand tests, plan brought up to date
- **Hand tests (owner, Windows 11):** profile shortcut works (profile on, then the app opens); tray right-click
  menu switches profiles on and off.
- **Decided:** macOS work beyond the foundation is on hold.
- **Docs:** PLAN.md — Priority 2 marked done with what it contains, Priority 3 turned into a list of
  candidates (Discord follow-up, SpoofDPI, strategy repo, self-update, macOS on hold), license settled.
- **Found:** SpoofDPI is in the engine model but has no package, so it cannot be installed or used.
- **Next:** Discord logs from 0.2.2; the owner picks Priority 3.

### 2026-09-30 (27) — Status after the switch to pull requests
- **Done:** PR #1 merged by the owner; the owner turned on the `main` ruleset and deleted the old `release`
  branch. Checklist brought up to date (shortcuts, diagnostics, releases and distribution, Linux items the
  owner's desktop has now covered).
- **Verified:** first push to `main` after the merge: Release ran only its `plan` job (0.2.2 already tagged, so
  builds and publish were skipped), Pages deployed the new landing page, CI green; ruleset `main` is active.
- **Hand tests so far:** Windows 11 — 0.2.1 installs and the service runs; Linux desktop — 0.2.1 installs
  and runs, ByeDPI downloads; Fedora COPR builds automatically on release.
- **Next:** Discord on Linux with 0.2.2's detailed log (DNS warning? which host resets?); profile shortcut
  on Windows; a first look on a Mac.

### 2026-09-30 (26) — PR-only main, releases on merge, landing page download box
- **Done:** Release workflow: a `plan` job publishes when `main` has an untagged version (or a tag is pushed),
  dry run via "Run workflow"; `release` / `release-check` branches removed. Landing page: marquee gone; a
  download box reads the newest release from the GitHub API and offers the file for the visitor's OS (plus
  "Other systems and versions"); the Windows/Linux/macOS cards link straight to the files; small flags
  (GB/TR/RU) next to the README languages. AGENTS.md: work on branches, PRs only.
- **Verified:** workflow YAML parses and jobs chain plan → builds → publish; page rendered in Chromium with
  Windows / macOS / Fedora / Linux / iPhone user agents against the live API (0.2.2): exe, dmg, COPR link,
  AppImage, generic button respectively.
- **Open → done:** the owner turned on the `main` ruleset and deleted the `release` branch (see (27)).

### 2026-09-30 (25) — 0.2.2: better logs, DNS check, shortcut housekeeping
- **Report (owner, Linux):** wizard set up Discord per-app with ByeDPI; engine logged "recv: Connection reset by
  peer" and Discord did not open. The routing worked (the reset comes from ByeDPI), but the log could not tell
  which host or why. Suspected cause in Turkey: DNS blocking (wrong address for Discord's names).
- **Done:** logs (DECISIONS #36): strategy/port on start, Lab baseline + best strategy + failed sites, routed
  processes and "no process matches", DNS vs 1.1.1.1 warning (`dnscheck.rs`), detailed log toggle with per-
  connection lines incl. SNI (`sni.rs`), engine handshake timeout (15 s) in the Linux relay; daily files
  opt-in, error snapshots automatic. Shortcuts (DECISIONS #37): dialog closes after Create, recreate
  replaces, profile delete removes, Settings → Remove all.
- **Verified (container):** detailed relay line `example.com (93.184.215.14:443): sent 597 B, received
  3222 B`; "now routed: curl (pid …)"; Lab lines (baseline 0/2, best 2/2 with strategy); DoH unreachable is
  reported in the detailed log; recreate removed an older differently named shortcut and left a foreign
  file alone; Remove all cleaned desktop file, icon and list; error snapshot contains the error line.
  Unit tests: SNI parser, DoH JSON, network comparison.
- **Next:** owner re-tests Discord with the detailed log on and sends the snapshot / DNS warning.

### 2026-09-30 (24) — 0.2.1: Windows service would not install
- **Bug (hand test on Windows 11):** after installing 0.2.0 over 0.1.x the service was never registered; "Install
  the service" asked for UAC and then nothing happened, so ISP detection and the engine list waited forever.
  Cause: the installer puts the files in `C:\Program Files\DPIMech`, `dpimech-service install` wants
  `C:\Program Files\dpimech` and compared the paths case-sensitively, so it tried to copy its own running exe
  onto itself and gave up after 20 tries (hidden window, no message).
- **Fixed:** same-file check via canonical paths (case-insensitive fallback); `install` writes
  `<data>/logs/install.log`; the GUI waits for the elevated installer (ShellExecuteEx / pkexec / osascript) and
  shows "installed" or the error from that log; update check and provider lookup no longer hang on
  "Checking…" / "Looking up…" without the service. Version 0.2.1.
- **Verified:** Windows cross clippy, Linux tests; hand test by the owner on Windows 11: 0.2.1 installs and
  the service runs.
- **Release chain verified:** pushing to `release` published v0.2.1 (all 7 files) and the COPR webhook started the
  Fedora build of 0.2.1 by itself (confirmed on COPR by the owner).
- **Hand test (owner, real Linux desktop):** 0.2.1 installed and ran without problems — the first test outside
  the development container.

### 2026-09-30 (23) — Releases without pushing tags
- **Done:** pushing to the `release` branch runs the release workflow, which creates the tag `v<version>`
  with the GitHub release (tag pushes are refused from cloud sessions); a version that already has a tag is
  refused. Tags pushed by hand still work. Published 0.2.0 this way.
- **Verified:** the run on `release` built all three systems and published v0.2.0 with every file; only the
  COPR webhook step failed (curl exit 3: malformed URL in the secret). Now `tools/copr-webhook.sh` trims
  whitespace, rejects placeholders / non-COPR URLs with a clear message and reports COPR's HTTP answer; the
  release step no longer fails the run; `copr.yml` runs it by hand to test the secret.
- **COPR:** the project was created as `halilkahraman/DPIMech` (COPR names are case-sensitive): all install
  commands and links use that name; the webhook is the *custom* one (`/webhooks/custom/…/<package>/`), not
  the GitHub one.
- **Also:** four Python wheels (47 MB) had been committed by mistake (a `pip download` in the repo root);
  removed from the whole history before the release, `*.whl` ignored.

### 2026-09-30 (22) — Fedora COPR
- **Done:** `packaging/fedora/dpimech.spec` (from source, vendored crates, service in /usr/libexec),
  `packaging/fedora/make-srpm.sh` + `.copr/Makefile` (COPR make_srpm, newest v* tag), release workflow
  triggers COPR via the `COPR_WEBHOOK_URL` secret, `docs/RELEASING.md` (release steps + one-time COPR setup),
  Fedora install instructions in the READMEs and on the landing page; GUI recognises /usr/libexec service.
  Commit author on the new repo fixed to the GitHub identity (noreply address) so commits link to the account.
- **Verified:** `make-srpm.sh` built `dpimech-0.2.0-1.src.rpm` (124 MB) in the container; `rpmbuild --rebuild`
  of it compiled everything offline from the vendored crates and produced `dpimech-0.2.0-1.x86_64.rpm` with the
  expected files (service in /usr/libexec, unit ExecStart pointing there, %post runs service-setup.sh). The
  container is Ubuntu, so Fedora's systemd macros were stubbed and BuildRequires not checked.
- **Open:** create the COPR project/package and the webhook secret (docs/RELEASING.md); first COPR build on
  real Fedora chroots; install test on Fedora (SELinux enforcing).

### 2026-09-30 (21) — Repo moved to halilkhrmn/dpimech
- **Done:** every link, `APP_REPO` (update check, issue links), installer, landing page and Cargo metadata point at
  `github.com/halilkhrmn/dpimech`. The new repo starts fresh: one commit with the 0.2.0 tree on top of its
  initial LICENSE commit (same GPL-3 text); the old history stays in the old repo.
- **Note:** 0.1.x installs look for updates in the old repo; once it is deleted they will not see new versions
  (a GitHub rename would have kept a redirect).
- **Open:** enable Pages (Settings → Pages → Source: GitHub Actions) and push tag `v0.2.0` from a local clone.

### 2026-09-30 (20) — Profile shortcuts
- **Done:** profile card ⋯ button + right-click menu (Edit, Create shortcut…), "Create shortcut…" in the editor;
  dialog with name, "then open" app, desktop / menu choice and a live icon preview (logo + app icon top-right);
  `dpimech --launch` progress window (DECISIONS #34); tr/ru translations.
- **Verified (Linux, Xvfb + dev service):** dialog found the apps and composed the icon from a hicolor theme icon;
  Create wrote both .desktop files with correct Exec quoting; running the shortcut started a stopped tpws
  profile, opened the app and closed with "tpws test is on"; unknown profile shows the error with
  "Open DPIMech"; right click and ⋯ open the menu, left click still opens the editor. Unit tests for icon
  composition, ICO/ICNS headers, Exec quoting, user-dirs parsing and argument parsing.
- **Not verified:** Windows .lnk creation and icon (PowerShell, PrivateExtractIconsW) and the macOS bundle —
  need hand tests on those systems.

### 2026-09-30 (19) — Licence and landing page
- **Done:** GPL-3.0-or-later (`LICENSE`, manifests, rpm/deb, installer ships LICENSE.txt, About line; DECISIONS #33);
  `site/index.html` landing page (deliberately 2000s look) deployed by `.github/workflows/pages.yml`.
- **Verified:** page rendered in Chromium at 1100 px and as a 375 px phone (no horizontal scroll); release dry run
  on `release-check` green on Windows, Linux and macOS after adding job timeouts.
- **Open:** enable Pages once (Settings → Pages → Source: GitHub Actions); push tag `v0.2.0` from a local clone
  (tag pushes are refused from the cloud session).

### 2026-09-30 (18) — v0.2.0
- **Done:** pinned `tray-icon` 0.24 (0.25 pulled a second `muda` next to Slint's and the macOS LTO link failed); version 0.2.0 in `Cargo.toml` and the installer; tag `v0.2.0`.
- **Verified:** release dry run on `release-check` built the Windows installer, the Linux deb/rpm/AppImage and the macOS .dmg.
- **Open:** hand tests on real Windows / Linux / macOS machines; licence choice (rpm has a placeholder); update `APP_REPO` after the repo rename.

### 2026-09-30 (17) — macOS foundation
- **Done:** `launchd.rs`, macOS socket path, zapret tpws package for macOS, wizard offers only what the OS supports (macOS: proxy via tpws), GUI macOS pieces (tray timing, LaunchAgent, notifications, admin prompt, folders, language), app/dmg build script, CI + release jobs.
- **Verified:** `cargo clippy --workspace --all-targets --target aarch64-apple-darwin -D warnings` clean in the Linux container (type-check only, no linking); Linux build and tests unchanged. The macOS CI job is the first real build.
- **Not verified:** anything at runtime on a Mac.

### 2026-09-30 (16) — Turkish and Russian
- **Done:** Slint bundled translations (`lang/<lang>/LC_MESSAGES/dpimech-gui.po`, no default context) plus `i18n.rs` for text built in Rust (status lines, wizard, Lab, notifications, tray, dialogs) reading the same catalogs; `trf!` with `{}` / `{n}` placeholders; `tools/i18n.py` extracts and merges; Settings → Language (Automatic / English / Türkçe / Русский) applies immediately and is saved; system language from `LANG`/`LC_*` or the Windows locale. Fixed while testing: the .po parser dropped a trailing escaped quote. Wizard "done" text no longer says "with Windows" on Linux.
- **Verified:** tests (placeholders equal in every entry, EXTRA_TEXTS translated, parser); GUI under `LANG=tr_TR.UTF-8` fully Turkish incl. Rust-built status ("Çalışıyor · az önce", "Uygulama bazlı"); switching to Русский in Settings changed the running window and saved `language = "ru"`; Russian sidebar label shortened and nav labels elide.
- **Not verified:** Windows locale detection (registry `LocaleName`), right-to-left or other languages.

### 2026-09-30 (15) — Linux per-app routing and tpws for the whole computer
- **Done:** `perapp_linux.rs`: apps (matched by executable name / comm) are moved into `dpimech/app-<port>` cgroups; an nftables `nat output` table sends their TCP to a relay in the service, which reads `SO_ORIGINAL_DST` and connects through the engine's SOCKS port. tpws "whole computer" = the same relay for all TCP to the strategy's ports, excluding the service's own cgroup (outside systemd the service moves itself and its engines into `dpimech/service`). Private/loopback addresses never redirected; TCP only. Supervisor: `engine_port` (fixed 10880 for a whole-computer proxy), `is_routed`, `takes_all_traffic`. Strategy Lab runs any proxy engine in parallel; tpws standard set (8). GUI: Linux app discovery (.desktop incl. Flatpak/Snap + running processes), wizard offers "only some apps" on Linux (needs only ByeDPI).
- **Verified (container, cgroup v2 hybrid, real nft NAT, SOCKS5 stand-in that really connects):** a process named `curl` moved into `dpimech/app-18090` and its connection reached the engine (`CONNECT 93.184.215.14:80`) with the reply coming back; a `wget` next to it stayed direct; tpws whole computer: `wget` went through the engine and back without looping; nfqws refused while tpws whole computer runs; tables and cgroups removed on stop. Lab with tpws: 4 strategies in parallel, 1/1 each. Picker lists .desktop apps and running processes; wizard shows "Only in some apps" with Discord suggested.
- **Not verified:** a real desktop with systemd (cgroup of a user session app, Flatpak apps), IPv6 path (no ::1 in the container), real tpws/ByeDPI binaries.

### 2026-09-30 (14) — Problem reports, Linux packages, Defender check mark
- **Done:** "Report a problem" (`report.rs`, `ui/report.slint`; Settings, wizard failure, tray) → prefilled GitHub issue or e-mail to the support address with the saved full report (DECISIONS #30). Linux packages: `.deb`/`.rpm` metadata in `crates/gui/Cargo.toml`, `packaging/linux/`, `dpimech-service unit`, AppImage-aware service install and autostart, `tools/build-linux-packages.sh`; release workflow builds Windows + Linux and publishes one SHA256SUMS; CI now also runs on Ubuntu 22.04. Windows Security exclusion: green check, then after 5 s the whole yellow hint fades away; the hint is not shown at all when the service reports the exclusion already exists (`DefenderStatus`, asked from Get-MpPreference, so a removed exclusion brings it back); errors stay, button disabled while working.
- **Verified:** report dialog with a typed note → GitHub URL (2 233 chars, note as title) and mailto (1 633 chars) decoded, full report file carries the note; `.deb` installed with apt in the container (old `/etc` unit + `/usr/local/lib/dpimech` removed), removed (data kept) and purged (data gone); `.rpm` contents, requires and scriptlets inspected; AppImage built and started (APPIMAGE/APPDIR set); Defender hint: check mark at 1 s, whole hint gone at 6 s and the engine list moved up (success path forced on Linux for the screenshots, then reverted). `DefenderStatus` on Windows not run yet. All checks pass (fmt, clippy for Linux and the Windows target, tests).
- **Not verified:** rpm install on Fedora, packages on a real desktop with systemd, pkexec from the AppImage, a real release run.
- **Open:** project licence (rpm carries `LicenseRef-not-yet-chosen`); Priority 2 details.

### 2026-09-30 (13) — Priority 1: polish, logs, About, warnings, README
- **Done:** embedded Fluent SVG icons (the Windows 11 icon font was missing on Windows 10/Linux); pointer cursor, refresh icon on cards, "avg." latency, last check in the editor; wizard one-bar progress in % with a live log panel, clearer failures (✕ on the failed step, advice per mode, real cause when every setting failed the same way); daily log files written by the GUI (`logfile.rs`, 14 days, folder in Settings); About ("not a VPN", source/issue links) and Thanks (+ Made with Slint); service-side scan for other DPI tools (`foreign.rs`, `ForeignTools` request) with a GUI banner and a profile-log warning; anti-cheat note (Windows) in the wizard and editor; READMEs rewritten per OS in three languages; priority list in PLAN.md.
- **Verified (Linux, Xvfb, stand-in engines):** screenshots of sidebar icons, card refresh button, editor "Last check: 0/1 sites answered", wizard 90 % with log panel and "The engine could not run on this computer: this kernel cannot queue packets…", Settings log folder and About/Thanks; log file `dpimech-2026-09-30.log` written; foreign scan: a compiled `tpws` in /tmp reported, the same binary under `<data>/engines` ignored, and a leftover `dpimngr-service` from an earlier test was found for real; anti-cheat note rendered (forced on for the screenshot). fmt/clippy/tests clean on Linux and for the Windows target.
- **Not verified:** everything Windows-only at runtime (Toolhelp scan as SYSTEM, notifications, installer migration).
- **Release note:** bump `version` in Cargo.toml before tagging (the `v0.1.2` run failed on the tag/version check).
- **Next:** Priority 2 (shortcuts and quick launch) once its details are written down; a Windows test pass of the rename migration; Linux per-app routing.

### 2026-09-30 (12) — New logo, rename to DPIMech with migration
- **Done:** octopus logo in the UI (window, sidebar, top bar, wizard) and in the tray with a status dot; installer icon; `ikon.jpg` removed. Renamed everything user- or OS-visible to DPIMech/`dpimech` (crates `dpimech-*`, env `DPIMECH_*`); migration in `install` (service `migrate.rs`, `winsvc.rs`, `systemd.rs`), GUI prefs/autostart migration, installer `[InstallDelete]` + `UsePreviousAppDir=no` (DECISIONS #28). Fixed: GUI prefs were not saved on Linux without `$XDG_CONFIG_HOME`; Linux notifications/autostart now have an icon (`~/.local/share/icons/hicolor/256x256/apps/dpimech.png`).
- **Verified:** Linux `install` over a fake 0.1.x install (old unit, binary, `/var/lib/dpimngr` with a profile and a user-owned file) → old service disabled and removed, data moved and re-owned by root, new unit enabled; a second `install` only upgrades. GUI with old prefs (`onboarded = true`) and an old `dpimngr.desktop --minimized` → no first-run question, `dpimech.desktop` with `--minimized`, "DPIMech" and the logo shown. Workspace fmt/clippy/tests clean on Linux; service and GUI clippy clean for the Windows target.
- **Not verified:** the Windows upgrade path (old service removal, ProgramData move, Inno `[InstallDelete]`) — needs a Windows machine with 0.1.0 installed.
- **Release note:** the `v0.1.2` release run failed because the tag did not match `Cargo.toml` (0.1.0); bump the version before tagging.
- **Noticed:** sidebar icons render as garbage on Linux (Windows icon font).

### 2026-09-30 (11) — GUI on Linux
- **Done:** `tray.rs` GTK thread (gtk::init + main loop there, profile lists sent over a channel; a missing tray no longer blocks the window), `single.rs` Unix socket, `autostart.rs` XDG entry with spec-compliant `Exec` quoting, `notify.rs` via `notify-send`, `prefs.rs` `pkexec dpimngr-service install`, `open_url` (explorer / open / xdg-open), wizard offers only what the OS supports.
- **Verified (Xvfb, fake engines):** before the fix the GUI panicked in GTK menu creation on Linux; now it opens, the wizard shows "The whole computer" (default) and "proxy" only; a second launch exits in 0.05 s and the first stays alone; clippy `-D warnings` clean for the GUI on Linux.
- **Not verified:** tray icon and notifications in a real desktop session (no D-Bus session in the container).

### 2026-09-30 (10) — zapret on Linux: nfqws + nftables, tpws proxy
- **Done:** `EngineKind::intercepts_packets` (WinDivert or NFQUEUE, one at a time), zapret package on Linux (`extract_rules(os, arch)`, `nfqws` main binary, `Packages::engine_path`, exec bits for the package's engines), argpolicy split into `ZAPRET_COMMON` + `WINWS_FILTERS` / `NFQWS_ONLY`, new `TPWS` table, `nfqueue.rs` (ruleset from the strategy's ports, guard removes the table), `launch::engine_rules` used by profiles and the Lab, `catalog::adapt_args` for nfqws. Linux routing modes trimmed to what works today (ByeDPI/tpws: local proxy; nfqws: system-wide).
- **Verified:** tests (argpolicy nfqws/tpws, port parsing refuses anything but ports, nfqws strategy adaptation, exec bits); clippy clean for Linux and for the Windows target (`cargo check`/`clippy --target x86_64-pc-windows-gnu` with a stub `windres`). Live with stand-in binaries: tpws profile launched as `--bind-addr=127.0.0.1 --port=18081 --socks <strategy>` and passed the SOCKS probe for 50 s without restarts; nfqws profile wrote the hostlist, failed cleanly with "this kernel cannot queue packets…" (no `nft_queue` here) and left no table; `--qnum` and per-app tpws refused on save.
- **Open:** a real Linux box with GitHub access: install zapret, check `nfqws --help`/`tpws --help` against the tables, run the nfqws Lab.
- **Next:** GUI on Linux (tray needs GTK; Win32-only modules), then per-app routing.

### 2026-09-30 (9) — Phase 6 start: Linux service foundation
- **Done:** `job::contain` (PR_SET_PDEATHSIG, applied in `launch::build`), Unix socket mode 0666 + refuse to steal a live socket, `systemd.rs` (install/uninstall/service, hardened unit), `acl.rs` Unix hardening, SIGTERM shutdown, tar.gz extraction with safe modes, `find_binary` accepts `ciadpi-x86_64`. Fixed the `defender.rs` unused-import warning on non-Windows.
- **Verified (Linux container, root, no systemd, no GitHub):**
  - Tests: tar keeps 0755 / drops setuid, skips symlinks, rejects `../`, applies extract rules; unit file quoting. clippy clean.
  - Live with a Python stand-in for ciadpi: profile started over the socket (`-i 127.0.0.1 -p 18080 -c 4096 -s 1`); `kill -9` of the service → engine gone; SIGTERM → engine stopped and logged; second service instance → "another service is already listening".
  - `install` with a stand-in `systemctl`: binary 755 root, unit 644, stop → daemon-reload → enable → restart; data dir with a uid-1000 file, a 4777 file and a symlink to `/etc/shadow` → all root:root, 755/644, link removed. `uninstall` removes unit and binary.
- **Not done:** real downloads (GitHub blocked by the container's network policy), real systemd.
- **Next:** check ByeDPI's Linux asset/binary names on a machine with GitHub; zapret nfqws/tpws + nftables; then the GUI on Linux.

### 2026-09-30 (8) — Connection monitor, installer, releases, full test pass
- **Done:** connection monitor (`health.rs` monitor, `MonitorHistory`, `Restart::Slow`), notifications (`notify.rs`, own AppUserModelID `dpimngr.app`), tray colours, app icon (`tools/make_icon.py` → `crates/gui/assets`), installer (`installer/dpimngr.iss`), CI + release workflows, app update check, Defender exclusion endpoint, single-instance GUI (`single.rs`). READMEs now point to the installer and the wizard.
- **Verified:**
  - Monitor: healthy profile reported ~235–323 ms, learned "usual" after 2 checks; broken profile (no strategy) → 0/2 sites → confirmed after 30 s → restart ×2 → paused with "run the Strategy Lab" advice. Cards show latency / warning, "Check now" works, toasts "is slow" / "was restarted" shown, and they appear under "dpimngr" (Action Center history for `dpimngr.app`).
  - Installer (silent, elevated): service installed from Program Files, autostart profile started; killing the service ended the engine (job object) and SCM restarted the service within 12 s, which restarted the autostart profile.
  - Wizard "whole computer" on the installed build: ~70 s, winws profile 8/8 (166 ms), discord.com 200 directly.
  - Ctrl+Alt+D stops both running profiles and restores them.
  - Lab cancel: cancelled after 9/73, the four lab engines were killed immediately.
  - App update check: "You have the latest version (0.1.0)" (no public release yet).
  - Found and fixed: two GUI instances could run at once (two tray icons) → single-instance guard; second launch exits with 0.
- **Not done:** tray right-click menu (user was using the app), driver upgrade and Defender exclusion (skipped on request).
- **Next:** v0.1.0 release via tag; then Phase 6 (Linux) or more UX items from the suggestions list.

### 2026-09-30 (7) — Elevated verification, Easy mode layout, history cleanup
- **Done:** Easy mode has no sidebar (slim top bar + settings button); "Quick setup" (wizard) always next to "New profile". Lab names made unique when a source repeats a name. Remote history replaced with the rewritten local one (`--force-with-lease`, same content, no attribution trailers); `main` is in sync with `origin/main`.
- **Verified with the upgraded SYSTEM service:**
  - zapret v72.13 (Windows binaries + 31 fake payloads only) and GoodbyeDPI 0.2.2 (pinned hash) installed.
  - winws profile: hostlist loaded, WinDivert started, unrelated sites untouched. The first standard strategy was reset by Superonline's DPI (curl 35) — exactly what the Lab is for.
  - winws Lab (Superonline, Discord): baseline 3/8; **★ SuperOnline preset 8/8** (173 ms) and standard "fake + multisplit (md5sig)" 8/8; Türk Telekom/Vodafone presets 3/8 — the ISP match mattered.
  - GoodbyeDPI Lab: 8 of 32 strategies 8/8 (best "Alternatif 15", 168 ms; standard "-5" 172 ms).
  - Real profiles with the winners: discord.com 200 directly through winws (0.28 s) and GoodbyeDPI (0.25 s). Test profiles removed, no engines left running.
- **Next:** Phase 5 — single installer (GUI + service, Start menu shortcut, uninstall), update notifications/self-update, Defender guidance; translations (tr, ru).

### 2026-09-30 (6) — Setup wizard + Easy mode
- **Done:** `wizard.slint` + `wizard.rs` (event-driven state machine fed by package updates, strategy lists, Lab events and a new `SaveAndStart` command), `prefs.rs` (GUI prefs, elevated service install), Easy mode sidebar/dashboard/settings.
- **Verified (dev service, temp APPDATA so the real prefs were untouched):** welcome → "Just make it work" → Discord → only apps (Discord suggested automatically) → provider detected → Start: engines already present, Lab ran, best strategy picked, profile "Discord" saved and started on port 1081 (1080 was taken) in ~35 s; "All set!" reported 8/8, 169 ms. The test profile was deleted afterwards.
- **Next:** the elevated batch (upgrade the installed service; zapret/GoodbyeDPI profiles, Lab and the wizard's "whole computer" path); then Phase 5 (installer bundling GUI + service, update notifications).

### 2026-09-30 (5) — Phase 3 + Strategy Lab, repo docs
- **Done:** zapret/GoodbyeDPI packages and policies, system-wide domain lists, `launch.rs` shared by profiles and the Lab, Strategy Lab (service `lab.rs`, GUI `lab.slint`/`labui.rs`), ISP detection, online lists with honest labels, pack probes, `DPIMNGR_PIPE` for dev next to the installed service. README in English/Turkish/Russian, CONTRIBUTING.md; `CLAUDE.md` untracked (kept locally, ignored).
- **Verified (dev service, unelevated):** ISP detected as Superonline AS34984 TR; 72 ByeDPI strategies (12 standard + 60 community); baseline 3/8 Discord probes, 25 strategies 8/8 (fastest ~171 ms), run takes ~40 s; "Use" opens a prefilled per-app profile. Argument policy tests for all three engines pass.
- **Not verified yet:** anything that needs admin (winws/GoodbyeDPI profiles and Lab runs) — the UAC prompt for upgrading the installed service was cancelled; the installed service is still the previous build.
- **Next:** one elevated session: upgrade service, install zapret + GoodbyeDPI, run a winws and a GoodbyeDPI profile and a Lab run for each; then Phase 4b (wizard + Easy mode).

### 2026-09-30 (4) — Daily-driver readiness: real service, security, autostart
- **Done:**
  - `core::argpolicy`: ByeDPI option table parsed like getopt (unique long prefixes, `--opt=value`, clustered short flags). Blocks `--cache-dump <path>` (file write as SYSTEM), file forms of `-H/-j/-l` outside `<data>/lists` (arbitrary file read, fake-data would even send the file to the network), `-i/-p` (would expose the proxy on the LAN), unknown options and stray arguments. Checked on save and again at launch.
  - `acl.rs`: `takeown /A /R` + `icacls` with SIDs → SYSTEM/Admins full, Users RX, OWNER RIGHTS limited to read-control, inheritance from ProgramData removed, children reset. Applied at install and on every service start.
  - `install` copies the service to `C:\Program Files\dpimngr`, stops/upgrades an existing service, sets recovery (restart after 5 s ×3).
  - `firewall.rs`: inbound allow rule for the installed `ProxiFyre.exe`, rewritten when the path changes, removed with the package.
  - GUI Settings → Startup: "Start dpimngr when I sign in" / "Start minimized to the tray" (HKCU Run, `--minimized` keeps the window hidden).
- **Verified:**
  - Elevated `install` (UAC) → service Running/Automatic from Program Files; `icacls` shows the intended ACL; as the normal user, creating a file or folder in `C:\ProgramData\dpimngr` → access denied; unelevated IPC `hello` to the SYSTEM service works.
  - Engines installed into ProgramData by the service; per-app profile for `curl` under SYSTEM: first attempt timed out. Root cause: no firewall rule for the new ProxiFyre path and session 0 cannot show the allow prompt (rules existed only for paths the user had approved interactively). After adding `firewall.rs` and upgrading: `example.com` 200, `discord.com` 200 through the SYSTEM ProxiFyre.
  - Autostart toggles via UI Automation wrote/removed the Run value correctly (restored afterwards); `--minimized` launch keeps the main window hidden.
  - argpolicy unit tests (allowed strategies, dangerous forms incl. prefixes/clusters/`..` escape).
- **State left behind:** the real service is installed and running (ByeDPI + ProxiFyre installed in ProgramData, no profiles). The dev service is stopped (same pipe name).
- **Next:** Phase 3 (zapret winws / GoodbyeDPI system-wide) or Phase 4 (Strategy Lab). Packaging (single installer for GUI + service) is part of Phase 5.

### 2026-09-30 (3) — Engine watchdog (fix for Discord ping spikes)
- **Done:**
  - `Profile.reliability { auto_restart (default on), restart_every_hours }`, `Routing::PerApp.tcp_only`, `ProfileStatus::Running.restarts` (all `serde(default)`, old configs load unchanged).
  - `run_process` is now a restart loop: crash → restart with 1 s, 2 s, … backoff, give up after 5 crashes in 5 min; stall → kill + respawn; scheduled restart. ProxiFyre is not touched on restart (same port).
  - `health.rs`: every 20 s a SOCKS5 greeting to the engine port; 2 failures in a row → restart. 30 s grace after each start.
  - Engine output: identical lines are collapsed ("previous line repeated N×"); `pool is full` is also treated as a stall signal.
  - ByeDPI gets `-c 4096` unless the user sets `-c`/`--max-conn`.
  - Editor "Reliability" section; card shows "· N auto-restarts".
- **Verified:**
  - ciadpi command lines: user profile without `-c` → `... -p 1080 -c 4096`; test profile with `-c 8` kept as is.
  - Stall reproduced with `-c 8` + 20 slow concurrent downloads: health probe failed twice (`os error 10053`, ciadpi closes accepted sockets when full) → engine restarted after 28 s, new PID, `restarts=1`; afterwards `example.com` through the proxy → 200.
  - Found why log-based detection alone is unreliable: ciadpi's stderr is block-buffered (4 KB) when piped. Running it directly with a tiny pool and killing it produced 0 bytes of stderr; the user's ByeDPI Manager log shows the same 4 KB bursts ending in a truncated `pool is f`.
  - `cargo clippy --all-targets` clean; tests pass (arg splitter, versions, assets, `-c` detection).
- **Noticed:** the user's "test1" profile has empty strategy arguments, so it proxies without bypassing (Discord fails); needs e.g. `-r 1+s`.
- **Next:** elevated Windows service install test, Start with Windows, argument allowlist; then Phase 3 (zapret winws / GoodbyeDPI) or Phase 4 (Strategy Lab) depending on the user's priority.

### 2026-09-30 (2) — Phase 2: packages, per-app routing, picker, tray
- **Done:** `core::packages` catalog; service `packages.rs` (GitHub API, streamed download, digest check, zip-slip-safe extract, `installed.json`, keep previous version), `perapp.rs` (ProxiFyre router), `job.rs` (job object); GUI Engines page, app picker (`apps.rs`, `picker.rs`), tray profile toggles, hotkey, local-time logs, accessibility labels.
- **Verified:**
  - Installed ByeDPI v0.17.3 and ProxiFyre v2.6.1 over IPC; SHA-256 matched GitHub digests; driver 3.6.1.1 detected, v3.6.2 flagged as update.
  - Per-app profile routing `curl`: curl's TLS connection seen on ciadpi (`Get-NetTCPConnection`), `curl https://discord.com` → 200, while unrouted PowerShell timed out (Discord is DPI-blocked here) → routing is selective.
  - Toggled the profile from the GUI via UI Automation (`tools/uia.ps1`): ciadpi + ProxiFyre start, card shows Running; toggling off stops both.
  - Hard-killing the service now kills ciadpi and ProxiFyre (job object). Before the fix they were orphaned.
  - Remove guard: "ByeDPI is in use by: test1 — stop those profiles first".
  - Release: GUI 34 MB working set with the icon cache warm, service 20 MB; exes 12.6 MB / 2.9 MB.
- **Diagnosed user issue** (`bdmanager_27-09-2026.log`, ByeDPI Manager): every few hours Discord ping jumps to ~5000 ms until ByeDPI is restarted. The log shows a burst of `recv: 10054` (connection resets) followed by hundreds of `add_event: pool is full`. That message comes from ciadpi's `conev.c` when its event pool reaches `--max-conn` (default 512 events ≈ 256 proxied connections). New connections then stall until restart. Cause is ciadpi's connection limit plus dead connections piling up, not ByeDPI Manager.
- **Next (planned fix for the above):**
  1. Default `-c/--max-conn` to a higher value (e.g. 4096) for ByeDPI profiles, overridable.
  2. Watchdog in the supervisor: detect `pool is full` in engine output (and optionally a periodic SOCKS probe with latency threshold) → auto-restart only the engine (ProxiFyre keeps pointing at the same port, so the gap is sub-second). Rate-limit restarts and log them.
  3. Optional per-profile scheduled restart (every N hours) and a "TCP only" option for per-app routing to keep UDP flows out of ciadpi.
  4. Then: elevated service install test, Start-with-Windows, argument allowlist.
- **Dev notes:** `DPIMNGR_DEBUG_PAGE=engines|logs|editor|picker` (+ `DPIMNGR_DEBUG_PROFILE=<id>`) opens the debug GUI on a page for screenshots. `tools/ipc.ps1` sends raw IPC requests; `tools/uia.ps1` clicks/toggles via UI Automation.

### 2026-09-30 — Phase 1 scaffolding
- Installed Rust 1.98.1 (rustup) and VS 2022 C++ Build Tools via winget.
- Built the workspace: `core` (model, config, paths, IPC), `service` (supervisor, pipe server, Windows service glue), `gui` (Slint Fluent UI, tray).
- **Verified:**
  - Started the service with `.dev-data`, drove it over the named pipe from PowerShell: `hello` → `start_profile` → ByeDPI (`ciadpi.exe -i 127.0.0.1 -p 1090 -r 1+s`) listening; `curl --socks5-hostname 127.0.0.1:1090 https://discord.com` → HTTP 200 in 0.36 s; `stop_profile` → engine exits, status/log events are pushed.
  - GUI connects to the service and shows the profile card; screenshots checked.
  - Memory: GUI 26 MB working set (release, software renderer); service ~13 MB.
  - `cargo test -p dpimngr-core` passes (arg splitter).
- **Not yet verified:** toggling from the GUI and the profile editor save flow by clicking (only via IPC); Windows service install; tray behaviour.
- **Next:** test the service install elevated, then Phase 2 (engine downloader + ProxiFyre per-app routing).
