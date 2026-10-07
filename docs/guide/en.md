<p align="center"><b>English</b> · <a href="tr.md">Türkçe</a> · <a href="ru.md">Русский</a> · <a href="fa.md">فارسی</a> · <a href="ar.md">العربية</a></p>

# DPIMech — User guide

[← Back to the README](../../README.md)

## Windows

### Getting started

1. **Download and run** `dpimech-setup-<version>.exe` from
   [Releases](https://github.com/halilkhrmn/dpimech/releases). Windows asks for permission once;
   the background service then starts with the computer by itself.
2. **Open DPIMech** and choose **Just make it work**. Pick the sites (for example Discord), where
   it should work (only some apps, the whole computer, or a proxy) and press **Start**.
   DPIMech installs what it needs, tests settings on your connection and turns the best one on.
3. That's it. Later you can press **Quick setup** to add another profile, or switch off
   *Easy mode* in Settings to see engines, the Strategy Lab and logs.

Tip: in **Settings** you can make DPIMech start when you sign in, hidden in the tray.

### Engines on Windows

| Engine | What it is good at | Needs |
|---|---|---|
| **ByeDPI** | A local proxy. Simple and safe. Pair it with ProxiFyre to route only chosen apps. | Nothing extra |
| **ProxiFyre** *(helper)* | Sends only the apps you pick through ByeDPI. | **Windows Packet Filter** driver (installed from the Engines page; a restart may be needed) |
| **zapret (winws)** | Works on the whole computer, also for UDP (Discord voice, QUIC). The most powerful option. | Uses WinDivert (included). Runs through the DPIMech service with administrator rights. |
| **GoodbyeDPI** | The classic whole-computer tool. Simple, but no longer actively developed. | Uses WinDivert (included). Same as above. |

Only **one** WinDivert engine (zapret or GoodbyeDPI) can run at a time. Some antivirus programs
wrongly flag WinDivert; DPIMech downloads it only from the engines' official releases.

## Linux

### Engines on Linux

| Engine | Mode | Needs |
|---|---|---|
| **zapret (nfqws)** | Whole computer. DPIMech adds its own nftables rules while it runs and removes them afterwards. | `nftables`, kernel modules `nft_queue` and `nfnetlink_queue` (standard on most distributions) |
| **zapret (tpws)** | Whole computer, only some apps, or a local SOCKS proxy | Nothing extra (cgroup v2 for "only some apps") |
| **ByeDPI** | Only some apps, or a local SOCKS proxy | Nothing extra (cgroup v2 for "only some apps") |
| **SpoofDPI** | Only some apps, or a local SOCKS proxy. Can resolve names over HTTPS (DoH), which also gets past DNS blocking | Nothing extra |

"Only some apps" and tpws for the whole computer route TCP only; UDP (for example voice)
goes direct. An app's traffic is picked up about a second after it starts.

### Install

Download from [Releases](https://github.com/halilkhrmn/dpimech/releases):

| Your system | File | Install |
|---|---|---|
| Ubuntu, Debian, Mint, Pop!_OS | `dpimech_<version>_amd64.deb` | `sudo apt install ./dpimech_*_amd64.deb` |
| Fedora | nothing to download: the [COPR repository](https://copr.fedorainfracloud.org/coprs/halilkahraman/DPIMech/) | see below |
| Arch, CachyOS, EndeavourOS, Manjaro | `dpimech-bin-<version>-1-x86_64.pkg.tar.zst` | `sudo pacman -U ./dpimech-bin-*.pkg.tar.zst` |
| openSUSE, RHEL | `dpimech-<version>-1.x86_64.rpm` | `sudo dnf install ./dpimech-*.x86_64.rpm` |
| Any other distribution | `DPIMech-<version>-x86_64.AppImage` | `chmod +x DPIMech-*.AppImage`, then run it |

[![Copr build status](https://copr.fedorainfracloud.org/coprs/halilkahraman/DPIMech/package/dpimech/status_image/last_build.png)](https://copr.fedorainfracloud.org/coprs/halilkahraman/DPIMech/package/dpimech/)

**Fedora** — add the repository once, then updates come with the rest of the system (`sudo dnf upgrade`):

```sh
sudo dnf copr enable halilkahraman/DPIMech
sudo dnf install dpimech
```

**Arch, CachyOS, EndeavourOS, Manjaro** — there is an Arch package (`dpimech-bin`), but it is not on the AUR yet: the AUR
takes no new accounts at the moment. The same package is attached to every release instead: download
`dpimech-bin-<version>-1-x86_64.pkg.tar.zst` from [Releases](https://github.com/halilkhrmn/dpimech/releases/latest) and install it
(pacman brings the dependencies; DPIMech tells you when a new version is out):

```sh
sudo pacman -U ./dpimech-bin-*-x86_64.pkg.tar.zst
```

The packages (`.deb`, `.rpm`, COPR, Arch) also install and start the background service. With the AppImage, open
DPIMech and press **Install the service** once (it asks for your password). The AppImage
brings the tray library along. On GNOME the tray icon needs the *AppIndicator and KStatusNotifierItem Support*
extension (Ubuntu has it built in).

### Build from source

Needs [Rust](https://rustup.rs), `systemd`, and for building the window the GTK 3 and
AppIndicator development packages (on Debian/Ubuntu:
`sudo apt install build-essential libgtk-3-dev libayatana-appindicator3-dev libxkbcommon-x11-0`).

```sh
git clone https://github.com/halilkhrmn/dpimech && cd dpimech
cargo build --release
sudo target/release/dpimech-service install   # background service (systemd unit "dpimech")
target/release/dpimech                         # the window
```

The tray icon needs a desktop with StatusNotifier/AppIndicator support (on GNOME: the
*AppIndicator* extension). Remove the service with `sudo /usr/local/lib/dpimech/dpimech-service uninstall`.

## macOS (early preview)

Download `DPIMech-<version>.dmg` from [Releases](https://github.com/halilkhrmn/dpimech/releases)
and drag DPIMech into Applications. The app is not signed yet: on first start, right-click it and
choose **Open**. Then press **Install the service** once (it asks for your password).

On macOS DPIMech can only run a **local SOCKS proxy** (zapret's tpws) for now; you set
`127.0.0.1:<port>` in your browser or app. Whole-computer and per-app modes are planned. Built
and checked automatically on macOS, but not yet tested by hand on a Mac.

## By country

DPIMech guesses your country from the system's region setting and offers the sites widely reported
blocked there first, already selected in the setup wizard. You can always pick others or type any site.
The list lives in [`strategies/domains.json`](../../strategies/domains.json) and updates without a new version.

| Country | Preselected | Keep in mind |
|---|---|---|
| Turkey | Discord, Roblox, Wattpad, Imgur | Many blocks are also done with DNS: see "Still blocked?" below. |
| Russia | YouTube, Discord, Instagram, X, Facebook, LinkedIn, Signal, Viber | YouTube is slowed rather than cut; zapret usually works best. |
| Iran | YouTube, Instagram, X, Telegram, Facebook, Signal, Discord | Filtering also blocks IP addresses and, at times, the whole international internet; DPIMech cannot help then. The Telegram and WhatsApp *apps* connect to IP addresses directly, so only their websites benefit. |
| Kazakhstan | SoundCloud | Most blocks target news sites and VPNs, often by IP address. Add the sites you need under "Other sites". |
| Belarus | TikTok, independent media (Zerkalo, Nasha Niva, Svaboda, Belsat, …) | Some outlets change their address often; add the current one under "Other sites". |
| Egypt | independent media (Mada Masr, Zawia3, Cairo 24) | Voice and video calls in WhatsApp and similar apps are blocked differently and stay blocked. |

Everywhere else Discord comes first. Missing a site or a country? Open an issue or a pull request
against `strategies/domains.json`.

## Good to know

- **Games:** on Windows, games with strict anti-cheat (for example Valorant or FACEIT) may refuse
  to start or disconnect you while a whole-computer engine is on. Turn that profile off before
  playing, or unblock only the apps you need.
- **One DPI tool at a time:** if another bypass tool (GoodbyeDPI, zapret, ByeDPI Manager, …) runs
  next to DPIMech, connections break in strange ways. DPIMech warns you when it sees one.
- **Still blocked? Check your DNS.** Some providers (for example in Turkey) also block sites by
  giving a wrong address for their names; no engine can fix that. The Strategy Lab warns when your
  DNS answers differently from 1.1.1.1. Then set your DNS to `1.1.1.1` / `1.0.0.1` (or turn on
  "DNS over HTTPS" in your system or browser).
- **Logs:** Settings → *Log*. When a profile stops with an error, the recent log is saved to a file
  by itself. *Detailed log* also records every connection (site, address, how it ended) — turn it
  on to find out why something does not open, and attach the file when asking for help.

## Is it safe?

- The engines only change how your own requests look.
- The background service runs with administrator rights because some engines need it. It only
  starts engines from its own protected folder and rejects settings that could write files or
  read private files.
- Checking your internet provider is optional. It happens only when you press
  *Detect my provider*, and it sends your public IP address to `ipwho.is`.
