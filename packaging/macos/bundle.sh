#!/bin/bash
# Build dist/Spelly.app and dist/Spelly-macOS.dmg (universal: Apple silicon + Intel).
set -euo pipefail
cd "$(dirname "$0")/../.."

rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
for t in aarch64-apple-darwin x86_64-apple-darwin; do
  cargo build --release -p spelly-mac --target "$t"
done

APP=dist/Spelly.app
rm -rf dist && mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
lipo -create -output "$APP/Contents/MacOS/Spelly" \
  target/aarch64-apple-darwin/release/spelly-mac target/x86_64-apple-darwin/release/spelly-mac

# icon
ICONSET=$(mktemp -d)/icon.iconset && mkdir -p "$ICONSET"
for s in 16 32 64 128 256 512; do
  sips -z $s $s assets/icon.png --out "$ICONSET/icon_${s}x${s}.png" >/dev/null
  sips -z $((s*2)) $((s*2)) assets/icon.png --out "$ICONSET/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/Spelly.icns"

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

# Ad-hoc signature: keeps the Accessibility permission stable between launches.
codesign --force --deep --sign - "$APP"

STAGE=$(mktemp -d)
cp -R "$APP" "$STAGE/" && ln -s /Applications "$STAGE/Applications"
hdiutil create -volname Spelly -srcfolder "$STAGE" -ov -format UDZO dist/Spelly-macOS.dmg >/dev/null
echo "dist/Spelly.app  dist/Spelly-macOS.dmg"
