#!/usr/bin/env bash
# Builds release binaries and the Linux packages into target/linux/:
#   dpimech_<version>_amd64.deb, dpimech-<version>-1.x86_64.rpm, DPIMech-<version>-x86_64.AppImage
# Needs: cargo-deb, cargo-generate-rpm (cargo install cargo-deb cargo-generate-rpm), dpkg-dev,
# libayatana-appindicator3-1 (bundled into the AppImage),
# and network access the first time (appimagetool is downloaded into target/).
set -euo pipefail
cd "$(dirname "$0")/.."

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
out=target/linux
rm -rf "$out"
mkdir -p "$out"

cargo build --release --workspace
# One unit for packages and `dpimech-service install`: the binary prints it.
target/release/dpimech-service unit --exe /usr/lib/dpimech/dpimech-service > target/release/dpimech.service

cargo deb -p dpimech-gui --no-build --output "$out/"
cargo generate-rpm -p crates/gui --output "$out/"

# AppImage: both binaries in one file. The window offers to install the service (pkexec).
appdir=target/AppDir
rm -rf "$appdir"
mkdir -p "$appdir/usr/bin" "$appdir/usr/share/applications" "$appdir/usr/share/icons/hicolor/256x256/apps"
cp target/release/dpimech target/release/dpimech-service "$appdir/usr/bin/"
# The tray library and the parts of it that desktops often lack (Arch, CachyOS). GTK and glib come
# from the system. The GUI loads these only when the system lacks them (src/bundled.rs).
mkdir -p "$appdir/usr/lib"
for lib in libayatana-appindicator3.so.1 libayatana-indicator3.so.7 libayatana-ido3-0.4.so.0 \
    libdbusmenu-glib.so.4 libdbusmenu-gtk3.so.4; do
    path=$(/sbin/ldconfig -p | awk -v l="$lib" '$1 == l && /x86-64/ { print $NF; exit }')
    [ -n "$path" ] || { echo "missing $lib (install libayatana-appindicator3-1)" >&2; exit 1; }
    cp -L "$path" "$appdir/usr/lib/$lib"
done
cp packaging/linux/dpimech.desktop "$appdir/"
cp packaging/linux/dpimech.desktop "$appdir/usr/share/applications/"
cp crates/gui/assets/dpimech.png "$appdir/dpimech.png"
cp crates/gui/assets/dpimech.png "$appdir/usr/share/icons/hicolor/256x256/apps/"
cat > "$appdir/AppRun" <<'APPRUN'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/dpimech" "$@"
APPRUN
chmod 755 "$appdir/AppRun"

tool=${APPIMAGETOOL:-target/appimagetool-x86_64.AppImage}
if [ ! -x "$tool" ]; then
    curl -fsSL -o "$tool" https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
    chmod 755 "$tool"
fi
# --appimage-extract-and-run: works without FUSE (CI runners, containers).
ARCH=x86_64 "$tool" --appimage-extract-and-run "$appdir" "$out/DPIMech-$version-x86_64.AppImage"

ls -la "$out"
