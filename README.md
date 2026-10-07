<p align="center"><b>English</b> · <a href="README.tr.md">Türkçe</a> · <a href="README.ru.md">Русский</a> · <a href="README.fa.md">فارسی</a> · <a href="README.ar.md">العربية</a></p>

<!-- ---------- Header ---------- -->
<div align="center">
  <img src="crates/gui/assets/logo-long-square.svg" width="260" alt="DPIMech">
  <p>Open the sites and apps your internet provider blocks (Discord, YouTube, …).<br>Free and open source, for Windows, Linux and macOS.</p>

  <img alt="License" src="https://img.shields.io/github/license/halilkhrmn/dpimech?color=397256&style=flat-square">
  <img alt="Release" src="https://img.shields.io/github/v/release/halilkhrmn/dpimech?color=397256&style=flat-square">
  <img alt="Downloads" src="https://img.shields.io/github/downloads/halilkhrmn/dpimech/total?color=397256&style=flat-square">
  <img alt="Last commit" src="https://img.shields.io/github/last-commit/halilkhrmn/dpimech?color=397256&style=flat-square">
  <img alt="Stars" src="https://img.shields.io/github/stars/halilkhrmn/dpimech?color=397256&style=flat-square">
</div>

> **DPIMech is not a VPN.** It runs well-known open-source tools that change how your traffic looks, so
> your provider's deep packet inspection (DPI) stops blocking it. Your traffic still goes straight to
> the site: ping stays the same, but your IP address is not hidden.

<!-- ---------- Features ---------- -->
## Features

- [x] One click on, one click off: a ready *profile* per site or app, for example Discord
- [x] Only the apps you choose, the whole computer, or a local proxy
- [x] The *Strategy Lab* finds the settings that work on your connection
- [x] Installs and updates the engines, each download checked against its checksum
- [x] Restarts a stuck engine by itself, in under a second
- [x] Desktop shortcuts that turn a profile on and open the app
- [x] English, Türkçe, Русский, فارسی, العربية
- [x] Light: about 30 MB of RAM for the window, 20 MB for the background service

<!-- ---------- Download ---------- -->
## Download

| System | Get it |
|---|---|
| **Windows 10 / 11** | `dpimech-setup-<version>.exe` from [Releases](https://github.com/halilkhrmn/dpimech/releases/latest) |
| **Ubuntu, Debian, Mint** | `.deb` from Releases → `sudo apt install ./dpimech_*_amd64.deb` |
| **Fedora** | `sudo dnf copr enable halilkahraman/DPIMech && sudo dnf install dpimech` |
| **Arch, CachyOS, Manjaro** | `.pkg.tar.zst` from Releases → `sudo pacman -U ./dpimech-bin-*.pkg.tar.zst` |
| **openSUSE, RHEL** | `.rpm` from Releases → `sudo dnf install ./dpimech-*.x86_64.rpm` |
| **Other Linux** | AppImage from Releases |
| **macOS** *(early preview)* | `.dmg` from Releases |

Then open DPIMech and choose **Just make it work**. Engines, countries, building from source and
troubleshooting are in the **[user guide](docs/guide/en.md)**.

<!-- ---------- Screenshots ---------- -->
## Screenshots

<p align="center">
  <img src="site/screenshots/windows-profiles.png" width="49%" alt="Profiles on Windows">
  <img src="site/screenshots/windows-editor.png" width="49%" alt="A Discord profile on Windows">
  <img src="site/screenshots/linux-profiles.png" width="49%" alt="Profiles on Linux">
  <img src="site/screenshots/linux-editor.png" width="49%" alt="The profile editor on Linux">
</p>

<!-- ---------- Contribution ---------- -->
## Feedback and contributions

***All contributions are welcome!***

- Bug reports and ideas: [Issues](https://github.com/halilkhrmn/dpimech/issues), or *Report a problem* in the app's settings.
- Code: fork the project and open a pull request; [CONTRIBUTING.md](CONTRIBUTING.md) explains how.
- Translations live in [`crates/gui/lang`](crates/gui/lang) as ordinary `.po` files.

## Credits

- **Logo** by [Kim De Vries](https://www.artstation.com/kdevries21), drawn by hand. No AI was used to make it.
- DPIMech only manages these projects; the real work is theirs:
  [ByeDPI](https://github.com/hufrea/byedpi) ·
  [zapret](https://github.com/bol-van/zapret) ·
  [GoodbyeDPI](https://github.com/ValdikSS/GoodbyeDPI) ·
  [WinDivert](https://github.com/basil00/WinDivert) ·
  [ProxiFyre](https://github.com/wiresock/proxifyre) ·
  [Windows Packet Filter](https://github.com/wiresock/ndisapi).
- Strategy lists: [ByeDPI Manager](https://github.com/romanvht/ByeDPIManager),
  [SplitWire-Turkey](https://github.com/cagritaskn/SplitWire-Turkey).
  Icons: [Fluent UI System Icons](https://github.com/microsoft/fluentui-system-icons). Built with [Slint](https://slint.dev).
- The code is developed with AI assistance (Claude) and every feature is tested by hand;
  [docs/PROGRESS.md](docs/PROGRESS.md) records what was tested and how.

## License

DPIMech is free software under the [GNU General Public License v3.0 or later](LICENSE): you can use,
study, share and change it. The engines it downloads keep their own licences.

<p align="center">Made with ❤️</p>
