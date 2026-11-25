# Değişiklikler

## 0.4.8
- VPN istemcileri (Cloudflare WARP, WireGuard, OpenVPN, …) yalnızca bağlıyken uyarıda gösteriliyor; yalnızca arka plan servislerinin çalışması artık uyarı vermiyor.
- "Ayrıca çalışıyor" günlük satırı artık profilin adıyla değil servis tarafından yazılıyor; profilin uygulaması diğer aracı bulmuş gibi görünmüyor.

## 0.4.7
- DPIMech her ağda hangi stratejinin çalıştığını hatırlıyor (ağ modeminden tanınıyor, hiçbir yere bir şey gönderilmiyor) ve o ağa dönünce ona geçiyor. Yeni bir ağda profillerin sitelerini kontrol ediyor; açılmıyorlarsa stratejileri kendisi deneyip en iyisini kullanıyor (ByeDPI, SpoofDPI, tpws).
- Arka plan servisi çalışmıyorsa pencere bunu "Servisi başlat" düğmesiyle söylüyor; profiller, motorlar ve Strateji Laboratuvarı servis yeniden çalışana kadar bekliyor.
- Ayarlar → Ağ, modeminin soruları ilettiği DNS servisini de gösteriyor (örneğin modemde ayarlı Cloudflare).
- Kısayoldan profil başlatınca ilerleme çubuğu olmayan sade bir "… başlatılıyor" bildirimi çıkıyor; yalnızca hata olursa ikinci bir bildirim geliyor.
- DPIMech, engel aşmaya karışabilen yazılımlar için de uyarıyor (AdGuard, Killer ve SmartByte ağ servisleri, ZoneAlarm, VPN istemcileri, Cloudflare WARP).

## 0.4.6
- İnternet yokken motor artık tekrar tekrar yeniden başlatılmıyor. DPIMech bekliyor, ağını kontrol etmeni söylüyor (kartta ve tek bir bildirimde) ve bağlantı gelince kendiliğinden devam ediyor.
- Bir profil bağlanırken ya da bir test çalışırken tepsi simgesi turuncu yanıp sönüyor.
- DPIMech ya da arka plan servisi çökerse bir sonraki açılışta bunu söylüyor ve kaydedilenleri göndermeyi öneriyor.
- Profil kısayolu artık kendi küçük penceresini açmıyor: ilerlemeyi (profili açma, uygulamayı başlatma) ve sonucu tek bir sistem bildirimi gösteriyor.
- Ayarlar → Ağ, bilgisayarının şu anda kullandığı DNS sunucularını ve bunların ağından mı geldiğini yoksa elle mi ayarlandığını gösteriyor; sorun raporlarında da yer alıyor.
- İsteğe bağlı: DPIMech DNS'ini bilinen bir genel servise (Cloudflare, Google, Quad9, AdGuard, OpenDNS, Yandex, …) geçirebiliyor. Önceki ayarlarını kaydediyor; "Ağının kendi DNS'i" seçildiğinde ya da DPIMech kaldırıldığında onları geri koyuyor. Windows ve Linux'ta (systemd-resolved ile, mümkünse şifreli).

## 0.4.5
- Windows: "Yeniden başlat ve güncelle" yeniden çalışıyor. 0.4.3 ve 0.4.4'te, uygulamaları yönlendiren bir profil bir kez durdurulduktan sonra "The handle is invalid" hatası veriyordu. Bu hatayı görürseniz bilgisayarı yeniden başlatıp hiçbir profili durdurmadan güncelleyin ya da sürüm sayfasındaki kurulum dosyasını çalıştırın.

## 0.4.4
- Wayland kullanan Linux'ta (örneğin KDE Plasma), DPIMech tepside başladığında tepsideki "DPIMech'i göster" artık pencereyi açıyor; önceden pencere görünmüyor ve görev çubuğunda boş bir girdi kalıyordu.
- DPIMech oturum açılışında tepside başladığında, tepsi simgesi birkaç saniye geç geldi diye penceresi artık açılmıyor.
- "DPIMech'i göster" simge durumuna küçültülmüş pencereyi de geri getiriyor.

## 0.4.3
- DPIMech kapalıyken hiçbir şey çalışmıyor. "Servisle birlikte başlat" artık "DPIMech ile birlikte başlat": profil DPIMech açılınca başlıyor, çıkınca bütün profiller duruyor.
- Bilgisayarın internet bağlantısı gittiğinde DPIMech artık motoru yeniden başlatmıyor ve ayarın çalışmadığını söylemiyor; kartta bağlantı olmadığı yazıyor.
- Servis günlüğü küçük kalıyor: motorun her bağlantı için yazdıkları yalnızca ayrıntılı günlük açıkken tutuluyor.
- Bilgisayar açıldığında ağ henüz hazır değilse güncelleme denetimi kısa süre sonra yeniden deniyor.
- Uygulama bazlı yönlendirme ProxiFyre'ı artık aniden sonlandırmak yerine düzgünce kapatıyor.
- Günlükler sayfasına süzgeç geldi; başka bir DPI aracı uyarısı gizlenebiliyor ve günlüğe bir kez yazılıyor; pencereyi ilk kapattığında DPIMech tepside çalışmaya devam ettiğini söylüyor.

## 0.4.2
- DPIMech'ten çıkınca profilleri artık duruyor. DPIMech kapalıyken yalnızca "Servisle birlikte başlat" açık olan profiller, bilgisayar açıldığından itibaren çalışmaya devam ediyor.
- Servisle birlikte başlayan bir profil varsa DPIMech oturum açınca tepside başlıyor; böylece neyin çalıştığını her zaman görüyorsun.

## 0.4.1
- Yeni logo artık görev çubuğunda, Başlat menüsünde, Linux uygulama menüsünde ve güncellemeden önce oluşturulan profil kısayollarında da görünüyor; 0.4.0'a güncelleyince bunlarda eski logo kalıyordu.

## 0.4.0
- Yeni logo: Kim De Vries'in elle çizdiği bir bukalemun.
- Daha sade profil düzenleyici: bağlantıyla tek bir "Çalışır durumda tut" anahtarı ilgileniyor; port, kontrol ayarları ve strateji argümanları "Gelişmiş ayarlar" altında.
- Ayarlar kısaldı: DPIMech bilgisayarla birlikte başladığında her zaman tepside açılıyor; "Ayrıntılı kayıt" anahtarı artık Kayıtlar sayfasında.

## 0.3.1
- Arch, CachyOS, EndeavourOS ve Manjaro: her sürüm sayfasında artık bir Arch paketi (`dpimech-bin`) var; `sudo pacman -U` ile kurulur.
- libxkbcommon-x11 olmayan Linux sistemlerde DPIMech açılışta çökmüyor; hangi paketi kurman gerektiğini söylüyor.
- Her paket artık sürüm yayımlanmadan önce temiz bir sistemde kurulup çalıştırılarak test ediliyor.

## 0.3.0
- Farsça (فارسی) ve Arapça (العربية) arayüz.
- Ülkende engelli siteler listenin başında ve kurulum sihirbazında hazır seçili: Türkiye, Rusya, İran, Kazakistan, Belarus ve Mısır.
- Yeni siteler: Telegram, WhatsApp, Facebook, Signal, Viber, LinkedIn, TikTok, SoundCloud, Imgur, ayrıca Belarus ve Mısır'ın bağımsız haber siteleri.
- Site listesi artık DPIMech'in yeni sürümünü beklemeden kendiliğinden güncelleniyor.
- Bu pencere: güncellemeden sonra DPIMech nelerin değiştiğini gösteriyor.
