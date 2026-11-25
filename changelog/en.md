# Changelog

User-facing changes per version. The app shows the new entries once after an update ("What's new"),
and the release page on GitHub uses the English text. Translations: tr.md, ru.md, fa.md, ar.md.

## 0.4.7
- DPIMech remembers which strategy works on each network (recognised by your router, nothing is sent anywhere) and switches to it when you come back. On a new network it checks the profiles' sites; if they do not open, it tests strategies by itself and uses the best one (ByeDPI, SpoofDPI, tpws).
- When the background service is not running, the window says so with a "Start the service" button, and profiles, engines and the Strategy Lab wait until it runs again.
- Settings → Network also names the DNS service your router forwards to (for example Cloudflare set in the router).
- Starting a profile from a shortcut shows a plain "Starting …" notification without a progress bar; only a failure adds another one.
- DPIMech also warns about software that can get in the way of bypassing (AdGuard, Killer and SmartByte network services, ZoneAlarm, VPN clients, Cloudflare WARP).

## 0.4.6
- Without internet the engine is no longer restarted again and again. DPIMech waits, says to check your network (on the card and in one notification) and carries on by itself once the connection is back.
- The tray icon blinks amber while a profile is connecting or a test is running.
- If DPIMech or its background service crashes, the next start says so and offers to send what was recorded.
- A profile shortcut no longer opens a small window of its own: one system notification shows the progress (turning on, opening the app) and the result.
- Settings → Network shows the DNS servers your computer uses right now, and whether they come from your network or were set by hand; problem reports include them.
- Optional: DPIMech can switch your DNS to a well-known public service (Cloudflare, Google, Quad9, AdGuard, OpenDNS, Yandex, …). It saves your previous settings and puts them back when you choose "Your network's own" or uninstall DPIMech. Windows and Linux (with systemd-resolved, encrypted where possible).

## 0.4.5
- Windows: "Restart and update" works again. In 0.4.3 and 0.4.4 it failed with "The handle is invalid" once a profile that routes apps had been stopped. If you see that error, restart the computer and update before stopping a profile, or run the installer from the release page.

## 0.4.4
- On Linux with Wayland (for example KDE Plasma), "Show DPIMech" in the tray now opens the window when DPIMech started in the tray; before, the window did not appear and an empty entry stayed in the taskbar.
- When DPIMech starts in the tray at sign-in, its window no longer opens because the tray icon came up a few seconds late.
- "Show DPIMech" also brings back a minimised window.

## 0.4.3
- Nothing runs while DPIMech is closed. "Start with service" is now "Start with DPIMech": the profile starts when DPIMech opens, and every profile stops when you quit.
- When the computer loses its internet connection, DPIMech no longer restarts the engine or says the setting stopped working; the card says there is no connection.
- The service log stays small: what an engine writes per connection is only kept with the detailed log on.
- The update check tries again soon when the network is not up yet after the computer starts.
- Per-app routing now closes ProxiFyre cleanly instead of ending it abruptly.
- The Logs page has a filter, the warning about another DPI tool can be hidden and is logged once, and the first time you close the window DPIMech says it keeps running in the tray.

## 0.4.2
- Quitting DPIMech now stops its profiles. Only a profile with "Start with service" keeps running while DPIMech is closed, from when the computer starts.
- When a profile starts with the service, DPIMech starts in the tray when you sign in, so you always see what runs.

## 0.4.1
- The new logo now also shows on the taskbar, in the Start menu, in the Linux app menu and on profile shortcuts made before the update; after updating to 0.4.0 they kept the old one.

## 0.4.0
- A new logo: a chameleon drawn by hand by Kim De Vries.
- A simpler profile editor: one "Keep it working" switch looks after the connection; the port, check settings and strategy arguments are under "Advanced settings".
- Settings are shorter: DPIMech always starts in the tray when it starts with your computer, and the "Detailed log" switch is now on the Logs page.

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
