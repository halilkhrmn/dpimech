<p align="center"><a href="README.md">English</a> · <b>Türkçe</b> · <a href="README.ru.md">Русский</a></p>

<p align="center"><img src="crates/gui/assets/logo.png" width="128" alt="DPIMech logosu"></p>

# DPIMech

**İnternet sağlayıcının engellediği siteleri ve uygulamaları (Discord, YouTube, …) aç.**

Bazı internet sağlayıcıları trafiğinin içine bakıp belirli siteleri engeller. Bu yönteme *DPI*
(derin paket incelemesi) denir. DPIMech, bu engeli aşmak için bilinen, açık kaynaklı küçük araçları
("motorlar") çalıştırır. Bu araçlar trafiğinin görünüşünü değiştirir ve engel işe yaramaz hâle gelir.

> **DPIMech bir VPN değildir.** Trafiğin yine doğrudan siteye gider, başka birinin sunucusundan
> geçmez. Hız ve ping değişmez; ama IP adresin gizlenmez ve başka bir yolla (örneğin IP adresiyle)
> engellenen siteler engelli kalır.

| | Durum |
|---|---|
| **Windows 10 / 11** | Hazır. Kurulum dosyası [Releases](https://github.com/halilkhrmn/dpimech/releases) sayfasında. |
| **Linux** | Önizleme: Releases sayfasında `.deb`, `.rpm` ve AppImage; Fedora için COPR (aşağıya bak). |
| **macOS** | Erken önizleme: Releases sayfasında `.dmg`, sadece yerel proxy (aşağıya bak). |

## Ne işe yarar?

- **Tek tıkla aç, tek tıkla kapat.** Her *profil* hazır bir ayardır, örneğin "Discord".
- **Sadece istediğin yerde.** Sadece seçtiğin uygulamalar, tüm bilgisayar ya da yerel proxy.
- **Sana uyan ayarı bulur.** *Strateji Laboratuvarı* onlarca ayarı gerçek sitelerde dener ve senin
  bağlantında hangilerinin çalıştığını gösterir. İnternet sağlayıcını tespit edip ona özel hazır
  ayarlar önerebilir (örneğin Türk Telekom ya da Superonline).
- **Motorları senin yerine kurar ve günceller.** Resmî GitHub sayfalarından indirir ve her dosyayı
  yayımlanan sağlama toplamıyla (checksum) doğrular.
- **Çalışır durumda tutar.** Bir motor takılırsa (örneğin birkaç saat sonra Discord'da ping aniden
  binlere çıkarsa) DPIMech onu bir saniyeden kısa sürede yeniden başlatır. Bağlantı kontrolü her
  profilde ortalama pingi gösterir.
- **Kısayollar.** Profile sağ tıkla (ya da ⋯) → *Kısayol oluştur*: masaüstünde ya da menüde bir simge;
  tıklayınca profili açar, ardından uygulamayı (örneğin Discord) başlatır.
- **Hafif:** Pencere yaklaşık 25–35 MB, arka plan servisi yaklaşık 20 MB RAM kullanır.

## Windows

### Başlarken

1. [Releases](https://github.com/halilkhrmn/dpimech/releases) sayfasından
   `dpimech-setup-<sürüm>.exe` dosyasını **indir ve çalıştır**. Windows bir kez izin ister; arka
   plan servisi ondan sonra bilgisayarla birlikte kendiliğinden başlar.
2. **DPIMech'i aç** ve **Just make it work** (Sadece çalışsın) seçeneğini seç. Siteleri (örneğin
   Discord) ve nerede çalışacağını (sadece bazı uygulamalar, tüm bilgisayar ya da proxy) seç,
   **Start**'a bas. DPIMech gerekenleri kurar, ayarları senin bağlantında dener ve en iyisini açar.
3. Bu kadar. Sonra başka bir profil eklemek için **Quick setup**'a basabilir ya da motorları,
   Strateji Laboratuvarı'nı ve kayıtları görmek için Ayarlar'dan *Easy mode*'u kapatabilirsin.

İpucu: **Ayarlar**'dan DPIMech'in oturum açınca tepside gizli başlamasını sağlayabilirsin.

### Windows'taki motorlar

| Motor | Neyde iyi? | Gerekenler |
|---|---|---|
| **ByeDPI** | Yerel proxy. Basit ve güvenli. Sadece seçtiğin uygulamaları geçirmek için ProxiFyre ile kullan. | Ek bir şey gerekmez |
| **ProxiFyre** *(yardımcı)* | Sadece seçtiğin uygulamaları ByeDPI'dan geçirir. | **Windows Packet Filter** sürücüsü (Motorlar sayfasından kurulur; yeniden başlatma gerekebilir) |
| **zapret (winws)** | Tüm bilgisayarda çalışır, UDP'de de (Discord sesli sohbet, QUIC). En güçlü seçenek. | WinDivert kullanır (pakette gelir). Yönetici yetkisiyle DPIMech servisi üzerinden çalışır. |
| **GoodbyeDPI** | Klasik, tüm bilgisayarda çalışan araç. Basit ama artık aktif geliştirilmiyor. | WinDivert kullanır (pakette gelir). Üsttekiyle aynı. |

Aynı anda sadece **bir** WinDivert motoru (zapret ya da GoodbyeDPI) çalışabilir. Bazı antivirüs
programları WinDivert'i yanlışlıkla zararlı sanar; DPIMech onu sadece motorların resmî
sürümlerinden indirir.

### dpimngr 0.1.x'ten yükseltme

DPIMech'in eski adı *dpimngr* idi. Yeni kurulum dosyasını çalıştırman yeterli: eski servisi
kaldırır, profillerini ve motorlarını taşır.

## Linux (önizleme)

Linux sürümü çalışıyor ama şimdiye kadar sadece bir geliştirme konteynerinde denendi, gerçek bir
masaüstünde henüz test edilmedi. Geri bildirimlerini bekliyoruz.

### Linux'taki motorlar

| Motor | Mod | Gerekenler |
|---|---|---|
| **zapret (nfqws)** | Tüm bilgisayar. DPIMech çalışırken kendi nftables kurallarını ekler, sonra kaldırır. | `nftables`, `nft_queue` ve `nfnetlink_queue` çekirdek modülleri (çoğu dağıtımda hazır gelir) |
| **zapret (tpws)** | Tüm bilgisayar, sadece bazı uygulamalar ya da yerel SOCKS proxy | Ek bir şey gerekmez ("sadece bazı uygulamalar" için cgroup v2) |
| **ByeDPI** | Sadece bazı uygulamalar ya da yerel SOCKS proxy | Ek bir şey gerekmez ("sadece bazı uygulamalar" için cgroup v2) |

"Sadece bazı uygulamalar" ve tüm bilgisayar için tpws sadece TCP trafiğini yönlendirir; UDP
(örneğin sesli sohbet) doğrudan gider. Bir uygulamanın trafiği, uygulama açıldıktan yaklaşık bir
saniye sonra yakalanır.

### Kurulum

[Releases](https://github.com/halilkhrmn/dpimech/releases) sayfasından indir:

| Sistemin | Dosya | Kurulum |
|---|---|---|
| Ubuntu, Debian, Mint, Pop!_OS | `dpimech_<sürüm>_amd64.deb` | `sudo apt install ./dpimech_*_amd64.deb` |
| Fedora | indirmeye gerek yok: [COPR deposu](https://copr.fedorainfracloud.org/coprs/halilkahraman/DPIMech/) | aşağıya bak |
| openSUSE, RHEL | `dpimech-<sürüm>-1.x86_64.rpm` | `sudo dnf install ./dpimech-*.x86_64.rpm` |
| Diğer tüm dağıtımlar | `DPIMech-<sürüm>-x86_64.AppImage` | `chmod +x DPIMech-*.AppImage`, sonra çalıştır |

**Fedora** — depoyu bir kez ekle, sonra güncellemeler sistemle birlikte gelir (`sudo dnf upgrade`):

```sh
sudo dnf copr enable halilkahraman/DPIMech
sudo dnf install dpimech
```

Paketler (`.deb`, `.rpm`, COPR) arka plan servisini de kurar ve başlatır. AppImage kullanıyorsan DPIMech'i açıp bir
kez **Install the service**'e bas (şifreni sorar).

### Kaynak koddan derleme

[Rust](https://rustup.rs), `systemd` ve pencereyi derlemek için GTK 3 ile AppIndicator geliştirme
paketleri gerekir (Debian/Ubuntu'da:
`sudo apt install build-essential libgtk-3-dev libxdo-dev libayatana-appindicator3-dev libxkbcommon-x11-0`).

```sh
git clone https://github.com/halilkhrmn/dpimech && cd dpimech
cargo build --release
sudo target/release/dpimech-service install   # arka plan servisi (systemd birimi "dpimech")
target/release/dpimech                         # pencere
```

Tepsi simgesi için masaüstünün StatusNotifier/AppIndicator desteği olmalı (GNOME'da: *AppIndicator*
eklentisi). Servisi kaldırmak için: `sudo /usr/local/lib/dpimech/dpimech-service uninstall`.

## macOS (erken önizleme)

[Releases](https://github.com/halilkhrmn/dpimech/releases) sayfasından `DPIMech-<sürüm>.dmg`
dosyasını indir ve DPIMech'i Uygulamalar klasörüne sürükle. Uygulama henüz imzalı değil: ilk
açılışta sağ tıklayıp **Aç**'ı seç. Sonra bir kez **Install the service**'e bas (şifreni sorar).

macOS'ta DPIMech şimdilik sadece **yerel SOCKS proxy** (zapret'in tpws aracı) çalıştırabiliyor;
tarayıcında ya da uygulamanda `127.0.0.1:<port>` ayarlarsın. Tüm bilgisayar ve uygulama bazlı
modlar planlanıyor. macOS'ta otomatik olarak derlenip kontrol ediliyor ama henüz bir Mac'te elle
test edilmedi.

## Bilmekte fayda var

- **Oyunlar:** Windows'ta sıkı hile koruması olan oyunlar (örneğin Valorant ya da FACEIT) tüm
  bilgisayar modu açıkken başlamayabilir ya da seni oyundan atabilir. Oynamadan önce o profili
  kapat ya da sadece ihtiyacın olan uygulamaları seç.
- **Aynı anda tek DPI aracı:** DPIMech'in yanında başka bir engel aşma aracı (GoodbyeDPI, zapret,
  ByeDPI Manager, …) çalışırsa bağlantılar tuhaf şekillerde bozulur. DPIMech böyle bir araç görünce
  seni uyarır.
- **Kayıtlar:** Ayarlar → *Log files* her gün için bir dosya kaydeder; yardım isterken işe yarar.

## Güvenli mi?

- Motorlar sadece kendi isteklerinin görünüşünü değiştirir.
- Arka plan servisi yönetici yetkisiyle çalışır, çünkü bazı motorlar bunu gerektirir. Motorları
  sadece kendi korumalı klasöründen başlatır; dosya yazabilecek ya da özel dosyaları okuyabilecek
  ayarları reddeder.
- İnternet sağlayıcını tespit etmek isteğe bağlıdır. Sadece *Detect my provider*'a bastığında
  olur ve genel IP adresini `ipwho.is` sitesine gönderir.

## Nasıl yapılıyor?

DPIMech yapay zekâ desteğiyle (Claude) geliştiriliyor. Özellikler "bitti" sayılmadan önce elle
test ediliyor; neyin nasıl test edildiği, henüz test edilmeyenler de dahil,
[docs/PROGRESS.md](docs/PROGRESS.md) dosyasında yazıyor.

## Teşekkürler

DPIMech bu projeleri sadece yönetir; asıl işi onlar yapar:
[ByeDPI](https://github.com/hufrea/byedpi) ·
[zapret](https://github.com/bol-van/zapret) ·
[GoodbyeDPI](https://github.com/ValdikSS/GoodbyeDPI) ·
[WinDivert](https://github.com/basil00/WinDivert) ·
[ProxiFyre](https://github.com/wiresock/proxifyre) ·
[Windows Packet Filter](https://github.com/wiresock/ndisapi).
Strateji Laboratuvarı strateji listelerini
[ByeDPI Manager](https://github.com/romanvht/ByeDPIManager) ve
[SplitWire-Turkey](https://github.com/cagritaskn/SplitWire-Turkey) projelerinden indirebilir.
Simgeler: [Fluent UI System Icons](https://github.com/microsoft/fluentui-system-icons) (MIT).
[Slint](https://slint.dev) ile yapıldı.

## Lisans

DPIMech, [GNU Genel Kamu Lisansı v3.0 veya sonrası](LICENSE) altında özgür yazılımdır: kullanabilir,
paylaşabilir ve değiştirebilirsin; bunun üzerine kurulup dağıtılan her şey de aynı koşullarla açık
kalmalıdır. İndirdiği motorlar kendi lisanslarını korur.

Yardım etmek ister misin? [CONTRIBUTING.md](CONTRIBUTING.md) dosyasına bak.
