# Fedora / COPR package, built from source with vendored crates (no network during the build).
# packaging/fedora/make-srpm.sh fills in Version and makes both source archives.

# Release builds are stripped (Cargo.toml [profile.release]); there is nothing for debuginfo.
%global debug_package %{nil}

Name:           dpimech
Version:        0.0.0
Release:        1%{?dist}
Summary:        Open sites your internet provider blocks (DPI bypass manager)
# DPIMech itself; the bundled Rust crates are MIT / Apache-2.0 / BSD / Zlib / MPL-2.0 / Unicode.
License:        GPL-3.0-or-later
URL:            https://github.com/halilkhrmn/dpimech
Source0:        %{name}-%{version}.tar.gz
Source1:        %{name}-%{version}-vendor.tar.xz

ExclusiveArch:  x86_64 aarch64

BuildRequires:  cargo >= 1.85
BuildRequires:  rust >= 1.85
BuildRequires:  gcc
BuildRequires:  pkgconfig
BuildRequires:  gtk3-devel
BuildRequires:  libxdo-devel
BuildRequires:  libayatana-appindicator-gtk3-devel
BuildRequires:  openssl-devel
BuildRequires:  systemd-rpm-macros

Requires:       nftables
Requires:       libayatana-appindicator-gtk3
Requires:       libxkbcommon-x11
Recommends:     libnotify
%{?systemd_requires}

%description
Opens sites and apps that your internet provider blocks with DPI, by running well-known
open-source bypass engines (zapret, ByeDPI). Not a VPN: traffic still goes straight to the
sites. The window runs as your user; a small system service starts the engines.

%prep
%autosetup -n %{name}-%{version}
tar -xJf %{SOURCE1}
mkdir -p .cargo
cat > .cargo/config.toml <<'EOF'
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
EOF

%build
cargo build --release --locked --offline --workspace
# /usr/libexec is labelled bin_t, so SELinux runs the service like any other system daemon.
target/release/dpimech-service unit --exe %{_libexecdir}/dpimech/dpimech-service > dpimech.service

%install
install -Dm755 target/release/dpimech %{buildroot}%{_bindir}/dpimech
install -Dm755 target/release/dpimech-service %{buildroot}%{_libexecdir}/dpimech/dpimech-service
install -Dm755 packaging/linux/service-setup.sh %{buildroot}%{_libexecdir}/dpimech/service-setup.sh
install -Dm644 dpimech.service %{buildroot}%{_unitdir}/dpimech.service
install -Dm644 packaging/linux/dpimech.desktop %{buildroot}%{_datadir}/applications/dpimech.desktop
install -Dm644 crates/gui/assets/dpimech.png %{buildroot}%{_datadir}/icons/hicolor/256x256/apps/dpimech.png

%post
%{_libexecdir}/dpimech/service-setup.sh

%preun
%systemd_preun dpimech.service

%postun
%systemd_postun dpimech.service

%files
%license LICENSE
%doc README.md
%{_bindir}/dpimech
%dir %{_libexecdir}/dpimech
%{_libexecdir}/dpimech/dpimech-service
%{_libexecdir}/dpimech/service-setup.sh
%{_unitdir}/dpimech.service
%{_datadir}/applications/dpimech.desktop
%{_datadir}/icons/hicolor/256x256/apps/dpimech.png

%changelog
* Wed Sep 30 2026 Halil Kahraman <halilkahraman@yandex.com> - 0.2.4-1
- Stricter connection checks: the Strategy Lab confirms its best strategies, profiles check their sites at start
* Wed Sep 30 2026 Halil Kahraman <halilkahraman@yandex.com> - 0.2.3-1
- SpoofDPI engine, in-app updates, fix for quickly restarting a profile
* Wed Sep 30 2026 Halil Kahraman <halilkahraman@yandex.com> - 0.2.2-1
- More useful logs (detailed log option, DNS check), shortcut clean-up
* Wed Sep 30 2026 Halil Kahraman <halilkahraman@yandex.com> - 0.2.1-1
- Windows: the service installs again after the upgrade from 0.1.x
* Wed Sep 30 2026 Halil Kahraman <halilkahraman@yandex.com> - 0.2.0-1
- Release notes: https://github.com/halilkhrmn/dpimech/releases
