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

NOTARIZE=false
for arg in "$@"; do
    case "$arg" in
        --notarize) NOTARIZE=true ;;
        -h|--help)
            cat <<'USAGE'
Builds dist/MapCheck.app.

    --notarize    Also submit to Apple and staple the ticket, so the app
                  opens on other Macs. Needs a Developer ID certificate and
                  stored notarytool credentials (see README).

Signing identity: a Developer ID Application certificate is used when the
keychain has one, otherwise the bundle is ad-hoc signed, which is fine on
the machine that built it. Override with MAPCHECK_SIGN_IDENTITY, or set it
to "-" to force ad-hoc. The notarytool keychain profile defaults to
"mapcheck"; override with MAPCHECK_NOTARY_PROFILE.
USAGE
            exit 0
            ;;
        *) echo "unknown option: $arg" >&2; exit 2 ;;
    esac
done

# The bundle is assembled, signed and verified in a temporary directory, then
# moved into dist/ at the end. Building it in place under ~/Documents is not
# reliable: iCloud Drive's file provider re-stamps extended attributes on the
# bundle asynchronously, and codesign refuses a bundle that carries any, so
# clearing them and signing afterwards is a race that gets lost.
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

APP="$STAGE/MapCheck.app"
CONTENTS="$APP/Contents"
FINAL="dist/MapCheck.app"
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)

# Prefer a real Developer ID certificate, the only kind another Mac accepts.
if [ -n "${MAPCHECK_SIGN_IDENTITY:-}" ]; then
    IDENTITY="$MAPCHECK_SIGN_IDENTITY"
else
    IDENTITY=$(security find-identity -v -p codesigning 2>/dev/null |
        sed -n 's/.*"\(Developer ID Application: .*\)"/\1/p' | head -1)
    IDENTITY="${IDENTITY:--}"
fi

if [ "$IDENTITY" = "-" ]; then
    echo "Signing ad-hoc (no Developer ID certificate found)."
    if [ "$NOTARIZE" = true ]; then
        echo "Cannot notarize an ad-hoc signed bundle: Apple requires a" >&2
        echo "Developer ID Application certificate." >&2
        exit 1
    fi
else
    echo "Signing as: $IDENTITY"
fi

./scripts/build-app.sh

echo
echo "Assembling $FINAL"
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

# Notarization requires the hardened runtime and a secure timestamp, so a
# Developer ID signature always gets both. Ad-hoc keeps the offline timestamp,
# never being destined for notarization.
if [ "$IDENTITY" = "-" ]; then
    SIGN_ARGS=(--sign - --timestamp=none)
else
    SIGN_ARGS=(--sign "$IDENTITY" --options runtime --timestamp)
fi

# cp copies extended attributes across, and codesign refuses a bundle that has
# any. Nothing puts them back in the staging directory.
xattr -cr "$APP" 2>/dev/null || true

echo
if ! codesign --force "${SIGN_ARGS[@]}" "$APP"; then
    echo "signing failed" >&2
    exit 1
fi

codesign --verify --strict --verbose=2 "$APP" 2>&1 | sed 's/^/  /'

if [ "$NOTARIZE" = true ]; then
    PROFILE="${MAPCHECK_NOTARY_PROFILE:-mapcheck}"
    ZIP="$STAGE/MapCheck.zip"

    echo
    echo "Submitting to Apple for notarization (usually a minute or two,"
    echo "occasionally much longer)..."

    # notarytool takes an archive, not a bundle; ditto preserves the
    # signature and any symlinks.
    ditto -c -k --keepParent "$APP" "$ZIP"

    if ! xcrun notarytool submit "$ZIP" --keychain-profile "$PROFILE" --wait; then
        echo >&2
        echo "Notarization failed. If the profile is missing, store it once with:" >&2
        echo "  xcrun notarytool store-credentials \"$PROFILE\" \\" >&2
        echo "    --apple-id <your-apple-id> --team-id <your-team-id>" >&2
        echo "For a rejection, read the log:" >&2
        echo "  xcrun notarytool log <submission-id> --keychain-profile \"$PROFILE\"" >&2
        exit 1
    fi

    # Stapling writes the ticket into the bundle so it opens even offline.
    xcrun stapler staple "$APP"
    xcrun stapler validate "$APP"

    echo
    echo "Gatekeeper assessment:"
    spctl --assess --type execute --verbose=2 "$APP" 2>&1 | sed 's/^/  /'
fi

# ditto rather than cp, so the signature and any symlinks survive intact.
mkdir -p dist
rm -rf "$FINAL"
ditto "$APP" "$FINAL"

echo
echo "Built $FINAL ($(du -sh "$FINAL" | cut -f1))"
echo "Open it with:  open $FINAL"
echo "Install it:    cp -r $FINAL /Applications/"

if [ "$IDENTITY" = "-" ]; then
    echo
    echo "This bundle is ad-hoc signed: it runs here, but another Mac will"
    echo "refuse it. Build with --notarize once a Developer ID certificate"
    echo "and notarytool credentials are in place."
elif [ "$NOTARIZE" != true ]; then
    echo
    echo "Signed with your Developer ID but not notarized, so another Mac"
    echo "will still refuse it. Add --notarize to submit it to Apple."
fi
