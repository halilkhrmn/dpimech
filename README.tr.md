<p align="center"><a href="README.md">English</a> · <b>Türkçe</b> · <a href="README.ru.md">Русский</a> · <a href="README.fa.md">فارسی</a> · <a href="README.ar.md">العربية</a></p>

<!-- ---------- Başlık ---------- -->
<div align="center">
  <img src="crates/gui/assets/logo-long-square.svg" width="260" alt="DPIMech">
  <p>İnternet sağlayıcının engellediği siteleri ve uygulamaları (Discord, YouTube, …) aç.<br>Ücretsiz ve açık kaynak; Windows, Linux ve macOS için.</p>

  <img alt="Lisans" src="https://img.shields.io/github/license/halilkhrmn/dpimech?color=397256&style=flat-square">
  <img alt="Sürüm" src="https://img.shields.io/github/v/release/halilkhrmn/dpimech?color=397256&style=flat-square">
  <img alt="İndirme" src="https://img.shields.io/github/downloads/halilkhrmn/dpimech/total?color=397256&style=flat-square">
  <img alt="Son commit" src="https://img.shields.io/github/last-commit/halilkhrmn/dpimech?color=397256&style=flat-square">
  <img alt="Yıldız" src="https://img.shields.io/github/stars/halilkhrmn/dpimech?color=397256&style=flat-square">
</div>

> **DPIMech bir VPN değildir.** Bilinen, açık kaynaklı araçları çalıştırarak trafiğinin görünüşünü
> değiştirir; böylece sağlayıcının derin paket incelemesi (DPI) onu engelleyemez. Trafiğin yine
> doğrudan siteye gider: ping değişmez ama IP adresin gizlenmez.

<!-- ---------- Özellikler ---------- -->
## Özellikler

- [x] Tek tıkla aç, tek tıkla kapat: her site ya da uygulama için hazır bir *profil*, örneğin Discord
- [x] Sadece seçtiğin uygulamalar, tüm bilgisayar ya da yerel proxy
- [x] *Strateji Laboratuvarı* bağlantında çalışan ayarları bulur
- [x] Motorları kurar ve günceller; her indirme sağlama toplamıyla kontrol edilir
- [x] Takılan motoru bir saniyeden kısa sürede kendiliğinden yeniden başlatır
- [x] Profili açıp uygulamayı başlatan masaüstü kısayolları
- [x] English, Türkçe, Русский, فارسی, العربية
- [x] Hafif: pencere için yaklaşık 30 MB, arka plan servisi için 20 MB RAM

<!-- ---------- İndir ---------- -->
## İndir

| Sistem | Nasıl |
|---|---|
| **Windows 10 / 11** | [Releases](https://github.com/halilkhrmn/dpimech/releases/latest) sayfasından `dpimech-setup-<sürüm>.exe` |
| **Ubuntu, Debian, Mint** | Releases'tan `.deb` → `sudo apt install ./dpimech_*_amd64.deb` |
| **Fedora** | `sudo dnf copr enable halilkahraman/DPIMech && sudo dnf install dpimech` |
| **Arch, CachyOS, Manjaro** | Releases'tan `.pkg.tar.zst` → `sudo pacman -U ./dpimech-bin-*.pkg.tar.zst` |
| **openSUSE, RHEL** | Releases'tan `.rpm` → `sudo dnf install ./dpimech-*.x86_64.rpm` |
| **Diğer Linux** | Releases'tan AppImage |
| **macOS** *(erken önizleme)* | Releases'tan `.dmg` |

Sonra DPIMech'i aç ve **Sadece çalışsın** seçeneğini seç. Motorlar, ülkeler, kaynak koddan derleme ve
sorun giderme **[kullanım kılavuzunda](docs/guide/tr.md)**.

<!-- ---------- Ekran görüntüleri ---------- -->
## Ekran görüntüleri

<p align="center">
  <img src="site/screenshots/windows-profiles.png" width="49%" alt="Windows'ta profiller">
  <img src="site/screenshots/windows-editor.png" width="49%" alt="Windows'ta bir Discord profili">
  <img src="site/screenshots/linux-profiles.png" width="49%" alt="Linux'ta profiller">
  <img src="site/screenshots/linux-editor.png" width="49%" alt="Linux'ta profil düzenleyici">
</p>

<!-- ---------- Katkı ---------- -->
## Geri bildirim ve katkı

***Her katkıya açığız!***

- Hata bildirimi ve fikirler: [Issues](https://github.com/halilkhrmn/dpimech/issues) ya da uygulamanın ayarlarındaki *Sorun bildir*.
- Kod: projeyi fork'la ve pull request aç; [CONTRIBUTING.md](CONTRIBUTING.md) nasıl yapılacağını anlatıyor.
- Çeviriler [`crates/gui/lang`](crates/gui/lang) klasöründe, sıradan `.po` dosyaları olarak duruyor.

## Teşekkürler

- **Logo:** [Kim De Vries](https://www.artstation.com/kdevries21) elle çizdi. Logoda yapay zekâ kullanılmadı.
- DPIMech bu projeleri sadece yönetir; asıl işi onlar yapar:
  [ByeDPI](https://github.com/hufrea/byedpi) ·
  [zapret](https://github.com/bol-van/zapret) ·
  [GoodbyeDPI](https://github.com/ValdikSS/GoodbyeDPI) ·
  [WinDivert](https://github.com/basil00/WinDivert) ·
  [ProxiFyre](https://github.com/wiresock/proxifyre) ·
  [Windows Packet Filter](https://github.com/wiresock/ndisapi).
- Strateji listeleri: [ByeDPI Manager](https://github.com/romanvht/ByeDPIManager),
  [SplitWire-Turkey](https://github.com/cagritaskn/SplitWire-Turkey).
  Simgeler: [Fluent UI System Icons](https://github.com/microsoft/fluentui-system-icons). [Slint](https://slint.dev) ile yapıldı.
- Kod yapay zekâ desteğiyle (Claude) geliştiriliyor ve her özellik elle test ediliyor;
  neyin nasıl test edildiği [docs/PROGRESS.md](docs/PROGRESS.md) dosyasında yazıyor.

## Lisans

DPIMech, [GNU Genel Kamu Lisansı v3.0 veya sonrası](LICENSE) altında özgür yazılımdır: kullanabilir,
inceleyebilir, paylaşabilir ve değiştirebilirsin. İndirdiği motorlar kendi lisanslarını korur.

<p align="center">❤️ ile yapıldı</p>
