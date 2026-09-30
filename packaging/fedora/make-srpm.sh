#!/bin/sh
# Builds dpimech-<version>-1.src.rpm into the directory given as $1 (default target/srpm):
# the source tree of a release tag plus every crate vendored, so the RPM build itself needs
# no network. Fedora COPR runs this through .copr/Makefile; it also works locally.
#
# Builds the newest v* tag, or DPIMECH_REF (any commit, branch or tag) when set.
set -eu
outdir=$(realpath -m "${1:-target/srpm}")
cd "$(dirname "$0")/../.."

ref=${DPIMECH_REF:-$(git tag --list 'v[0-9]*' --sort=-v:refname | head -n 1)}
ref=${ref:-HEAD}
version=$(git show "$ref:Cargo.toml" | sed -n 's/^version = "\(.*\)"/\1/p' | head -n 1)
echo "building dpimech $version from $ref"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
git archive --format=tar --prefix="dpimech-$version/" "$ref" | tar -x -C "$work"
git archive --format=tar.gz --prefix="dpimech-$version/" -o "$work/dpimech-$version.tar.gz" "$ref"
(
    cd "$work/dpimech-$version"
    cargo vendor --locked --quiet vendor > /dev/null
    tar -cJf "$work/dpimech-$version-vendor.tar.xz" vendor
)
sed "s/^Version:.*/Version:        $version/" packaging/fedora/dpimech.spec > "$work/dpimech.spec"
mkdir -p "$outdir"
rpmbuild -bs --define "_sourcedir $work" --define "_srcrpmdir $outdir" "$work/dpimech.spec"
