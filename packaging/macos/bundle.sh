#!/bin/bash
# Build dist/Spelly.app and dist/Spelly-macOS.dmg (universal: Apple silicon + Intel).
set -euo pipefail
cd "$(dirname "$0")/../.."

# ARCHS="aarch64-apple-darwin" ./bundle.sh builds a single-arch app (faster, for testing).
ARCHS=${ARCHS:-"aarch64-apple-darwin x86_64-apple-darwin"}
rustup target add $ARCHS >/dev/null
for t in $ARCHS; do
  cargo build --release -p spelly-mac -p spelly-ui --target "$t"
done
bin() { for t in $ARCHS; do echo "target/$t/release/$1"; done; }

APP=dist/Spelly.app
rm -rf dist/Spelly.app dist/Spelly-macOS.dmg && mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
# Spelly = the background app (keyboard + menu bar). spelly-ui = the window it opens.
lipo -create -output "$APP/Contents/MacOS/Spelly" $(bin spelly-mac)

# The window is its own little app inside Spelly.app, so it gets a Dock icon and Cmd-Tab
# (the main app is menu-bar only). Opening it twice just brings it forward.
WIN="$APP/Contents/Helpers/Spelly Window.app"
mkdir -p "$WIN/Contents/MacOS" "$WIN/Contents/Resources"
lipo -create -output "$WIN/Contents/MacOS/spelly-ui" $(bin spelly-ui)

# icon
ICONSET=$(mktemp -d)/icon.iconset && mkdir -p "$ICONSET"
for s in 16 32 64 128 256 512; do
  sips -z $s $s assets/icon.png --out "$ICONSET/icon_${s}x${s}.png" >/dev/null
  sips -z $((s*2)) $((s*2)) assets/icon.png --out "$ICONSET/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/Spelly.icns"
cp "$APP/Contents/Resources/Spelly.icns" "$WIN/Contents/Resources/Spelly.icns"

VERSION=$(grep -m1 '^version' crates/spelly-mac/Cargo.toml | cut -d'"' -f2)
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>Spelly</string>
  <key>CFBundleDisplayName</key><string>Spelly</string>
  <key>CFBundleIdentifier</key><string>dev.spelly.app</string>
  <key>CFBundleExecutable</key><string>Spelly</string>
  <key>CFBundleIconFile</key><string>Spelly</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>LSUIElement</key><true/>
</dict></plist>
PLIST

cat > "$WIN/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>Spelly</string>
  <key>CFBundleDisplayName</key><string>Spelly</string>
  <key>CFBundleIdentifier</key><string>dev.spelly.window</string>
  <key>CFBundleExecutable</key><string>spelly-ui</string>
  <key>CFBundleIconFile</key><string>Spelly</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST

# Ad-hoc signature: keeps the Accessibility permission stable between launches.
codesign --force --deep --sign - "$APP"

STAGE=$(mktemp -d)
cp -R "$APP" "$STAGE/" && ln -s /Applications "$STAGE/Applications"
hdiutil create -volname Spelly -srcfolder "$STAGE" -ov -format UDZO dist/Spelly-macOS.dmg >/dev/null
echo "dist/Spelly.app  dist/Spelly-macOS.dmg"
