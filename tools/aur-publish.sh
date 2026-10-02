#!/usr/bin/env bash
# Builds the Arch package dpimech-bin for a release, installs it, and with PKG_OUT=<dir> copies the
# package there (release.yml attaches it to the GitHub release: `sudo pacman -U` installs it).
# Runs in an Arch container as root: tools/aur-publish.sh <version> <deb> [--push]
# The .deb is the one the release publishes; makepkg uses it instead of downloading.
# --push publishes to the AUR; on hold (the AUR takes no new accounts, DECISIONS #48). It needs
# AUR_SSH_PRIVATE_KEY (the key added to the AUR account that owns dpimech-bin).
set -euo pipefail

version=$1
deb=$(readlink -f "$2")
push=${3:-}
root=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)

pacman -Syu --noconfirm --needed base-devel git openssh >/dev/null
id builder >/dev/null 2>&1 || useradd -m builder

sum=$(sha256sum "$deb" | cut -d' ' -f1)
pkg="$work/dpimech-bin"
mkdir -p "$pkg"
sed -e "s/@VERSION@/$version/" -e "s/@SHA256@/$sum/" "$root/packaging/aur/PKGBUILD" >"$pkg/PKGBUILD"
cp "$root/packaging/aur/dpimech-bin.install" "$pkg/"
cp "$deb" "$pkg/dpimech_${version}-1_amd64.deb"
chown -R builder "$work"

# makepkg runs as a normal user and cannot install the dependencies itself. Installing them
# here also checks that every name in depends= exists in Arch.
mapfile -t deps < <(bash -c 'source "$1"; printf "%s\n" "${depends[@]}"' _ "$pkg/PKGBUILD")
pacman -S --noconfirm --needed "${deps[@]}" >/dev/null
# Building checks the PKGBUILD and the checksum.
su builder -c "cd '$pkg' && makepkg --noconfirm --cleanbuild >/dev/null && makepkg --printsrcinfo >.SRCINFO" \
    || { echo "makepkg failed" >&2; exit 1; }
pacman -U --noconfirm "$pkg"/dpimech-bin-"$version"-1-x86_64.pkg.tar.* >/dev/null
test -x /usr/bin/dpimech && test -x /usr/lib/dpimech/dpimech-service
echo "built and installed dpimech-bin $version"
if [ -n "${PKG_OUT:-}" ]; then
    mkdir -p "$PKG_OUT"
    cp "$pkg"/dpimech-bin-"$version"-1-x86_64.pkg.tar.* "$PKG_OUT/"
fi

[ "$push" = "--push" ] || exit 0
[ -n "${AUR_SSH_PRIVATE_KEY:-}" ] || { echo "AUR_SSH_PRIVATE_KEY is not set; not publishing" >&2; exit 1; }
mkdir -p ~/.ssh
printf '%s\n' "$AUR_SSH_PRIVATE_KEY" >~/.ssh/aur
chmod 600 ~/.ssh/aur
ssh-keyscan -t ed25519 aur.archlinux.org >>~/.ssh/known_hosts 2>/dev/null
export GIT_SSH_COMMAND="ssh -i $HOME/.ssh/aur -o IdentitiesOnly=yes"
git clone -q ssh://aur@aur.archlinux.org/dpimech-bin.git "$work/aur"
cp "$pkg/PKGBUILD" "$pkg/.SRCINFO" "$pkg/dpimech-bin.install" "$work/aur/"
cd "$work/aur"
git add PKGBUILD .SRCINFO dpimech-bin.install
if git diff --cached --quiet; then
    echo "AUR already has dpimech-bin $version"
    exit 0
fi
git -c user.name="Halil Kahraman" -c user.email="halilkahraman@yandex.com" commit -qm "Update to $version"
git push -q origin HEAD:master
echo "published dpimech-bin $version to the AUR"
