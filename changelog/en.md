# Changelog

User-facing changes per version. The app shows the new entries once after an update ("What's new"),
and the release page on GitHub uses the English text. Translations: tr.md, ru.md, fa.md, ar.md.

## 0.3.1
- Arch, CachyOS, EndeavourOS and Manjaro: an Arch package (`dpimech-bin`) is now on every release page; install it with `sudo pacman -U`.
- On Linux without libxkbcommon-x11 DPIMech no longer crashes at start: it tells you which package to install.
- Every package is now installed and started on a clean system before a release is published.

## 0.3.0
- Persian (فارسی) and Arabic (العربية) interface.
- Sites blocked in your country come first and are preselected in the setup wizard: Turkey, Russia, Iran, Kazakhstan, Belarus and Egypt.
- New sites to unblock: Telegram, WhatsApp, Facebook, Signal, Viber, LinkedIn, TikTok, SoundCloud, Imgur, and independent news sites of Belarus and Egypt.
- The list of sites updates by itself, without a new version of DPIMech.
- This window: after an update, DPIMech shows what changed.
