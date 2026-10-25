#!/usr/bin/env bash
# Build script: compiles Run Run my Znak and packages it as a .deb
# Run it from the project root (the folder with Cargo.toml, img/, music/, sfx/...)

set -euo pipefail

PKG="run-run-my-znak"
VERSION="0.1.0"
ARCH="$(dpkg --print-architecture)"
MAINTAINER="Stormwindsky <Stormwindsky@proton.me>"
DESCRIPTION_SHORT="A Dino Run clone titled Run Run my Znak"

# --- Sanity checks ---------------------------------------------------------
for item in Cargo.toml img music sfx font.otf credits.txt icon.png; do
    if [ ! -e "$item" ]; then
        echo "ERROR: '$item' not found. Run this script from the project root."
        exit 1
    fi
done

# --- 1. Compile ------------------------------------------------------------
echo ">>> Building release binary..."
cargo build --release

# --- 2. Prepare package tree ----------------------------------------------
ROOT="build_deb/${PKG}_${VERSION}_${ARCH}"
rm -rf "build_deb"
mkdir -p "$ROOT/DEBIAN"
mkdir -p "$ROOT/usr/bin"
mkdir -p "$ROOT/usr/lib/$PKG"
mkdir -p "$ROOT/usr/share/$PKG"
mkdir -p "$ROOT/usr/share/applications"
mkdir -p "$ROOT/usr/share/pixmaps"
mkdir -p "$ROOT/usr/share/doc/$PKG"

# Game binary
cp "target/release/$PKG" "$ROOT/usr/lib/$PKG/$PKG"
strip "$ROOT/usr/lib/$PKG/$PKG" || true

# Game assets (the game loads them with relative paths)
cp -r img music sfx "$ROOT/usr/share/$PKG/"
cp font.otf credits.txt "$ROOT/usr/share/$PKG/"

# Launcher: moves into the assets folder, then starts the game
cat > "$ROOT/usr/bin/$PKG" <<'EOF'
#!/bin/sh
cd /usr/share/run-run-my-znak || exit 1
exec /usr/lib/run-run-my-znak/run-run-my-znak "$@"
EOF

# Icon
cp icon.png "$ROOT/usr/share/pixmaps/$PKG.png"

# Menu entry
cat > "$ROOT/usr/share/applications/$PKG.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=Run Run my Znak
Comment=A Dino Run clone titled Run Run my Znak
Exec=run-run-my-znak
Icon=run-run-my-znak
Terminal=false
Categories=Game;ArcadeGame;
EOF

# Copyright file
cat > "$ROOT/usr/share/doc/$PKG/copyright" <<'EOF'
Run Run my Znak
Code: MIT-0
Sprites: CC0 1.0 (made by Stormwindsky)
Sounds, music and font: see credits.txt
EOF

# --- 3. Control file -------------------------------------------------------
INSTALLED_SIZE="$(du -sk "$ROOT" | cut -f1)"

cat > "$ROOT/DEBIAN/control" <<EOF
Package: $PKG
Version: $VERSION
Section: games
Priority: optional
Architecture: $ARCH
Installed-Size: $INSTALLED_SIZE
Depends: libc6, libasound2t64 | libasound2, libgl1, libx11-6, libxi6, xdg-utils
Maintainer: $MAINTAINER
Description: $DESCRIPTION_SHORT
 A Dino Run style game where Znak must jump over obstacles and use a shield.
EOF

# --- 4. Permissions + build .deb ------------------------------------------
find "$ROOT" -type d -exec chmod 755 {} +
find "$ROOT" -type f -exec chmod 644 {} +
chmod 755 "$ROOT/usr/bin/$PKG" "$ROOT/usr/lib/$PKG/$PKG"

dpkg-deb --build --root-owner-group "$ROOT"

mv "build_deb/${PKG}_${VERSION}_${ARCH}.deb" .
echo
echo ">>> Done: ${PKG}_${VERSION}_${ARCH}.deb"
echo ">>> Install with: sudo apt install ./${PKG}_${VERSION}_${ARCH}.deb"
