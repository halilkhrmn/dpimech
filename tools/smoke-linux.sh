#!/usr/bin/env bash
# Release smoke test: installs one Linux package on a clean system (a container in CI) and
# checks that it starts. Usage, as root: tools/smoke-linux.sh appimage|deb|rpm <file>
#
# Catches what unit tests cannot: a library the binary links but the distribution lacks
# (libxdo.so.3 on CachyOS, 0.2.7), a tray that cannot load, a GUI that panics at start.
# Needs on the test system: Xvfb, python3, and for the AppImage gtk3 and a font.
set -euo pipefail

kind=$1
file=$(readlink -f "$2")
work=$(mktemp -d)
fail() { echo "SMOKE FAIL: $*" >&2; exit 1; }

case "$kind" in
    appimage)
        chmod +x "$file"
        (cd "$work" && "$file" --appimage-extract >/dev/null)
        gui="$work/squashfs-root/usr/bin/dpimech"
        svc="$work/squashfs-root/usr/bin/dpimech-service"
        # Started the way users start it; the runtime needs no FUSE this way.
        run_gui=("$file" --appimage-extract-and-run)
        ;;
    deb)
        apt-get install -y "$file" >/dev/null
        gui=/usr/bin/dpimech
        svc=/usr/lib/dpimech/dpimech-service
        run_gui=("$gui")
        ;;
    rpm)
        dnf install -y "$file" >/dev/null
        gui=/usr/bin/dpimech
        svc=/usr/lib/dpimech/dpimech-service
        run_gui=("$gui")
        ;;
    *) fail "unknown package kind $kind" ;;
esac

echo "== libraries"
for bin in "$gui" "$svc"; do
    missing=$(ldd "$bin" | grep 'not found' || true)
    [ -z "$missing" ] || fail "$(basename "$bin") needs libraries this system lacks:"$'\n'"$missing"
done

echo "== service"
export DPIMECH_SOCKET="$work/dpimech.sock"
"$svc" run --data-dir "$work/data" >"$work/service.log" 2>&1 &
for _ in $(seq 50); do [ -S "$DPIMECH_SOCKET" ] && break; sleep 0.2; done
[ -S "$DPIMECH_SOCKET" ] || { cat "$work/service.log"; fail "the service did not open its socket"; }
python3 - "$DPIMECH_SOCKET" <<'PY' || fail "the service did not answer"
import json, socket, sys
s = socket.socket(socket.AF_UNIX)
s.settimeout(10)
s.connect(sys.argv[1])
f = s.makefile("rw")
f.write(json.dumps({"id": 1, "request": {"type": "hello", "client_version": "smoke"}}) + "\n")
f.flush()
while True:
    frame = json.loads(f.readline())
    if frame.get("kind") == "reply" and frame.get("id") == 1:
        print("service", frame["result"]["Ok"]["service_version"])
        break
PY

echo "== window"
Xvfb :99 -screen 0 1280x800x24 >/dev/null 2>&1 &
sleep 1
mkdir -p "$work/home" "$work/run"
chmod 700 "$work/run"
set +e
DISPLAY=:99 HOME="$work/home" XDG_RUNTIME_DIR="$work/run" timeout 10 "${run_gui[@]}" >"$work/gui.log" 2>&1
code=$?
set -e
cat "$work/gui.log"
# 124: still running when the timeout stopped it, which is what a working GUI does.
if [ "$code" -ne 124 ] && [ "$kind" = appimage ]; then
    # Tell a runtime problem from an app problem: start the extracted copy directly.
    echo "== window, extracted AppImage"
    DISPLAY=:99 HOME="$work/home" XDG_RUNTIME_DIR="$work/run" RUST_BACKTRACE=1 \
        timeout 10 "$work/squashfs-root/AppRun" 2>&1 || echo "exit code $?"
fi
[ "$code" -eq 124 ] || fail "the GUI exited with code $code"
if grep -E 'panicked|error while loading shared libraries|tray unavailable' "$work/gui.log"; then
    fail "the GUI reported a problem at start"
fi
echo "SMOKE OK: $kind $(basename "$file")"
