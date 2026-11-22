#!/usr/bin/env bash
set -euo pipefail
pacman -Syu --noconfirm --needed base-devel
useradd --create-home builder
printf 'builder ALL=(root) NOPASSWD: /usr/bin/pacman\n' > /etc/sudoers.d/builder
cp /recipe/PKGBUILD /build/
chown -R builder:builder /build
cd /build
sudo --preserve-env=MUSICOS_VERSION,MUSICOS_APPIMAGE_SHA256 -u builder makepkg --syncdeps --noconfirm
pacman -U --noconfirm ./*.pkg.tar.zst
pacman -Qkk musicos
test -x /usr/bin/musicos
test -x /opt/musicos/AppRun
