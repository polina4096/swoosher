#!/bin/bash
set -euo pipefail

VERSION="${1:?Usage: $0 <version>}"
TARGET="aarch64-apple-darwin"
APP="target/swoosher.app"

# Build
cargo build --release --target "$TARGET"

# Clean previous .app
rm -rf "$APP" target/swoosher.app.zip

# Create .app structure
mkdir -p "$APP/Contents/MacOS"

cp "target/$TARGET/release/swoosher" "$APP/Contents/MacOS/swoosher"

cat > "$APP/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>
  <string>swoosher</string>
  <key>CFBundleDisplayName</key>
  <string>swoosher</string>
  <key>CFBundleIdentifier</key>
  <string>fish.stupid.swoosher</string>
  <key>CFBundleVersion</key>
  <string>${VERSION}</string>
  <key>CFBundleShortVersionString</key>
  <string>${VERSION}</string>
  <key>CFBundleExecutable</key>
  <string>swoosher</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>LSUIElement</key>
  <true/>
</dict>
</plist>
EOF

cd target
zip -r swoosher.app.zip swoosher.app
echo "Packaged: target/swoosher.app.zip"
