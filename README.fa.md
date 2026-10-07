<p align="center"><a href="README.md">English</a> · <a href="README.tr.md">Türkçe</a> · <a href="README.ru.md">Русский</a> · <b>فارسی</b> · <a href="README.ar.md">العربية</a></p>

<!-- ---------- سرآغاز ---------- -->
<div align="center">
  <img src="crates/gui/assets/logo-long-square.svg" width="260" alt="DPIMech">
  <p dir="rtl">سایت‌ها و برنامه‌هایی (Discord، YouTube، …) را که ارائه‌دهندهٔ اینترنت شما مسدود کرده باز کنید.<br>رایگان و متن‌باز، برای Windows، Linux و macOS.</p>

  <img alt="مجوز" src="https://img.shields.io/github/license/halilkhrmn/dpimech?color=397256&style=flat-square">
  <img alt="نسخه" src="https://img.shields.io/github/v/release/halilkhrmn/dpimech?color=397256&style=flat-square">
  <img alt="دانلودها" src="https://img.shields.io/github/downloads/halilkhrmn/dpimech/total?color=397256&style=flat-square">
  <img alt="آخرین کامیت" src="https://img.shields.io/github/last-commit/halilkhrmn/dpimech?color=397256&style=flat-square">
  <img alt="ستاره‌ها" src="https://img.shields.io/github/stars/halilkhrmn/dpimech?color=397256&style=flat-square">
</div>

<div dir="rtl">

> **DPIMech یک VPN نیست.** ابزارهای شناخته‌شده و متن‌بازی را اجرا می‌کند که ظاهر ترافیک شما را تغییر
> می‌دهند تا بازرسی عمیق بسته‌ها (DPI) نزد ارائه‌دهنده دیگر نتواند آن را مسدود کند. ترافیک همچنان
> مستقیم به سایت می‌رود: پینگ تغییر نمی‌کند، اما نشانی IP شما پنهان نمی‌شود.

<!-- ---------- امکانات ---------- -->
## امکانات

- [x] روشن و خاموش با یک کلیک: یک *پروفایل* آماده برای هر سایت یا برنامه، مثلاً Discord
- [x] فقط برنامه‌هایی که انتخاب می‌کنید، کل رایانه یا یک پراکسی محلی
- [x] *آزمایشگاه استراتژی* تنظیماتی را پیدا می‌کند که روی اتصال شما کار می‌کنند
- [x] موتورها را نصب و به‌روز می‌کند و هر دانلود را با چک‌سام آن بررسی می‌کند
- [x] موتوری را که گیر کرده خودش در کمتر از یک ثانیه دوباره راه‌اندازی می‌کند
- [x] میانبرهای دسکتاپ که پروفایل را روشن و برنامه را باز می‌کنند
- [x] English، Türkçe، Русский، فارسی، العربية
- [x] سبک: حدود ۳۰ مگابایت حافظه برای پنجره و ۲۰ مگابایت برای سرویس پس‌زمینه

<!-- ---------- دانلود ---------- -->
## دانلود

| سیستم | روش نصب |
|---|---|
| **Windows 10 / 11** | فایل `dpimech-setup-<version>.exe` از صفحهٔ [Releases](https://github.com/halilkhrmn/dpimech/releases/latest) |
| **Ubuntu، Debian، Mint** | فایل `.deb` از Releases ← `sudo apt install ./dpimech_*_amd64.deb` |
| **Fedora** | `sudo dnf copr enable halilkahraman/DPIMech && sudo dnf install dpimech` |
| **Arch، CachyOS، Manjaro** | فایل `.pkg.tar.zst` از Releases ← `sudo pacman -U ./dpimech-bin-*.pkg.tar.zst` |
| **openSUSE، RHEL** | فایل `.rpm` از Releases ← `sudo dnf install ./dpimech-*.x86_64.rpm` |
| **سایر توزیع‌های Linux** | AppImage از Releases |
| **macOS** *(پیش‌نمایش اولیه)* | فایل `.dmg` از Releases |

سپس DPIMech را باز کنید و **فقط کار کند** را انتخاب کنید. موتورها، کشورها، ساخت از کد منبع و رفع
مشکلات در **[راهنمای استفاده](docs/guide/fa.md)** آمده است.

<!-- ---------- تصاویر ---------- -->
## تصاویر

</div>

<p align="center">
  <img src="site/screenshots/windows-profiles.png" width="49%" alt="پروفایل‌ها در Windows">
  <img src="site/screenshots/windows-editor.png" width="49%" alt="پروفایل Discord در Windows">
  <img src="site/screenshots/linux-profiles.png" width="49%" alt="پروفایل‌ها در Linux">
  <img src="site/screenshots/linux-editor.png" width="49%" alt="ویرایشگر پروفایل در Linux">
</p>

<div dir="rtl">

<!-- ---------- مشارکت ---------- -->
## بازخورد و مشارکت

***از هر کمکی استقبال می‌کنیم!***

- گزارش خطا و ایده‌ها: [Issues](https://github.com/halilkhrmn/dpimech/issues) یا *گزارش مشکل* در تنظیمات برنامه.
- کد: پروژه را fork کنید و یک pull request باز کنید؛ [CONTRIBUTING.md](CONTRIBUTING.md) روش کار را توضیح می‌دهد.
- ترجمه‌ها در پوشهٔ [`crates/gui/lang`](crates/gui/lang) به صورت فایل‌های معمولی `.po` هستند.

## سپاس

- **نشان (لوگو):** اثر [Kim De Vries](https://www.artstation.com/kdevries21)، با دست کشیده شده. در ساخت آن از هوش مصنوعی استفاده نشده است.
- DPIMech فقط این پروژه‌ها را مدیریت می‌کند؛ کار اصلی را آن‌ها انجام می‌دهند:
  [ByeDPI](https://github.com/hufrea/byedpi) ·
  [zapret](https://github.com/bol-van/zapret) ·
  [GoodbyeDPI](https://github.com/ValdikSS/GoodbyeDPI) ·
  [WinDivert](https://github.com/basil00/WinDivert) ·
  [ProxiFyre](https://github.com/wiresock/proxifyre) ·
  [Windows Packet Filter](https://github.com/wiresock/ndisapi).
- فهرست‌های استراتژی: [ByeDPI Manager](https://github.com/romanvht/ByeDPIManager)،
  [SplitWire-Turkey](https://github.com/cagritaskn/SplitWire-Turkey).
  نمادها: [Fluent UI System Icons](https://github.com/microsoft/fluentui-system-icons). ساخته‌شده با [Slint](https://slint.dev).
- کد با کمک هوش مصنوعی (Claude) نوشته می‌شود و هر قابلیت با دست آزمایش می‌شود؛
  اینکه چه چیزی و چگونه آزمایش شده در [docs/PROGRESS.md](docs/PROGRESS.md) ثبت شده است.

## مجوز

DPIMech نرم‌افزار آزاد تحت [مجوز عمومی همگانی گنو نسخهٔ ۳ یا بالاتر](LICENSE) است: می‌توانید از آن
استفاده کنید، آن را بررسی، منتشر و تغییر دهید. موتورهایی که دانلود می‌کند مجوزهای خودشان را دارند.

</div>

<p align="center">ساخته‌شده با ❤️</p>
