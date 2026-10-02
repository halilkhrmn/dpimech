<p align="center"><a href="README.md">English</a> · <a href="README.tr.md">Türkçe</a> · <a href="README.ru.md">Русский</a> · <a href="README.fa.md">فارسی</a> · <b>العربية</b></p>

<p align="center"><img src="crates/gui/assets/logo.png" width="128" alt="شعار DPIMech"></p>

<div dir="rtl">

# DPIMech

**افتح المواقع والتطبيقات (Discord وYouTube و…) التي يحجبها مزود الإنترنت لديك.**

بعض مزودي الإنترنت ينظرون داخل حركة مرورك ويحجبون مواقع معيّنة. تُسمّى هذه التقنية *DPI* (الفحص العميق
للحزم). يشغّل DPIMech أدوات صغيرة معروفة ومفتوحة المصدر («محركات») تغيّر شكل حركة مرورك، فيتوقف الحجب عن العمل.

> **DPIMech ليس VPN.** تذهب حركة مرورك مباشرةً إلى الموقع؛ لا يمر شيء عبر خادم شخص آخر. تبقى السرعة
> والبينغ كما هي، لكن عنوان IP الخاص بك لا يُخفى، والمواقع المحجوبة بطريقة أخرى (مثلًا بعنوان IP) تبقى محجوبة.

| | الحالة |
|---|---|
| **Windows 10 / 11** | جاهز. المثبّت في صفحة [Releases](https://github.com/halilkhrmn/dpimech/releases). |
| **Linux** | جاهز: ‏`.deb` و‏`.rpm` وAppImage في صفحة Releases، وFedora عبر COPR (انظر أدناه). |
| **macOS** | معاينة مبكرة: ‏`.dmg` في صفحة Releases، وكيل محلي فقط (انظر أدناه). |

</div>

<p align="center">
  <a href="site/screenshots/windows-profiles.png"><img src="site/screenshots/windows-profiles.png" width="49%" alt="صفحة الملفات الشخصية في DPIMech على Windows"></a>
  <a href="site/screenshots/windows-editor.png"><img src="site/screenshots/windows-editor.png" width="49%" alt="تعديل ملف شخصي لتطبيق Discord على Windows"></a>
</p>

<div dir="rtl">

## ماذا يفعل

- **نقرة للتشغيل ونقرة للإيقاف.** كل *ملف شخصي* إعداد جاهز، مثل «Discord».
- **فقط حيث تريد.** التطبيقات التي تختارها فقط، أو الحاسوب بأكمله، أو وكيل محلي.
- **يجد الإعدادات التي تعمل لديك.** يجرّب *مختبر الإستراتيجيات* عشرات الإعدادات على مواقع حقيقية ويُظهر
  أيها يعمل على اتصالك.
- **يثبّت المحركات ويحدّثها نيابةً عنك** من صفحاتها الرسمية على GitHub، ويتحقق من كل تنزيل مقابل الـ checksum المنشور.
- **يُبقي الاتصال يعمل.** إذا علق محرك، يعيد DPIMech تشغيله في أقل من ثانية. ويُظهر فحص الاتصال متوسط
  البينغ لكل ملف شخصي.
- **الاختصارات.** انقر بزر الفأرة الأيمن على ملف شخصي (أو اضغط ⋯) ← *إنشاء اختصار*: أيقونة على سطح المكتب
  أو في القائمة تشغّل الملف الشخصي ثم تفتح التطبيق.
- **خفيف:** نحو 25–35 ميغابايت من الذاكرة للنافذة ونحو 20 ميغابايت لخدمة الخلفية.
- **بلغتك:** العربية والفارسية والإنجليزية والتركية والروسية.

## حسب البلد

يخمّن DPIMech بلدك من إعداد المنطقة في النظام، ويعرض أولًا المواقع التي تفيد التقارير على نطاق واسع
بحجبها هناك، ويحددها مسبقًا في معالج الإعداد. يمكنك دائمًا اختيار غيرها أو كتابة أي موقع. القائمة موجودة
في [`strategies/domains.json`](strategies/domains.json) وتتحدث دون إصدار جديد.

| البلد | محدد مسبقًا | تذكّر |
|---|---|---|
| مصر | وسائل الإعلام المستقلة (Mada Masr وZawia3 وCairo 24) | المكالمات الصوتية والمرئية في WhatsApp والتطبيقات المشابهة محجوبة بطريقة أخرى وتبقى محجوبة. |
| إيران | YouTube وInstagram وX وTelegram وFacebook وSignal وDiscord | الترشيح يحجب عناوين IP أيضًا، وأحيانًا الإنترنت الدولي بأكمله؛ عندها لا يستطيع DPIMech المساعدة. *تطبيقا* Telegram وWhatsApp يتصلان بعناوين IP مباشرةً، لذا تستفيد مواقعهما فقط. |
| تركيا | Discord وRoblox وWattpad وImgur | كثير من الحجب يتم عبر DNS أيضًا: انظر «ما زال محجوبًا؟» أدناه. |
| روسيا | YouTube وDiscord وInstagram وX وFacebook وLinkedIn وSignal وViber | YouTube يُبطّأ بدلًا من قطعه؛ عادةً يعمل zapret بشكل أفضل. |
| كازاخستان | SoundCloud | معظم الحجب يستهدف المواقع الإخبارية وخدمات VPN، غالبًا بعنوان IP. أضف المواقع التي تحتاجها في «مواقع أخرى». |
| بيلاروس | TikTok ووسائل الإعلام المستقلة (Zerkalo وNasha Niva وSvaboda وBelsat و…) | بعض الوسائل تغيّر عناوينها كثيرًا؛ أضف العنوان الحالي في «مواقع أخرى». |

في البلدان الأخرى يأتي Discord أولًا. ينقصك موقع أو بلد؟ افتح issue أو pull request لملف `strategies/domains.json`.

## Windows

### البدء

1. **نزّل وشغّل** `dpimech-setup-<version>.exe` من [Releases](https://github.com/halilkhrmn/dpimech/releases).
   يطلب Windows الإذن مرة واحدة؛ بعدها تبدأ خدمة الخلفية مع الحاسوب تلقائيًا.
2. **افتح DPIMech** واختر **فقط اجعله يعمل**. اختر المواقع، وأين يعمل (بعض التطبيقات فقط، أو الحاسوب
   بأكمله، أو وكيل) واضغط **ابدأ**. يثبّت DPIMech ما يحتاجه، ويختبر الإعدادات على اتصالك ويفعّل أفضلها.
3. هذا كل شيء. لاحقًا يمكنك الضغط على **إعداد سريع** لإضافة ملف شخصي آخر، أو إيقاف *الوضع السهل* من
   الإعدادات لرؤية المحركات ومختبر الإستراتيجيات والسجلات.

### المحركات على Windows

| المحرك | فيمَ يتميّز | يحتاج إلى |
|---|---|---|
| **ByeDPI** | وكيل محلي. بسيط وآمن. مع ProxiFyre يمرّر التطبيقات المختارة فقط. | لا شيء إضافي |
| **ProxiFyre** *(مساعد)* | يمرّر التطبيقات التي تختارها فقط عبر ByeDPI. | برنامج تشغيل **Windows Packet Filter** (يُثبَّت من صفحة المحركات؛ قد تلزم إعادة التشغيل) |
| **zapret (winws)** | يعمل على الحاسوب بأكمله، حتى لـ UDP (صوت Discord وQUIC). الخيار الأقوى. | يستخدم WinDivert (مضمّن). يعمل عبر خدمة DPIMech بصلاحيات المسؤول. |
| **GoodbyeDPI** | الأداة الكلاسيكية للحاسوب بأكمله. بسيطة، لكنها لم تعد تُطوَّر بنشاط. | مثل السابق. |

لا يمكن تشغيل إلا محرك WinDivert **واحد** (zapret أو GoodbyeDPI) في كل مرة. بعض برامج مكافحة الفيروسات
تصنّف WinDivert خطأً؛ ينزّله DPIMech فقط من الإصدارات الرسمية للمحركات.

## Linux

### المحركات على Linux

| المحرك | الوضع | يحتاج إلى |
|---|---|---|
| **zapret (nfqws)** | الحاسوب بأكمله. يضيف DPIMech قواعد nftables الخاصة به أثناء التشغيل ويزيلها بعده. | `nftables` ووحدتا النواة `nft_queue` و`nfnetlink_queue` (قياسيتان في معظم التوزيعات) |
| **zapret (tpws)** | الحاسوب بأكمله، أو بعض التطبيقات فقط، أو وكيل SOCKS محلي | لا شيء إضافي (cgroup v2 لوضع «بعض التطبيقات فقط») |
| **ByeDPI** | بعض التطبيقات فقط، أو وكيل SOCKS محلي | لا شيء إضافي (cgroup v2 لوضع «بعض التطبيقات فقط») |
| **SpoofDPI** | بعض التطبيقات فقط، أو وكيل SOCKS محلي. يمكنه حل الأسماء عبر HTTPS ‏(DoH)، ما يتجاوز حجب DNS أيضًا | لا شيء إضافي |

### التثبيت

نزّل من [Releases](https://github.com/halilkhrmn/dpimech/releases):

| نظامك | الملف | التثبيت |
|---|---|---|
| Ubuntu وDebian وMint وPop!_OS | `dpimech_<version>_amd64.deb` | `sudo apt install ./dpimech_*_amd64.deb` |
| Fedora | لا حاجة للتنزيل: [مستودع COPR](https://copr.fedorainfracloud.org/coprs/halilkahraman/DPIMech/) | انظر أدناه |
| Arch وCachyOS وEndeavourOS وManjaro | `dpimech-bin-<version>-1-x86_64.pkg.tar.zst` | `sudo pacman -U ./dpimech-bin-*.pkg.tar.zst` |
| openSUSE وRHEL | `dpimech-<version>-1.x86_64.rpm` | `sudo dnf install ./dpimech-*.x86_64.rpm` |
| أي توزيعة أخرى | `DPIMech-<version>-x86_64.AppImage` | `chmod +x DPIMech-*.AppImage` ثم شغّله |

**Fedora** — أضف المستودع مرة واحدة، فتصل التحديثات مع بقية النظام (`sudo dnf upgrade`):

</div>

```sh
sudo dnf copr enable halilkahraman/DPIMech
sudo dnf install dpimech
```

<div dir="rtl">

**Arch وCachyOS وEndeavourOS وManjaro** — حزمة Arch ‏(`dpimech-bin`) جاهزة، لكنها ليست على AUR بعد: لا يقبل AUR
حاليًا تسجيل أعضاء جدد. الحزمة نفسها مرفقة بكل إصدار: نزّل `dpimech-bin-<version>-1-x86_64.pkg.tar.zst` من
[Releases](https://github.com/halilkhrmn/dpimech/releases/latest) وثبّتها (يجلب pacman الاعتماديات، وينبّهك DPIMech عند صدور إصدار جديد):

</div>

```sh
sudo pacman -U ./dpimech-bin-*-x86_64.pkg.tar.zst
```

<div dir="rtl">

الحزم (`.deb` و`.rpm` وCOPR وArch) تثبّت خدمة الخلفية وتشغّلها أيضًا. مع AppImage، افتح DPIMech واضغط
**تثبيت الخدمة** مرة واحدة (تطلب كلمة المرور). في GNOME تحتاج أيقونة شريط المهام إلى إضافة
*AppIndicator and KStatusNotifierItem Support* (مضمّنة في Ubuntu).

### البناء من المصدر

يحتاج إلى [Rust](https://rustup.rs) و`systemd` وحزم تطوير GTK 3 وAppIndicator (في Debian/Ubuntu:
`sudo apt install build-essential libgtk-3-dev libayatana-appindicator3-dev libxkbcommon-x11-0`).

</div>

```sh
git clone https://github.com/halilkhrmn/dpimech && cd dpimech
cargo build --release
sudo target/release/dpimech-service install   # background service (systemd unit "dpimech")
target/release/dpimech                         # the window
```

<div dir="rtl">

## macOS (معاينة مبكرة)

نزّل `DPIMech-<version>.dmg` من [Releases](https://github.com/halilkhrmn/dpimech/releases) واسحب DPIMech
إلى مجلد التطبيقات. التطبيق غير موقّع بعد: في أول تشغيل انقر عليه بزر الفأرة الأيمن واختر **Open**. ثم
اضغط **تثبيت الخدمة** مرة واحدة. حاليًا لا يدعم macOS إلا **وكيل SOCKS محليًا** (tpws من zapret)؛ اضبط
`127.0.0.1:<port>` في متصفحك أو تطبيقك.

## من المفيد معرفته

- **الألعاب:** على Windows، قد ترفض الألعاب ذات أنظمة مكافحة الغش الصارمة (مثل Valorant أو FACEIT) التشغيل
  أو تقطع اتصالك أثناء عمل محرك للحاسوب بأكمله. أوقف ذلك الملف الشخصي قبل اللعب.
- **أداة DPI واحدة في كل مرة:** إذا عملت أداة تجاوز أخرى (GoodbyeDPI أو zapret أو ByeDPI Manager أو …) بجانب
  DPIMech، تتعطل الاتصالات بطرق غريبة. ينبّهك DPIMech عندما يرى واحدة.
- **ما زال محجوبًا؟ افحص DNS.** بعض المزودين يحجبون المواقع أيضًا بإعطاء عنوان خاطئ لأسمائها؛ لا يستطيع أي
  محرك إصلاح ذلك. ينبّهك مختبر الإستراتيجيات عندما تختلف إجابات DNS لديك عن 1.1.1.1. عندها اضبط DNS على
  `1.1.1.1` / `1.0.0.1` (أو فعّل «DNS over HTTPS» في النظام أو المتصفح).
- **السجلات:** الإعدادات ← *السجل*. عندما يتوقف ملف شخصي بسبب خطأ، يُحفظ السجل الأخير في ملف تلقائيًا.
  *السجل المفصّل* يسجّل كل اتصال أيضًا؛ أرفق الملف عند طلب المساعدة.

## هل هو آمن؟

- المحركات تغيّر شكل طلباتك أنت فقط.
- تعمل خدمة الخلفية بصلاحيات المسؤول لأن بعض المحركات تحتاجها. وهي لا تشغّل المحركات إلا من مجلدها المحمي،
  وترفض الإعدادات التي قد تكتب ملفات أو تقرأ ملفات خاصة.
- اكتشاف مزود الإنترنت اختياري، ولا يحدث إلا عند الضغط على *اكتشف مزودي*، ويرسل عنوان IP العام إلى `ipwho.is`.

## كيف يُصنع

يُطوَّر DPIMech بمساعدة الذكاء الاصطناعي (Claude). تُختبر الميزات يدويًا قبل اعتبارها مكتملة؛ ويسجّل
[docs/PROGRESS.md](docs/PROGRESS.md) ما الذي اختُبر وكيف.

## شكر

DPIMech يدير هذه المشاريع فقط؛ العمل الحقيقي من صنعها:
[ByeDPI](https://github.com/hufrea/byedpi) ·
[zapret](https://github.com/bol-van/zapret) ·
[GoodbyeDPI](https://github.com/ValdikSS/GoodbyeDPI) ·
[WinDivert](https://github.com/basil00/WinDivert) ·
[ProxiFyre](https://github.com/wiresock/proxifyre) ·
[Windows Packet Filter](https://github.com/wiresock/ndisapi).
الأيقونات: [Fluent UI System Icons](https://github.com/microsoft/fluentui-system-icons) ‏(MIT).
صُنع باستخدام [Slint](https://slint.dev).

## الترخيص

DPIMech برنامج حر بموجب [رخصة GNU العمومية العامة الإصدار 3 أو أحدث](LICENSE): يمكنك استخدامه ومشاركته
وتعديله، وكل ما توزّعه مبنيًا عليه يجب أن يبقى مفتوحًا بالشروط نفسها. المحركات التي ينزّلها تحتفظ بتراخيصها.

تريد المساعدة؟ انظر [CONTRIBUTING.md](CONTRIBUTING.md).

</div>
