<p align="center"><a href="en.md">English</a> · <b>Türkçe</b> · <a href="ru.md">Русский</a> · <a href="fa.md">فارسی</a> · <a href="ar.md">العربية</a></p>

# DPIMech — Kullanım kılavuzu

[← README'ye dön](../../README.tr.md)

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

## Linux

### Linux'taki motorlar

| Motor | Mod | Gerekenler |
|---|---|---|
| **zapret (nfqws)** | Tüm bilgisayar. DPIMech çalışırken kendi nftables kurallarını ekler, sonra kaldırır. | `nftables`, `nft_queue` ve `nfnetlink_queue` çekirdek modülleri (çoğu dağıtımda hazır gelir) |
| **zapret (tpws)** | Tüm bilgisayar, sadece bazı uygulamalar ya da yerel SOCKS proxy | Ek bir şey gerekmez ("sadece bazı uygulamalar" için cgroup v2) |
| **ByeDPI** | Sadece bazı uygulamalar ya da yerel SOCKS proxy | Ek bir şey gerekmez ("sadece bazı uygulamalar" için cgroup v2) |
| **SpoofDPI** | Sadece bazı uygulamalar ya da yerel SOCKS proxy. Adları HTTPS üzerinden (DoH) çözebilir; bu, DNS engelini de aşar | Ek bir şey gerekmez |

"Sadece bazı uygulamalar" ve tüm bilgisayar için tpws sadece TCP trafiğini yönlendirir; UDP
(örneğin sesli sohbet) doğrudan gider. Bir uygulamanın trafiği, uygulama açıldıktan yaklaşık bir
saniye sonra yakalanır.

### Kurulum

[Releases](https://github.com/halilkhrmn/dpimech/releases) sayfasından indir:

| Sistemin | Dosya | Kurulum |
|---|---|---|
| Ubuntu, Debian, Mint, Pop!_OS | `dpimech_<sürüm>_amd64.deb` | `sudo apt install ./dpimech_*_amd64.deb` |
| Fedora | indirmeye gerek yok: [COPR deposu](https://copr.fedorainfracloud.org/coprs/halilkahraman/DPIMech/) | aşağıya bak |
| Arch, CachyOS, EndeavourOS, Manjaro | `dpimech-bin-<sürüm>-1-x86_64.pkg.tar.zst` | `sudo pacman -U ./dpimech-bin-*.pkg.tar.zst` |
| openSUSE, RHEL | `dpimech-<sürüm>-1.x86_64.rpm` | `sudo dnf install ./dpimech-*.x86_64.rpm` |
| Diğer tüm dağıtımlar | `DPIMech-<sürüm>-x86_64.AppImage` | `chmod +x DPIMech-*.AppImage`, sonra çalıştır |

[![Copr build status](https://copr.fedorainfracloud.org/coprs/halilkahraman/DPIMech/package/dpimech/status_image/last_build.png)](https://copr.fedorainfracloud.org/coprs/halilkahraman/DPIMech/package/dpimech/)

**Fedora** — depoyu bir kez ekle, sonra güncellemeler sistemle birlikte gelir (`sudo dnf upgrade`):

```sh
sudo dnf copr enable halilkahraman/DPIMech
sudo dnf install dpimech
```

**Arch, CachyOS, EndeavourOS, Manjaro** — Arch paketimiz (`dpimech-bin`) hazır, ama AUR şu anda yeni üye kaydı almadığı
için henüz AUR'da değil. Aynı paket her sürümde indirilebilir: [Releases](https://github.com/halilkhrmn/dpimech/releases/latest)
sayfasından `dpimech-bin-<sürüm>-1-x86_64.pkg.tar.zst` dosyasını indir ve kur (bağımlılıkları pacman getirir; yeni sürüm
çıkınca DPIMech haber verir):

```sh
sudo pacman -U ./dpimech-bin-*-x86_64.pkg.tar.zst
```

Paketler (`.deb`, `.rpm`, COPR, Arch) arka plan servisini de kurar ve başlatır. AppImage kullanıyorsan DPIMech'i açıp bir
kez **Install the service**'e bas (şifreni sorar). AppImage tepsi kütüphanesini
kendi içinde getirir. GNOME'da tepsi simgesi için *AppIndicator and KStatusNotifierItem Support* eklentisi gerekir
(Ubuntu'da hazır gelir).

### Kaynak koddan derleme

[Rust](https://rustup.rs), `systemd` ve pencereyi derlemek için GTK 3 ile AppIndicator geliştirme
paketleri gerekir (Debian/Ubuntu'da:
`sudo apt install build-essential libgtk-3-dev libayatana-appindicator3-dev libxkbcommon-x11-0`).

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

## Ülkelere göre

DPIMech ülkeni sistemin bölge ayarından tahmin eder ve orada engellendiği yaygın olarak bilinen siteleri
listenin başında, kurulum sihirbazında da hazır seçili gösterir. İstediğin başka siteyi her zaman seçebilir
ya da yazabilirsin. Liste [`strategies/domains.json`](../../strategies/domains.json) dosyasında durur ve yeni sürüm
beklemeden güncellenir.

| Ülke | Hazır seçili | Akılda tut |
|---|---|---|
| Türkiye | Discord, Roblox, Wattpad, Imgur | Pek çok engel DNS ile de yapılıyor: aşağıdaki "Hâlâ açılmıyor mu?" maddesine bak. |
| Rusya | YouTube, Discord, Instagram, X, Facebook, LinkedIn, Signal, Viber | YouTube kesilmek yerine yavaşlatılıyor; genelde en iyi zapret çalışır. |
| İran | YouTube, Instagram, X, Telegram, Facebook, Signal, Discord | Filtreleme IP adreslerini ve zaman zaman tüm uluslararası interneti de kapatıyor; o durumda DPIMech yardımcı olamaz. Telegram ve WhatsApp *uygulamaları* doğrudan IP adreslerine bağlandığı için yalnızca web siteleri fayda görür. |
| Kazakistan | SoundCloud | Engellerin çoğu haber sitelerini ve VPN'leri hedefliyor, çoğunlukla IP adresiyle. İhtiyacın olan siteleri "Diğer siteler"e ekle. |
| Belarus | TikTok, bağımsız medya (Zerkalo, Nasha Niva, Svaboda, Belsat, …) | Bazı yayınlar adreslerini sık değiştiriyor; güncel adresi "Diğer siteler"e ekle. |
| Mısır | bağımsız medya (Mada Masr, Zawia3, Cairo 24) | WhatsApp ve benzeri uygulamalardaki sesli/görüntülü aramalar başka yolla engelleniyor ve engelli kalır. |

Diğer ülkelerde Discord başta gelir. Eksik bir site ya da ülke mi var? `strategies/domains.json` için bir
issue ya da pull request aç.

## Bilmekte fayda var

- **Oyunlar:** Windows'ta sıkı hile koruması olan oyunlar (örneğin Valorant ya da FACEIT) tüm
  bilgisayar modu açıkken başlamayabilir ya da seni oyundan atabilir. Oynamadan önce o profili
  kapat ya da sadece ihtiyacın olan uygulamaları seç.
- **Aynı anda tek DPI aracı:** DPIMech'in yanında başka bir engel aşma aracı (GoodbyeDPI, zapret,
  ByeDPI Manager, …) çalışırsa bağlantılar tuhaf şekillerde bozulur. DPIMech böyle bir araç görünce
  seni uyarır.
- **Hâlâ açılmıyor mu? DNS'ini kontrol et.** Bazı sağlayıcılar (örneğin Türkiye'de) siteleri,
  adlarına yanlış adres vererek de engeller; bunu hiçbir motor düzeltemez. Strateji Laboratuvarı,
  DNS'in 1.1.1.1'den farklı cevap verirse uyarır. O zaman DNS'ini `1.1.1.1` / `1.0.0.1` yap (ya da
  sistemde veya tarayıcıda "DNS over HTTPS"i aç).
- **Kayıtlar:** Ayarlar → *Kayıt*. Bir profil hatayla durursa son kayıtlar kendiliğinden bir dosyaya
  kaydedilir. *Ayrıntılı kayıt* her bağlantıyı da (site, adres, nasıl bittiği) yazar; bir şeyin neden
  açılmadığını bulmak için aç ve yardım isterken dosyayı ekle.

## Güvenli mi?

- Motorlar sadece kendi isteklerinin görünüşünü değiştirir.
- Arka plan servisi yönetici yetkisiyle çalışır, çünkü bazı motorlar bunu gerektirir. Motorları
  sadece kendi korumalı klasöründen başlatır; dosya yazabilecek ya da özel dosyaları okuyabilecek
  ayarları reddeder.
- İnternet sağlayıcını tespit etmek isteğe bağlıdır. Sadece *Detect my provider*'a bastığında
  olur ve genel IP adresini `ipwho.is` sitesine gönderir.
