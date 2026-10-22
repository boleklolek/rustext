#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-0.2.0}"
VERSION="${VERSION#v}"
ARCH="amd64"
PKG_NAME="rustext_${VERSION}_${ARCH}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BUILD_DIR="${REPO_ROOT}/target/package/linux/${PKG_NAME}"

BIN_PATH="${REPO_ROOT}/target/release/rustext"
if [ ! -f "$BIN_PATH" ]; then
    echo "[1/3] Compilando Rustext en modo release..."
    cargo build --release --locked --bin rustext --manifest-path "${REPO_ROOT}/Cargo.toml"
fi

echo "[2/3] Preparando estructura del paquete Debian (${PKG_NAME})..."
rm -rf "$BUILD_DIR"
mkdir -p "${BUILD_DIR}/DEBIAN"
mkdir -p "${BUILD_DIR}/usr/bin"
mkdir -p "${BUILD_DIR}/usr/share/applications"
mkdir -p "${BUILD_DIR}/usr/share/icons/hicolor/scalable/apps"
mkdir -p "${BUILD_DIR}/usr/share/rustext/themes"
mkdir -p "${BUILD_DIR}/usr/share/doc/rustext"

cp "$BIN_PATH" "${BUILD_DIR}/usr/bin/rustext"
chmod 0755 "${BUILD_DIR}/usr/bin/rustext"

cp "${REPO_ROOT}/assets/rustext.desktop" "${BUILD_DIR}/usr/share/applications/rustext.desktop"
chmod 0644 "${BUILD_DIR}/usr/share/applications/rustext.desktop"

cp "${REPO_ROOT}/assets/logo.svg" "${BUILD_DIR}/usr/share/icons/hicolor/scalable/apps/rustext.svg"
chmod 0644 "${BUILD_DIR}/usr/share/icons/hicolor/scalable/apps/rustext.svg"

cp "${REPO_ROOT}/assets/themes/"*.theme "${BUILD_DIR}/usr/share/rustext/themes/"
chmod 0644 "${BUILD_DIR}/usr/share/rustext/themes/"*.theme

cp "${REPO_ROOT}/README.md" "${REPO_ROOT}/README_EN.md" "${REPO_ROOT}/MANUAL.md" "${REPO_ROOT}/MANUAL_TEMAS_Y_EXTENSIONES.md" "${REPO_ROOT}/LICENSE" "${BUILD_DIR}/usr/share/doc/rustext/"
chmod 0644 "${BUILD_DIR}/usr/share/doc/rustext/"*

INSTALLED_SIZE=$(du -sk "${BUILD_DIR}/usr" | cut -f1)

cat > "${BUILD_DIR}/DEBIAN/control" <<EOF
Package: rustext
Version: ${VERSION}
Section: editors
Priority: optional
Architecture: ${ARCH}
Installed-Size: ${INSTALLED_SIZE}
Depends: libc6, libx11-6
Maintainer: boleklolek <bolek.lolek@gmail.com>
Homepage: https://github.com/boleklolek/rustext
Description: Ultra-fast, zero-dependency native code editor written in 100% pure Rust
 Rustext is a native GUI (Wayland/X11) and TUI text and code editor built
 exclusively with the Rust standard library (std), featuring a Piece Table
 buffer, parallel multi-cursor editing, live Git integration, and a full LSP suite.
EOF
chmod 0644 "${BUILD_DIR}/DEBIAN/control"

DEB_OUT="${REPO_ROOT}/target/package/linux/${PKG_NAME}.deb"
echo "[3/3] Construyendo paquete .deb..."
dpkg-deb --root-owner-group --build "$BUILD_DIR" "$DEB_OUT"
echo "Paquete creado exitosamente: ${DEB_OUT}"
