#!/usr/bin/env bash
# Builds dist/MapCheck.app — a double-clickable macOS bundle with the whole
# app, the Rust/WASM core and the icon inside it.
set -euo pipefail

cd "$(dirname "$0")/.."

if [ "$(uname -s)" != "Darwin" ]; then
    echo "App bundles are a macOS format; use scripts/build-app.sh elsewhere." >&2
    exit 1
fi

if [ -d "$HOME/.cargo/bin" ]; then
    PATH="$HOME/.cargo/bin:$PATH"
fi

APP="dist/MapCheck.app"
CONTENTS="$APP/Contents"
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)

./scripts/build-app.sh

echo
echo "Assembling $APP"
rm -rf "$APP"
mkdir -p "$CONTENTS/MacOS" "$CONTENTS/Resources"

cp target/release/mapcheck "$CONTENTS/MacOS/mapcheck"
python3 scripts/make-icon.py "$CONTENTS/Resources/AppIcon.icns"

cat > "$CONTENTS/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>                  <string>MapCheck</string>
    <key>CFBundleDisplayName</key>           <string>MapCheck</string>
    <key>CFBundleIdentifier</key>            <string>com.github.jollytiki.mapcheck</string>
    <key>CFBundleExecutable</key>            <string>mapcheck</string>
    <key>CFBundleIconFile</key>              <string>AppIcon</string>
    <key>CFBundlePackageType</key>           <string>APPL</string>
    <key>CFBundleShortVersionString</key>    <string>$VERSION</string>
    <key>CFBundleVersion</key>               <string>$VERSION</string>
    <key>CFBundleInfoDictionaryVersion</key> <string>6.0</string>
    <key>LSMinimumSystemVersion</key>        <string>11.0</string>
    <key>NSHighResolutionCapable</key>       <true/>
    <!-- The browser is the interface, so the bundle itself stays out of the
         Dock. It exits on its own once the last tab closes. -->
    <key>LSUIElement</key>                   <true/>
</dict>
</plist>
PLIST

# Classic four-byte type/creator file. Harmless, and some tools still look.
printf 'APPL????' > "$CONTENTS/PkgInfo"

plutil -lint "$CONTENTS/Info.plist" > /dev/null

# `cp` carries extended attributes across, and codesign refuses to sign a
# bundle that has any ("resource fork, Finder information, or similar
# detritus not allowed").
xattr -cr "$APP"

# Ad-hoc signature. Enough for this Mac and for Gatekeeper to stop complaining
# about a missing signature; it is not a Developer ID and will not pass
# notarisation if you distribute it.
if ! codesign --force --sign - --timestamp=none "$APP"; then
    echo "note: could not ad-hoc sign; the app still runs locally." >&2
fi

# Finder stamps com.apple.FinderInfo on the bundle once it has an icon, and
# iCloud Drive adds its own attributes to anything under ~/Documents. Either
# makes codesign reject the bundle, so clear them after signing — the seal
# covers the contents, not the directory's attributes.
xattr -c "$APP" 2>/dev/null || true

if ! codesign --verify --strict "$APP" 2>/dev/null; then
    echo "warning: the bundle failed signature verification." >&2
    codesign --verify --strict --verbose=2 "$APP" || true
fi

echo
echo "Built $APP ($(du -sh "$APP" | cut -f1))"
echo "Open it with:  open $APP"
echo "Install it:    cp -r $APP /Applications/"
