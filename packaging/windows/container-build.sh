#!/bin/bash
# Runs INSIDE the build container (see build-in-container.sh): compile, then package.
set -euo pipefail

# Build from a scratch copy so patching GPUI below never touches your Cargo.lock or sources.
mkdir -p /build && tar -C /work --exclude=./.git --exclude=./harper/.git --exclude=./target --exclude=./dist -cf - . | tar -C /build -xf -
cd /build

# GPUI's Windows build script only works on a Windows host: patch a throw-away copy of it.
cargo fetch --target x86_64-pc-windows-msvc
SRC=$(ls -d "$CARGO_HOME"/registry/src/*/gpui-pre-windows-* | head -1)
rm -rf /opt/gpui-windows && cp -r "$SRC" /opt/gpui-windows
python3 /work/packaging/windows/patch-gpui-windows.py /opt/gpui-windows

cargo xwin build --release --target x86_64-pc-windows-msvc -p spelly-win -p spelly-ui \
  --config 'patch.crates-io.gpui-pre-windows.path="/opt/gpui-windows"'
cd /work

OUT=/target/x86_64-pc-windows-msvc/release
VERSION=$(grep -m1 '^version' crates/spelly-win/Cargo.toml | cut -d'"' -f2)
ISCC=$(ls "$WINEPREFIX"/drive_c/Program\ Files*/Inno\ Setup\ 6/ISCC.exe | head -1)

mkdir -p dist
xvfb-run -a wine "$ISCC" "/DAppVersion=$VERSION" '/DBinDir=Z:\target\x86_64-pc-windows-msvc\release' 'Z:\work\packaging\windows\spelly.iss'
wineserver -w || true

# Handy for testing without the installer
cp "$OUT/spelly.exe" "$OUT/spelly-ui.exe" dist/ 2>/dev/null || true
ls -la dist/
