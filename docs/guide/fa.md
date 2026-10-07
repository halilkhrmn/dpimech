<p align="center"><a href="en.md">English</a> · <a href="tr.md">Türkçe</a> · <a href="ru.md">Русский</a> · <b>فارسی</b> · <a href="ar.md">العربية</a></p>

<div dir="rtl">

# DPIMech — راهنمای استفاده

[← بازگشت به README](../../README.fa.md)

## بر اساس کشور

DPIMech کشور شما را از تنظیم منطقهٔ سیستم حدس می‌زند و سایت‌هایی را که به‌طور گسترده گزارش شده در آنجا
مسدودند در ابتدای فهرست و از پیش انتخاب‌شده در راه‌انداز نشان می‌دهد. همیشه می‌توانید سایت‌های دیگری
انتخاب کنید یا هر سایتی را بنویسید. فهرست در [`strategies/domains.json`](../../strategies/domains.json) است و بدون
نسخهٔ جدید به‌روز می‌شود.

| کشور | از پیش انتخاب‌شده | در نظر داشته باشید |
|---|---|---|
| ایران | YouTube، Instagram، X، Telegram، Facebook، Signal، Discord | فیلترینگ نشانی‌های IP و گاهی کل اینترنت بین‌الملل را هم می‌بندد؛ در آن صورت DPIMech کاری از پیش نمی‌برد. *برنامه‌های* Telegram و WhatsApp مستقیم به نشانی‌های IP وصل می‌شوند، پس فقط وب‌سایتشان سود می‌برد. |
| ترکیه | Discord، Roblox، Wattpad، Imgur | بسیاری از مسدودسازی‌ها با DNS هم انجام می‌شود: بند «هنوز باز نمی‌شود؟» را ببینید. |
| روسیه | YouTube، Discord، Instagram، X، Facebook، LinkedIn، Signal، Viber | YouTube به جای قطع، کند می‌شود؛ معمولاً zapret بهترین نتیجه را دارد. |
| قزاقستان | SoundCloud | بیشتر مسدودسازی‌ها سایت‌های خبری و VPNها را هدف می‌گیرند، اغلب با نشانی IP. سایت‌های لازم را در «سایت‌های دیگر» اضافه کنید. |
| بلاروس | TikTok، رسانه‌های مستقل (Zerkalo، Nasha Niva، Svaboda، Belsat، …) | برخی رسانه‌ها نشانی خود را مرتب عوض می‌کنند؛ نشانی فعلی را در «سایت‌های دیگر» اضافه کنید. |
| مصر | رسانه‌های مستقل (Mada Masr، Zawia3، Cairo 24) | تماس صوتی و تصویری در WhatsApp و برنامه‌های مشابه به روش دیگری مسدود است و مسدود می‌ماند. |

در کشورهای دیگر Discord اول می‌آید. سایت یا کشوری کم است؟ برای `strategies/domains.json` یک issue یا
pull request باز کنید.

## Windows

### شروع

1. فایل `dpimech-setup-<version>.exe` را از [Releases](https://github.com/halilkhrmn/dpimech/releases)
   **دانلود و اجرا کنید**. Windows یک بار اجازه می‌خواهد؛ پس از آن سرویس پس‌زمینه خودش با رایانه شروع می‌شود.
2. **DPIMech را باز کنید** و **فقط کار کند** را انتخاب کنید. سایت‌ها، محل کار (فقط برخی برنامه‌ها، کل رایانه
   یا پراکسی) را انتخاب کنید و **شروع** را بزنید. DPIMech هرچه لازم است نصب می‌کند، تنظیمات را روی اتصال
   شما می‌آزماید و بهترین را روشن می‌کند.
3. همین. بعداً با **راه‌اندازی سریع** پروفایل دیگری اضافه کنید، یا در تنظیمات *حالت ساده* را خاموش کنید
   تا موتورها، آزمایشگاه راهبرد و گزارش‌ها را ببینید.

### موتورها در Windows

| موتور | در چه چیزی خوب است | نیاز دارد |
|---|---|---|
| **ByeDPI** | پراکسی محلی. ساده و امن. همراه ProxiFyre فقط برنامه‌های انتخابی را عبور می‌دهد. | چیز اضافه‌ای نه |
| **ProxiFyre** *(کمکی)* | فقط برنامه‌های انتخابی شما را از ByeDPI عبور می‌دهد. | درایور **Windows Packet Filter** (از صفحهٔ موتورها نصب می‌شود؛ شاید راه‌اندازی دوباره لازم باشد) |
| **zapret (winws)** | روی کل رایانه، حتی برای UDP (صدای Discord، QUIC). قوی‌ترین گزینه. | از WinDivert استفاده می‌کند (همراهش است). با دسترسی مدیر از طریق سرویس DPIMech اجرا می‌شود. |
| **GoodbyeDPI** | ابزار کلاسیک برای کل رایانه. ساده، اما دیگر به‌طور فعال توسعه نمی‌یابد. | مانند بالا. |

در هر زمان فقط **یک** موتور WinDivert (zapret یا GoodbyeDPI) می‌تواند اجرا شود. برخی آنتی‌ویروس‌ها
WinDivert را به اشتباه خطرناک می‌دانند؛ DPIMech آن را فقط از انتشارهای رسمی موتورها دانلود می‌کند.

## Linux

### موتورها در Linux

| موتور | حالت | نیاز دارد |
|---|---|---|
| **zapret (nfqws)** | کل رایانه. DPIMech هنگام اجرا قواعد nftables خودش را اضافه و سپس حذف می‌کند. | `nftables` و ماژول‌های هستهٔ `nft_queue` و `nfnetlink_queue` (در بیشتر توزیع‌ها استاندارد) |
| **zapret (tpws)** | کل رایانه، فقط برخی برنامه‌ها یا پراکسی SOCKS محلی | چیز اضافه‌ای نه (cgroup v2 برای «فقط برخی برنامه‌ها») |
| **ByeDPI** | فقط برخی برنامه‌ها یا پراکسی SOCKS محلی | چیز اضافه‌ای نه (cgroup v2 برای «فقط برخی برنامه‌ها») |
| **SpoofDPI** | فقط برخی برنامه‌ها یا پراکسی SOCKS محلی. می‌تواند نام‌ها را از راه HTTPS (DoH) پیدا کند که از مسدودسازی DNS هم عبور می‌کند | چیز اضافه‌ای نه |

### نصب

از [Releases](https://github.com/halilkhrmn/dpimech/releases) دانلود کنید:

| سیستم شما | فایل | نصب |
|---|---|---|
| Ubuntu، Debian، Mint، Pop!_OS | `dpimech_<version>_amd64.deb` | `sudo apt install ./dpimech_*_amd64.deb` |
| Fedora | نیازی به دانلود نیست: [مخزن COPR](https://copr.fedorainfracloud.org/coprs/halilkahraman/DPIMech/) | پایین‌تر |
| Arch، CachyOS، EndeavourOS، Manjaro | `dpimech-bin-<version>-1-x86_64.pkg.tar.zst` | `sudo pacman -U ./dpimech-bin-*.pkg.tar.zst` |
| openSUSE، RHEL | `dpimech-<version>-1.x86_64.rpm` | `sudo dnf install ./dpimech-*.x86_64.rpm` |
| هر توزیع دیگر | `DPIMech-<version>-x86_64.AppImage` | `chmod +x DPIMech-*.AppImage` و سپس اجرا |

**Fedora** — یک بار مخزن را اضافه کنید؛ پس از آن به‌روزرسانی‌ها همراه بقیهٔ سیستم می‌آیند (`sudo dnf upgrade`):

</div>

```sh
sudo dnf copr enable halilkahraman/DPIMech
sudo dnf install dpimech
```

<div dir="rtl">

**Arch، CachyOS، EndeavourOS، Manjaro** — بستهٔ Arch (`dpimech-bin`) آماده است، اما هنوز در AUR نیست: AUR در حال حاضر
عضو جدید نمی‌پذیرد. همین بسته به هر انتشار پیوست می‌شود: `dpimech-bin-<version>-1-x86_64.pkg.tar.zst` را از
[Releases](https://github.com/halilkhrmn/dpimech/releases/latest) دانلود و نصب کنید (pacman وابستگی‌ها را می‌آورد و DPIMech
نسخهٔ جدید را خبر می‌دهد):

</div>

```sh
sudo pacman -U ./dpimech-bin-*-x86_64.pkg.tar.zst
```

<div dir="rtl">

بسته‌ها (`.deb`، `.rpm`، COPR، Arch) سرویس پس‌زمینه را هم نصب و اجرا می‌کنند. با AppImage، DPIMech را باز کنید و
یک بار **نصب سرویس** را بزنید (گذرواژه می‌خواهد). در GNOME نماد سینی سیستم به افزونهٔ
*AppIndicator and KStatusNotifierItem Support* نیاز دارد (در Ubuntu از پیش هست).

### ساخت از کد منبع

به [Rust](https://rustup.rs)، ‏`systemd` و بسته‌های توسعهٔ GTK 3 و AppIndicator نیاز دارد (در Debian/Ubuntu:
`sudo apt install build-essential libgtk-3-dev libayatana-appindicator3-dev libxkbcommon-x11-0`).

</div>

```sh
git clone https://github.com/halilkhrmn/dpimech && cd dpimech
cargo build --release
sudo target/release/dpimech-service install   # background service (systemd unit "dpimech")
target/release/dpimech                         # the window
```

<div dir="rtl">

## macOS (پیش‌نمایش اولیه)

`DPIMech-<version>.dmg` را از [Releases](https://github.com/halilkhrmn/dpimech/releases) دانلود کنید و
DPIMech را به Applications بکشید. برنامه هنوز امضا نشده است: بار اول روی آن کلیک راست کنید و **Open** را
بزنید. سپس یک بار **نصب سرویس** را بزنید. فعلاً در macOS فقط **پراکسی SOCKS محلی** (tpws از zapret) ممکن
است؛ `127.0.0.1:<port>` را در مرورگر یا برنامه تنظیم کنید.

## خوب است بدانید

- **بازی‌ها:** در Windows، بازی‌هایی با ضدتقلب سخت‌گیر (مثلاً Valorant یا FACEIT) ممکن است وقتی موتور کل
  رایانه روشن است اجرا نشوند یا اتصال شما را قطع کنند. پیش از بازی آن پروفایل را خاموش کنید.
- **هر بار فقط یک ابزار DPI:** اگر ابزار دیگری برای دور زدن (GoodbyeDPI، zapret، ByeDPI Manager، …) کنار
  DPIMech اجرا شود، اتصال‌ها به شکل عجیبی خراب می‌شوند. DPIMech در این صورت هشدار می‌دهد.
- **هنوز باز نمی‌شود؟ DNS را بررسی کنید.** برخی ارائه‌دهندگان با دادن نشانی اشتباه برای نام سایت‌ها هم
  مسدود می‌کنند؛ هیچ موتوری این را درست نمی‌کند. آزمایشگاه راهبرد اگر DNS شما با 1.1.1.1 فرق داشته باشد
  هشدار می‌دهد. آنگاه DNS را `1.1.1.1` / `1.0.0.1` بگذارید (یا «DNS over HTTPS» را در سیستم یا مرورگر روشن کنید).
- **گزارش‌ها:** تنظیمات ← *گزارش*. اگر پروفایلی با خطا متوقف شود، گزارش اخیر خودکار در فایلی ذخیره
  می‌شود. *گزارش مفصل* هر اتصال را هم ثبت می‌کند؛ هنگام درخواست کمک فایل را پیوست کنید.

## آیا امن است؟

- موتورها فقط شکل درخواست‌های خود شما را تغییر می‌دهند.
- سرویس پس‌زمینه با دسترسی مدیر اجرا می‌شود چون برخی موتورها به آن نیاز دارند. فقط موتورها را از پوشهٔ
  محافظت‌شدهٔ خودش اجرا می‌کند و تنظیماتی را که بتوانند فایل بنویسند یا فایل‌های خصوصی را بخوانند رد می‌کند.
- تشخیص ارائه‌دهندهٔ اینترنت اختیاری است و فقط وقتی *تشخیص ارائه‌دهنده* را بزنید انجام می‌شود؛ نشانی IP
  عمومی شما را به `ipwho.is` می‌فرستد.

</div>
