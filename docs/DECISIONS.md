# Decisions

Numbered, never renumbered. Superseded decisions stay, marked **Superseded by #N**.

## 1. Rust + Slint for the app (2026-09-30)
Wanted: native feel, small footprint, cross-platform. Compared Tauri 2 (WebView2, 50–90 MB RAM when visible),
Qt 6/QML (30–60 MB, 40 MB+ distribution), Avalonia (60–120 MB). Slint renders natively, is small and has a Fluent style.

## 2. Privileged service + unprivileged GUI (2026-09-30)
System-wide engines (WinDivert, NFQUEUE, pf) need admin/root. A service avoids a UAC prompt on every launch,
lets profiles keep running when the GUI is closed, and maps cleanly to systemd/launchd for Linux/macOS.

## 3. Line-delimited JSON over named pipe / Unix socket (2026-09-30)
Human-readable, trivial to debug from PowerShell or `socat`, no extra dependencies. Volume is tiny.

## 4. Slint software renderer instead of femtovg/OpenGL (2026-09-30)
Measured on Windows 11, release build, dashboard visible:
femtovg → 127 MB working set / 196 MB private; software → 26 MB working set / 8 MB private.
The UI is simple enough that software rendering is smooth. Revisit only if animations stutter.

## 5. Update policy: notify, then one-click install (2026-09-30)
Engine and strategy updates show a notification with changelog. Auto-install is an opt-in setting.

## 6. English-only UI for now, i18n-ready (2026-09-30)
All Slint strings go through `@tr()` (gettext). Turkish/Russian can be added as `.po` files later.

## 7. Engine binaries are downloaded at runtime, not bundled (2026-09-30)
Keeps the installer small, lets engines update independently, and avoids mixing upstream licenses (MIT/GPL) into our distribution.

## 8. One shared ProxiFyre for all per-app profiles (2026-09-30)
ProxiFyre drives a single kernel filter, and its config already supports several rules (one per SOCKS endpoint).
Running one instance per profile would fight over the driver. The service regenerates `app-config.json` and
restarts ProxiFyre whenever the set of running per-app profiles changes; engine binaries are always in `excludes`.

## 9. Refuse unverified downloads (2026-09-30)
Packages are installed only if the GitHub release asset carries a `sha256:` digest and the download matches it.
No digest → no install. Files are extracted into a staging dir, zip entries with unsafe paths abort the install.

## 10. Windows job object for engine processes (2026-09-30)
`kill_on_drop` only runs on a clean shutdown. Engines are assigned to a job with KILL_ON_JOB_CLOSE so the kernel
terminates them if the service crashes or is killed. Verified: hard-killing the service now also ends ciadpi/ProxiFyre.

## 11. Store the executable name for per-app rules (2026-09-30)
ProxiFyre matches a bare name against the whole executable name and a value with a slash as a path substring.
The picker stores the exe stem (e.g. `Discord`) so rules survive app updates that change the install path;
Squirrel shortcuts (`Update.exe --processStart X.exe`) are resolved to the real executable.

## 12. Active health probe instead of trusting engine logs (2026-09-30)
ByeDPI's `pool is full` would be the natural stall signal, but its stderr is block-buffered (4 KB) when piped, so the
line arrives only after ~170 rejected connections, or never. The service therefore probes each ByeDPI port with a
SOCKS5 greeting every 20 s and restarts the engine after two failures; log markers remain a secondary signal.
The connection limit is also raised to 4096 by default (Windows build uses WSAPoll, no FD_SETSIZE cap).

## 13. Argument allowlist per engine (2026-09-30)
Profiles reach a SYSTEM process from any local user, so arguments are parsed like the engine's getopt and every option
is classified (allow / managed by dpimngr / inline-or-lists-dir file / stdout only). Unknown options are rejected rather
than passed through, so a new engine release cannot silently add an unsafe flag.

## 14. Service data dir ACL via takeown/icacls with SIDs (2026-09-30)
ProgramData lets users create files in new subfolders, which would let them plant `installed.json` and get code run as
SYSTEM. The service resets the tree (owner Administrators, no inherited or explicit user write) at install and on every
start. The built-in tools handle ownership privileges; SIDs avoid localisation issues (Turkish "Kullanıcılar").

## 15. The service manages ProxiFyre's firewall rule (2026-09-30)
ProxiFyre's redirected connections count as inbound. Interactive runs get a firewall prompt, the session-0 service does not,
so traffic silently failed. One named inbound rule scoped to the installed ProxiFyre.exe is (re)written before start.

## 16. zapret from the official releases, not a third-party bundle (2026-09-30)
Community bundles (e.g. zapret-discord-youtube) are popular, but winws runs as SYSTEM, so binaries come from
`bol-van/zapret` releases only. The zip ships every platform; the installer keeps just the Windows binaries and
`files/fake`. Ideas from community presets are adapted into our own strategy set using `{fake}` payloads.

## 17. Pinned SHA-256 for releases without GitHub digests (2026-09-30)
GoodbyeDPI 0.2.2 (2022) predates GitHub asset digests. Its hash is pinned in the catalog; any other asset without a
digest is still refused (DECISIONS #9 stands).

## 18. Online strategy lists are fetched, labelled and credited, never bundled (2026-09-30)
ByeDPI Manager's list is GPL-3.0 and SplitWire-Turkey's presets are MIT; both are downloaded at runtime when the user
presses "Update online lists" and cached in the data dir. The UI never presents a source as an engine: strategies are
labelled "dpimngr standard set", "Community list" or "Turkey ISP presets", and the origin repo is named.

## 19. ISP lookup is opt-in (2026-09-30)
Detecting the provider sends the public IP to a third party (`ipwho.is`), so it only happens when the user presses
"Detect my ISP". Known ISPs are matched by ASN, then by name, to highlight presets made for them.

## 20. Lab probes a reachable subset of each domain pack (2026-09-30)
Hostlists need apex domains such as `discordapp.net`, but several have no A record, so requesting them always fails
and made every strategy look worse (best was 9/10). Each pack now has separate `probes` that answer HTTPS on `/`.

## 21. Connection monitor learns each line's normal latency (2026-09-30)
A fixed threshold would be wrong for most users (Discord answers in 150 ms on one line and 600 ms on another). The monitor
uses the median of the last 10 healthy checks as "usual", calls a check bad when most sites fail or latency exceeds
max(3× usual, usual + 500 ms), confirms after 30 s, and only then acts. Two restarts in 30 minutes without improvement
stop the restarts and advise the Strategy Lab, so a strategy that stopped working does not cause a restart loop.

## 22. Own AppUserModelID for notifications (2026-09-30)
Unpackaged apps must borrow an identity or register one. Borrowing PowerShell's made toasts look like they came from
PowerShell. The GUI registers `dpimngr.app` under HKCU\Software\Classes\AppUserModelId (name + icon, no admin) and the
installer's Start menu shortcut carries the same ID.

## 23. Inno Setup for the installer, GitHub Actions for releases (2026-09-30)
Inno Setup is small, scriptable, preinstalled on GitHub's Windows runners and supports AppUserModelID shortcuts and
multilingual setup pages. Releases are created only from `v*` tags whose version matches Cargo.toml.

## 24. The Unix socket is open to all local users (2026-09-30)
Same trust model as the Windows pipe (authenticated users may connect): the socket is 0666 and every request is
validated as if it came from an unprivileged user. A `dpimngr` group would add an install step per user for little gain,
since the service already treats clients as untrusted. Binding refuses to replace a socket that still answers.

## 25. Linux engines are tied to the service twice (2026-09-30)
`PR_SET_PDEATHSIG(SIGKILL)` kills an engine when the service dies, including in `run` mode without systemd. It fires on
the death of the forking *thread*, which is fine because engines are spawned from tokio worker threads that live as long
as the runtime. Under systemd `KillMode=control-group` covers the same case independently.

## 26. nfqws gets its traffic from one service-owned nftables table (2026-09-30)
`inet dpimngr` exists only while an nfqws engine runs (profile or Lab test) and is removed at service start. Only the first
packets of a connection are queued (`ct original packets 1-6`), packets nfqws sent itself are skipped by its mark
`0x40000000`, and `queue ... bypass` lets traffic flow if nfqws is gone, so a crash never cuts the network. Ports come from
the strategy's `--filter-tcp/--filter-udp` (default 80,443 / 443) and are parsed as numbers before they reach the root
firewall script. The queue number and mark are managed options users cannot set.

## 27. Linux strategies reuse the winws set (2026-09-30)
nfqws and winws share the desync engine; only the WinDivert `--wf-*` filters differ, and on Linux nftables does that job.
`catalog::adapt_args` strips them, so one tested set serves both OSes instead of two copies drifting apart.

## 28. Renamed to DPIMech; 0.1.x installs migrate on upgrade (2026-09-30)
The working name "dpimngr" became **DPIMech**. Everything a user or the OS sees changed: executables (`dpimech`,
`dpimech-service`), service/unit `dpimech`, pipe `\\.\pipe\dpimech`, socket `/run/dpimech/dpimech.sock`, data dirs
`C:\ProgramData\dpimech` / `/var/lib/dpimech`, AppUserModelID `dpimech.app`, nftables table `inet dpimech`. `install`
removes the old `dpimngr` service, binary, firewall rule and nftables table and *moves* the old data dir (profiles and
verified engines survive; a custom `--data-dir` is left alone). The GUI copies old prefs and re-creates the autostart
entry. The installer keeps the same AppId, so Windows shows one app, but sets `UsePreviousAppDir=no` and deletes the old
Program Files folder and shortcuts. The GitHub repository keeps its name; GitHub redirects if it is ever renamed.

## 29. Linux ships as .deb, .rpm and AppImage; no Flatpak for now (2026-09-30)
deb and rpm install the service as a packaged systemd unit (`/usr/lib/dpimech`, unit text printed by
`dpimech-service unit`, so packages and `install` cannot drift) and remove a source install that would shadow it.
The AppImage carries both binaries; the window installs the service through pkexec after copying the service
binary out of the FUSE mount (root cannot read another user's FUSE mount). Packages are built on Ubuntu 22.04 so
they run on newer glibc. Flatpak is left out: its sandbox is built to keep apps away from exactly what DPIMech
needs (a root service, nftables, a socket in /run), so it would need `flatpak-spawn --host` — a sandbox escape
that also rules out Flathub.

## 30. Problem reports are reviewed by the user and sent by the user (2026-09-30)
The report (version, OS, service, engines, profiles, recent log) is shown in full before anything is sent, and
leaves only through the user's browser (GitHub issue) or mail program (`mailto:` to the support address). Links
are length-limited, so they carry the summary plus the newest log lines that fit; the full report is saved next
to the log files for attaching. DPIMech itself never uploads anything.

## 31. Linux per-app routing: cgroups + nftables + a relay in the service (2026-09-30)
nftables cannot match a process name, but it can match a socket's cgroup. The service moves the chosen apps into a
cgroup per profile and redirects their new TCP connections to a relay that forwards them through the engine's SOCKS
port (like ProxiFyre does on Windows), so the engines stay unmodified and the health probe and connection monitor keep
working. Processes are found by a 1 s /proc scan, so the very first connection of an app that finishes in under a
second can go direct; long-running apps (Discord, browsers) are caught at start. Only TCP is routed. The same relay
gives tpws a whole-computer mode by redirecting all TCP to the strategy's ports except the service's own cgroup.

## 32. macOS starts as local-proxy only (2026-09-30)
macOS has neither WinDivert nor NFQUEUE; a whole-computer mode needs `pf` redirection plus tpws, and per-app needs a
network extension. The first macOS build therefore offers the local SOCKS proxy (tpws, universal binary from zapret's
own release), a launchd service and an unsigned universal `.dmg`. The wizard hides options an OS cannot do.

## 33. Licence: GPL-3.0-or-later (2026-09-30)
- **Decision:** DPIMech is licensed under the GNU GPL v3.0 or later (`LICENSE`, `license` in the workspace manifest, rpm/deb metadata).
- **Why:** it keeps forks open, so nobody can repackage the app as a closed or paid "unblocker" (a real risk for this kind of tool).
  It also matches Slint's GPLv3 option, so the app does not depend on the terms of Slint's royalty-free licence. All Rust dependencies
  (MIT / Apache-2.0 / BSD / Zlib / MPL) are GPL-3 compatible.
- **Engines are not affected:** ByeDPI, zapret, GoodbyeDPI, ProxiFyre etc. are separate programs downloaded at run time and started as
  processes, not linked, so each keeps its own licence.

## 34. Profile shortcuts run a separate launcher process (2026-09-30)
- **Decision:** a shortcut runs `dpimech --launch <profile> [--open <app>]`. That process shows a small
  frameless window (bottom-right on Windows), starts the profile over IPC, waits until it runs (polling,
  60 s limit), opens the app, shows "… is on" and closes. It does not take the single-instance lock, so
  it works whether or not the main window / tray is running.
- **Why a window, not only a notification:** starting can take a few seconds and can fail; a window can
  show progress and offer "Open DPIMech" on errors, and it disappears by itself on success.
- **What gets opened:** the app's Start Menu shortcut / .desktop entry when there is one (survives app
  updates that move the .exe, e.g. Discord's versioned folders), otherwise the executable. The target is
  stored in the shortcut itself; nothing new is sent to the service, which stays unaware of shortcuts.
- **How the files are made:** Windows .lnk through PowerShell's WScript.Shell (values passed in
  environment variables, never inside the script), icon as a PNG-in-ICO in `%LOCALAPPDATA%\DPIMech\shortcuts`;
  Linux .desktop in the applications dir and/or the XDG desktop dir (marked trusted for GNOME); macOS a tiny
  .app bundle with a shell script and an .icns. Icon files carry a timestamp because shells cache icons by path.

## 35. Fedora through COPR, built from source (2026-09-30)
- **Decision:** Fedora users get DPIMech from the COPR repository `halilkahraman/DPIMech`
  (`dnf copr enable`), so updates arrive with `dnf upgrade`. COPR builds from source: `make_srpm`
  (`.copr/Makefile` → `packaging/fedora/make-srpm.sh`) packs the newest `v*` tag with every crate
  vendored, and the RPM build runs offline. The release workflow triggers COPR through its custom
  webhook (secret `COPR_WEBHOOK_URL`) only after a GitHub release is published.
- **Why:** COPR expects packages built from source, and a repository gives automatic updates, which a
  downloaded `.rpm` does not. Building the newest tag (not `main`) keeps COPR in step with releases even
  when someone presses Rebuild later.
- **Paths:** the Fedora package puts the service in `/usr/libexec/dpimech` (labelled `bin_t`, so SELinux
  starts it as an ordinary unconfined service); the `.deb` and the release `.rpm` keep `/usr/lib/dpimech`.
  The GUI accepts both when it offers to start a packaged service.
- **Trade-off:** the source RPM is ~120 MB because `cargo vendor` includes the Windows crates.

## 36. Logs: short by default, detailed on request, saved when something fails (2026-09-30)
- **Default log** says what matters for a support question: the strategy and port an engine started with,
  the Lab's result without a bypass and its best strategy (with the sites that failed), which app processes
  were routed (Linux) and a warning when none match, and a DNS comparison with 1.1.1.1.
- **Detailed log** (Settings, off by default) adds one line per connection on Linux per-app routing
  (site from the TLS SNI, address, bytes, how it ended) and one per tested strategy. It is a `Debug` level
  the service only emits after a GUI sent `SetDetailedLog`, so older GUIs never see the new level.
- **Files:** daily files are now opt-in; instead, when a profile stops with an error the GUI writes the
  recent log (last 500 lines) to `dpimech-error-<date>_<time>.log` in the log folder (10 kept). The service
  still never writes to a user-chosen path (see #26).

## 37. Shortcuts are tracked so they can be replaced and removed (2026-09-30)
- The GUI records every shortcut it made in `shortcuts.toml` (per profile). Creating a shortcut again for a
  profile replaces the old ones (even with another name or place); deleting a profile removes its
  shortcuts; Settings has "Remove all shortcuts".
- Removal only deletes files that really are ours (a .lnk / .desktop / .app whose command contains
  `--launch`), because the list is a user-writable file and must not be able to point at anything else.

## 38. Releases come from merged pull requests (2026-09-30)
- **Decision:** `main` is protected and changes only through pull requests. The Release workflow runs on
  every push to `main` and publishes only when Cargo.toml has a version without a tag; it creates the tag
  itself. Tags pushed by hand still publish; "Run workflow" is a dry run.
- **Why:** the earlier `release` / `release-check` branches existed only because tag pushes are refused from
  the cloud sessions; with PR-only `main`, "merge the version bump" is the natural release button and needs
  no extra branches.
- **Landing page:** the version is no longer written into `site/index.html`; a small script reads
  `releases/latest` from the GitHub API and offers the right file (Windows .exe, macOS .dmg, Fedora → COPR,
  Ubuntu/Debian .deb, other Linux AppImage). Without the API every link opens the releases page.

## 39. SpoofDPI as a SOCKS5 engine on Linux and macOS (2026-09-30)
- **Decision:** SpoofDPI 1.5 (`xvzc/SpoofDPI`) is installed from its GitHub releases like the other engines
  (tar.gz, GitHub SHA-256 digest) and started as `--app-mode socks5 --listen-addr 127.0.0.1:<port> --no-tui
  --clean`, so it fits the existing proxy routes (local proxy, Linux per-app) and the Strategy Lab.
- **Why:** its own DNS over HTTPS is a direct answer to DNS blocking. For that to work behind per-app routing,
  the Linux relay hands SpoofDPI the TLS server name instead of the address the app got (`Route::by_name`).
- **Limits:** no Windows builds exist for 1.x, so it is Linux/macOS only; its SOCKS5 mode is marked
  experimental upstream. `--config`, `--auto-configure-network` and the listen/mode options are refused by
  the argument policy; `--clean` keeps a planted `/etc/spoofdpi.toml` from being read as root.

## 40. In-app updates (2026-09-30)
- **Decision:** when a newer release exists, the GUI asks the service to download it (`PrepareAppUpdate`):
  the service fetches the file for this system into the admin-only `<data>/updates`, checks GitHub's SHA-256
  digest, and the GUI shows "DPIMech X is ready — Restart and update" (banner, Settings, notification).
  Windows: the service runs the installer with `/VERYSILENT` (no UAC prompt, it is already SYSTEM); a hidden
  PowerShell started by the GUI waits for the new version and opens the window again. AppImage: the GUI
  copies the file over its own AppImage and starts it once the old process has exited. deb/rpm/COPR: the
  package manager stays in charge; the app only says how.
- **Safety:** only versions newer than the running one, only files with a GitHub digest, the installer is
  hashed again right before it runs, and the client cannot name a file or URL.

## 41. A strategy "works" only when data flows, every time (2026-09-30)
- **Problem:** the Lab counted any HTTP response header as success and trusted a single round. DPI boxes
  often let the handshake through and reset the connection once data flows, or let one connection in and
  reset the next; the owner's Discord profile through ByeDPI failed although the Lab had picked a strategy.
- **Decision:** a site is open only when its page starts loading (the first 64 KB, or the whole page when
  shorter) — used by the Lab and the connection monitor alike. After the quick round the 5 best strategies get
  3 more rounds, one engine at a time; only one that never missed is `confirmed` and ranks above the rest.
  Profiles check their sites 4 s after start and log which ones failed right away.
- **Cost:** a Lab run takes up to ~5 × 3 rounds longer (seconds per candidate); the page download is capped
  so big sites do not slow the check down.

## 42. Standard strategies in a JSON file in this repository (2026-09-30)
- **Decision:** the standard set lives in `strategies/default.json`, embedded with `include_str!` and fetched by
  the service from `main` (raw.githubusercontent.com) with the daily update check and on "Update online lists". The
  owner preferred this to a separate `dpimech-strategies` repository: one repo, one review flow.
- **Safety:** the file only supplies names and argument strings; every strategy still goes through the argument
  policy at launch, and a file this build cannot parse (or with another `format`) is ignored. It is stored in
  the admin-only data folder, written atomically.
- **Not moved:** ISP presets stay in the code. Domain packs moved to `strategies/domains.json` later (#43).

## 43. Site packs in a JSON file, tagged with the countries that block them (2026-10-02)
- **Decision:** domain packs live in `strategies/domains.json`, fetched from `main` like the strategies (#42). Each pack
  may list `countries` (ISO codes; `*` = offered first everywhere) and translated `names`. The GUI takes the country
  from the system region (Windows: "Country or region"; Unix/macOS: the locale's region, else a language spoken
  mainly in one country: fa → IR, kk → KZ, be → BY), shows that country's packs first and preselects them in the
  wizard and the Lab. Same idea as the Android app's `CountryPreset`, with the same pack ids.
- **Why the locale, not the ISP lookup:** the lookup sends the IP address to a third party and is opt-in; the
  region setting is local and good enough for ordering a list. Nothing is hidden: every pack is always offered.
- **Lists stay short:** a country only lists sites widely reported blocked there (Freedom House, news reports).
  Packs whose site refuses automated requests (403 from a bot check) keep those hosts in `domains` but not in
  `probes`, or every strategy would look broken.
- **Chips wrap in Rust:** Slint has no wrapping layout, so the GUI splits the chips into rows from an estimate of
  each label's width and the width Slint reports for the area (`init` and `changed width`).

## 44. Persian and Arabic without mirroring the layout (2026-10-02)
- **Decision:** add `fa` and `ar` catalogs. Slint's software renderer shapes Arabic script and lays out each text
  right to left, but cannot mirror the whole layout; the window keeps its left-to-right arrangement. Screenshots
  in both languages read correctly, so this is acceptable until Slint supports layout mirroring.
- **READMEs:** `README.fa.md` and `README.ar.md` wrap the text in `<div dir="rtl">`; code blocks stay outside so
  commands keep their direction.

## 45. "What's new" from a changelog built into the app (2026-10-02)
- **Decision:** `changelog/<lang>.md` holds user-facing notes per version (`## <version>` sections). The first start
  of a new version shows the sections since the version that ran before (`last_version` in the GUI prefs; at most
  three), in the UI language with English as fallback. The GitHub release uses the English section as its text.
- **Why built in, not the GitHub release text:** works offline and in every UI language; the release text was a
  list of pull-request titles. `cargo test` fails while a language lacks the current version's section.

## 46. Android reads `domains.json` too (2026-10-02)
- **Decision:** the Android app (halilkhrmn/dpimech-android) uses `strategies/domains.json` as its site list,
  embedded and fetched from `main` like here, instead of a copy in its own code. Each pack may carry
  `android_packages`: Android apps that use the site, which the Android app routes through its tunnel when the
  pack is chosen. Desktop ignores the field (serde skips unknown fields), so no format change.
- **Why:** one list for both apps; a new blocked site reaches phones and computers without a release.

## 47. libxkbcommon-x11 is required, not bundled (2026-10-02)
- **Problem:** on a bare Arch the AppImage panicked at start: winit opens libxkbcommon-x11 for an X11 window, and on
  Arch it is a package of its own that GTK does not pull in.
- **Tried:** bundling the build machine's copy (Ubuntu 22.04). It crashed in `xkb_state_update_mask` on Arch:
  libxkbcommon-x11 uses libxkbcommon's internal structures, so it only works with the exact libxkbcommon it was
  built with, and the system's libxkbcommon is already loaded by GTK.
- **Decision:** no bundled copy. When there is an X11 display, no Wayland, and the library is missing, the GUI says
  what to install (stderr and a GTK dialog, translated) and exits. The AUR package depends on it; the Arch smoke
  test checks the message, installs the library and then checks the window.

## 48. Arch package on the release page; AUR on hold (2026-10-02)
- **Problem:** the AUR currently takes no new accounts, so `dpimech-bin` cannot be published there.
- **Decision:** the release workflow already builds `dpimech-bin` from the release `.deb` and installs it in a
  clean Arch; it now attaches that `.pkg.tar.zst` to the GitHub release. Arch-based users install it with
  `sudo pacman -U`; pacman pulls in the dependencies (including libxkbcommon-x11) and removes it cleanly.
- **Trade-off:** no automatic updates through `yay`/`pacman -Syu`; the in-app update check still tells users
  about a new version. The AUR push stays in `tools/aur-publish.sh` (`--push`) for when registration reopens.


## 49. Fewer visible settings (2026-10-05)
- **Problem:** the profile editor showed eleven settings at once, Easy mode included (it only hid the sidebar),
  and three of them were different ways of saying "restart the engine when it stops working".
- **Decision:** one "Keep it working" switch (auto-restart + a connection check every 5 minutes) next to name,
  engine, routing and "Start with service". Port, check interval and sites, slow action, preventive restart,
  TCP only and strategy arguments sit behind "Advanced settings", which Easy mode does not show (Easy mode also
  hides engine and routing; the wizard picks them). "Start minimized" is gone: starting at sign-in always starts
  in the tray, and old entries are rewritten once. The detailed-log switch moved to the Logs page, and the
  Shortcuts group only shows once there are shortcuts. The unused "Coming soon" page is removed.
- **Kept:** "Detect my ISP" stays a button (Lab and wizard): it sends the public IP to ipwho.is, so it should
  not happen without a click. The crash-only auto-restart without site checks is no longer a separate switch.

## 50. "Open with proxy": the app's own proxy switch instead of a packet driver (2026-10-05)
- **Problem:** a profile for one Chromium/Electron app (Discord) used ProxiFyre and its packet driver, although
  those apps take `--proxy-server` and can talk to the engine's SOCKS5 port directly. The driver is what clashes
  with anti-cheat, and macOS had no way to route a single app at all.
- **Decision:** a new routing mode `app_proxy { app, port }`. The service runs the engine exactly like "Local
  proxy"; the GUI of whoever switches the profile on (window, tray or shortcut) opens the app with
  `--proxy-server=socks5://127.0.0.1:<port>` once the engine runs. Squirrel installs (Discord, Slack…) start
  through `Update.exe --processStart <exe> --process-start-args`, so the profile survives app updates that move
  the exe into a new `app-<version>` folder. Profiles the service starts by itself (at boot) open nothing.
- **Security:** profiles are shared by every user, so user A could name a program for user B to run. The GUI only
  opens apps from Program Files, Windows or the user's own app folders (Windows), or files owned by root or the
  user that nobody else can change, checked on the real file and its folder (Linux, macOS).
- **Limits:** only DPIMech's own launch uses the engine (an app opened another way, or already running, does
  not); voice/UDP goes direct; the editor warns when the app does not look like Chromium/Electron. ProxiFyre
  stays the default for per-app profiles because it catches the app however it starts.
- **Never offered (owner's choice):** the wizard, the Strategy Lab and Easy mode never pick or suggest it; it is
  the last routing option in the editor, for people who choose it themselves.

## 51. Hand-drawn logo; short READMEs with a separate user guide (2026-10-07)
- **Logo:** a chameleon by Kim De Vries (https://www.artstation.com/kdevries21), drawn by hand, no AI. Three
  SVG masters in `crates/gui/assets/`: the mark (`logo.svg`) is the app icon everywhere (exe, installer, tray,
  notifications, packages, shortcuts, macOS); the one-line logo (`logo-long.svg`, its chameleon is the "D") is the
  window header; the stacked one (`logo-long-square.svg`) heads the READMEs. `tools/make_icon.py` renders every
  icon size straight from the SVG (sharper than scaling one image down). The UI uses PNGs from it: Slint is built
  without SVG support and the software renderer would rasterise them anyway. The website puts the mark on a white
  tile because its title bar is blue.
- **READMEs:** short, in the style of WallYou: logo, one line, badges, features, download table, screenshots,
  contributions, credits (the logo credit says plainly it was drawn by hand), license, "Made with ❤️". Everything
  that was in them (engines per OS, install details, countries, troubleshooting, safety) moved unchanged into
  `docs/guide/<lang>.md`, one per README language, so nothing was lost or left untranslated.
