#!/bin/sh
# Shared by the deb postinst and the rpm %post: enable and (re)start the packaged service.
# A source install (`dpimech-service install`) writes its own unit to /etc, which would
# shadow the packaged one in /usr/lib, so it is removed first. Profiles and engines in
# /var/lib/dpimech are kept either way.
if [ -f /etc/systemd/system/dpimech.service ] && grep -q /usr/local/lib/dpimech /etc/systemd/system/dpimech.service; then
    systemctl disable --now dpimech.service >/dev/null 2>&1 || true
    rm -f /etc/systemd/system/dpimech.service
    rm -rf /usr/local/lib/dpimech
fi
# Only on a running systemd (not in containers or chroots used to build images).
if [ -d /run/systemd/system ]; then
    systemctl daemon-reload || true
    systemctl enable dpimech.service >/dev/null 2>&1 || true
    systemctl restart dpimech.service || true
fi
