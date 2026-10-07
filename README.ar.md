<p align="center"><a href="README.md">English</a> · <a href="README.tr.md">Türkçe</a> · <a href="README.ru.md">Русский</a> · <a href="README.fa.md">فارسی</a> · <b>العربية</b></p>

<!-- ---------- الترويسة ---------- -->
<div align="center">
  <img src="crates/gui/assets/logo-long-square.svg" width="260" alt="DPIMech">
  <p dir="rtl">افتح المواقع والتطبيقات التي يحجبها مزوّد الإنترنت لديك (Discord وYouTube و…).<br>مجاني ومفتوح المصدر، لأنظمة Windows وLinux وmacOS.</p>

  <img alt="الترخيص" src="https://img.shields.io/github/license/halilkhrmn/dpimech?color=397256&style=flat-square">
  <img alt="الإصدار" src="https://img.shields.io/github/v/release/halilkhrmn/dpimech?color=397256&style=flat-square">
  <img alt="التنزيلات" src="https://img.shields.io/github/downloads/halilkhrmn/dpimech/total?color=397256&style=flat-square">
  <img alt="آخر تعديل" src="https://img.shields.io/github/last-commit/halilkhrmn/dpimech?color=397256&style=flat-square">
  <img alt="النجوم" src="https://img.shields.io/github/stars/halilkhrmn/dpimech?color=397256&style=flat-square">
</div>

<div dir="rtl">

> **DPIMech ليس VPN.** يشغّل أدوات معروفة ومفتوحة المصدر تغيّر شكل حركة بياناتك، فلا يعود الفحص العميق
> للحزم (DPI) لدى المزوّد قادرًا على حجبها. تذهب بياناتك مباشرة إلى الموقع كما كانت: لا يتغيّر زمن
> الاستجابة (ping)، لكن عنوان IP الخاص بك لا يُخفى.

<!-- ---------- الميزات ---------- -->
## الميزات

- [x] تشغيل وإيقاف بنقرة واحدة: *ملف شخصي* جاهز لكل موقع أو تطبيق، مثل Discord
- [x] التطبيقات التي تختارها فقط، أو الكمبيوتر كله، أو وكيل محلي
- [x] *مختبر الاستراتيجيات* يجد الإعدادات التي تعمل على اتصالك
- [x] يثبّت المحركات ويحدّثها، ويتحقق من كل تنزيل بمجموع التحقق الخاص به
- [x] يعيد تشغيل المحرك المتعلّق بنفسه في أقل من ثانية
- [x] اختصارات على سطح المكتب تشغّل الملف الشخصي ثم تفتح التطبيق
- [x] English وTürkçe وРусский وفارسی والعربية
- [x] خفيف: نحو 30 ميغابايت من الذاكرة للنافذة و20 ميغابايت لخدمة الخلفية

<!-- ---------- التنزيل ---------- -->
## التنزيل

| النظام | طريقة التثبيت |
|---|---|
| **Windows 10 / 11** | الملف `dpimech-setup-<version>.exe` من صفحة [Releases](https://github.com/halilkhrmn/dpimech/releases/latest) |
| **Ubuntu وDebian وMint** | ملف `.deb` من Releases ← `sudo apt install ./dpimech_*_amd64.deb` |
| **Fedora** | `sudo dnf copr enable halilkahraman/DPIMech && sudo dnf install dpimech` |
| **Arch وCachyOS وManjaro** | ملف `.pkg.tar.zst` من Releases ← `sudo pacman -U ./dpimech-bin-*.pkg.tar.zst` |
| **openSUSE وRHEL** | ملف `.rpm` من Releases ← `sudo dnf install ./dpimech-*.x86_64.rpm` |
| **توزيعات Linux الأخرى** | AppImage من Releases |
| **macOS** *(معاينة مبكرة)* | ملف `.dmg` من Releases |

ثم افتح DPIMech واختر **فقط اجعله يعمل**. المحركات والبلدان والبناء من المصدر وحل المشكلات موجودة في
**[دليل الاستخدام](docs/guide/ar.md)**.

<!-- ---------- لقطات الشاشة ---------- -->
## لقطات الشاشة

</div>

<p align="center">
  <img src="site/screenshots/windows-profiles.png" width="49%" alt="الملفات الشخصية على Windows">
  <img src="site/screenshots/windows-editor.png" width="49%" alt="ملف شخصي لـ Discord على Windows">
  <img src="site/screenshots/linux-profiles.png" width="49%" alt="الملفات الشخصية على Linux">
  <img src="site/screenshots/linux-editor.png" width="49%" alt="محرر الملف الشخصي على Linux">
</p>

<div dir="rtl">

<!-- ---------- المساهمة ---------- -->
## الملاحظات والمساهمة

***نرحّب بكل مساهمة!***

- البلاغات عن الأخطاء والأفكار: [Issues](https://github.com/halilkhrmn/dpimech/issues) أو *الإبلاغ عن مشكلة* في إعدادات التطبيق.
- الشيفرة: انسخ المشروع (fork) وافتح طلب دمج (pull request)؛ يشرح [CONTRIBUTING.md](CONTRIBUTING.md) الطريقة.
- الترجمات موجودة في [`crates/gui/lang`](crates/gui/lang) كملفات `.po` عادية.

## شكر

- **الشعار:** من تصميم [Kim De Vries](https://www.artstation.com/kdevries21)، مرسوم يدويًا. لم يُستخدم الذكاء الاصطناعي في صنعه.
- DPIMech يدير هذه المشاريع فقط؛ العمل الحقيقي عملها:
  [ByeDPI](https://github.com/hufrea/byedpi) ·
  [zapret](https://github.com/bol-van/zapret) ·
  [GoodbyeDPI](https://github.com/ValdikSS/GoodbyeDPI) ·
  [WinDivert](https://github.com/basil00/WinDivert) ·
  [ProxiFyre](https://github.com/wiresock/proxifyre) ·
  [Windows Packet Filter](https://github.com/wiresock/ndisapi).
- قوائم الاستراتيجيات: [ByeDPI Manager](https://github.com/romanvht/ByeDPIManager) و
  [SplitWire-Turkey](https://github.com/cagritaskn/SplitWire-Turkey).
  الأيقونات: [Fluent UI System Icons](https://github.com/microsoft/fluentui-system-icons). مبني باستخدام [Slint](https://slint.dev).
- تُطوَّر الشيفرة بمساعدة الذكاء الاصطناعي (Claude)، وتُختبر كل ميزة يدويًا؛
  ما الذي اختُبر وكيف مسجّل في [docs/PROGRESS.md](docs/PROGRESS.md).

## الترخيص

DPIMech برنامج حر بموجب [رخصة جنو العمومية العامة، الإصدار 3.0 أو أحدث](LICENSE): يمكنك استخدامه
ودراسته ومشاركته وتعديله. تحتفظ المحركات التي ينزّلها بتراخيصها الخاصة.

</div>

<p align="center">صُنع بـ ❤️</p>
