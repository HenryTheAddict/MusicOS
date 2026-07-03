#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
appdir="$repo_root/out/x86_64/AppDir"

echo "Creating AppDir structure..."
mkdir -p "$appdir/usr/bin"
mkdir -p "$appdir/usr/share/applications"
mkdir -p "$appdir/usr/share/icons/hicolor/256x256/apps"

# Copy binary
cp -f "$repo_root/target/x86_64-unknown-linux-gnu/release/musicos-init" "$appdir/usr/bin/musicos-init"

# Copy icon
cp -f "$repo_root/assets/boot splash.png" "$appdir/usr/share/icons/hicolor/256x256/apps/musicos.png"
cp -f "$repo_root/assets/boot splash.png" "$appdir/musicos.png"

# Create Desktop entry
cat >"$appdir/musicos.desktop" <<EOF
[Desktop Entry]
Name=MusicOS
Exec=musicos-init
Icon=musicos
Type=Application
Terminal=true
Categories=Audio;
EOF
cp -f "$appdir/musicos.desktop" "$appdir/usr/share/applications/musicos.desktop"

# Create AppRun script
cat >"$appdir/AppRun" <<'EOF'
#!/bin/sh
SELF=$(readlink -f "$0")
HERE=$(dirname "$SELF")
export LD_LIBRARY_PATH="$HERE/usr/lib:$HERE/lib:$LD_LIBRARY_PATH"
exec "$HERE/usr/bin/musicos-init" "$@"
EOF
chmod +x "$appdir/AppRun"

# Run AppImage compiler inside Colima VM!
echo "Running appimagetool inside Colima VM..."
colima ssh -- bash -lc "
  set -euo pipefail
  cd /Users/henryperzinski/Developer/MusicOS

  # Download appimagetool if not exists
  if [ ! -f vendor/appimagetool ]; then
    echo 'Downloading appimagetool...'
    curl -L -o vendor/appimagetool https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
    chmod +x vendor/appimagetool
  fi

  # Extract appimagetool
  if [ ! -d vendor/squashfs-root ]; then
    echo 'Extracting appimagetool...'
    cd vendor
    ./appimagetool --appimage-extract
    cd ..
  fi

  # Package AppImage
  echo 'Compiling MusicOS-x86_64.AppImage...'
  ARCH=x86_64 ./vendor/squashfs-root/AppRun out/x86_64/AppDir out/x86_64/MusicOS-x86_64.AppImage
"

echo "MusicOS AppImage package completed successfully!"
file "$repo_root/out/x86_64/MusicOS-x86_64.AppImage"
