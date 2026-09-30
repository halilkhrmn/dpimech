#!/usr/bin/env bash
# Builds DPIMech.app (universal: Apple Silicon + Intel) and target/macos/DPIMech-<version>.dmg.
# Runs on macOS only (lipo, sips, iconutil, hdiutil). The app is not signed: on first start,
# right-click → Open (or System Settings → Privacy & Security → Open Anyway).
set -euo pipefail
cd "$(dirname "$0")/.."

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
out=target/macos
app="$out/DPIMech.app"
rm -rf "$out"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"

for target in aarch64-apple-darwin x86_64-apple-darwin; do
    rustup target add "$target" >/dev/null
    cargo build --release --workspace --target "$target"
done
for bin in dpimech dpimech-service; do
    lipo -create -output "$app/Contents/MacOS/$bin" \
        "target/aarch64-apple-darwin/release/$bin" "target/x86_64-apple-darwin/release/$bin"
done

# Icon: every size macOS asks for, from the lossless master.
iconset="$out/dpimech.iconset"
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
    sips -z $size $size crates/gui/assets/logo-source.png --out "$iconset/icon_${size}x${size}.png" >/dev/null
    sips -z $((size * 2)) $((size * 2)) crates/gui/assets/logo-source.png --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/dpimech.icns"
rm -rf "$iconset"

cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>DPIMech</string>
    <key>CFBundleDisplayName</key><string>DPIMech</string>
    <key>CFBundleIdentifier</key><string>io.github.dpimech</string>
    <key>CFBundleExecutable</key><string>dpimech</string>
    <key>CFBundleIconFile</key><string>dpimech</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$version</string>
    <key>CFBundleVersion</key><string>$version</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

# Disk image with an Applications shortcut for drag-and-drop install.
stage="$out/dmg"
mkdir -p "$stage"
cp -R "$app" "$stage/"
ln -s /Applications "$stage/Applications"
hdiutil create -volname DPIMech -srcfolder "$stage" -ov -format UDZO "$out/DPIMech-$version.dmg"
rm -rf "$stage"
ls -la "$out"
