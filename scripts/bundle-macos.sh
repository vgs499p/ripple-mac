#!/bin/sh
set -eu
PROJECT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
DIST_DIR=${RIPPLE_DIST_DIR:-"$PROJECT_DIR/dist"}
BUILD_DIR=${CARGO_TARGET_DIR:-"$PROJECT_DIR/target"}
export CARGO_TARGET_DIR="$BUILD_DIR"
export MACOSX_DEPLOYMENT_TARGET=${MACOSX_DEPLOYMENT_TARGET:-11.0}
cargo build ${RIPPLE_CARGO_TARGET:+--target "$RIPPLE_CARGO_TARGET"} --manifest-path "$PROJECT_DIR/Cargo.toml" --locked --release -p ripple-mac
python3 "$PROJECT_DIR/scripts/generate-notices.py"
APP_NAME=${RIPPLE_APP_NAME:-"Ripple Beta"}
APP_DIR="$DIST_DIR/$APP_NAME.app"
mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources"
cp "$BUILD_DIR/${RIPPLE_CARGO_TARGET:+$RIPPLE_CARGO_TARGET/}release/ripple" "$APP_DIR/Contents/MacOS/ripple"
cp "$PROJECT_DIR/assets/Ripple.icns" "$APP_DIR/Contents/Resources/Ripple.icns"
cp "$PROJECT_DIR/LICENSE" "$PROJECT_DIR/COPYING-GPL-3.0" "$PROJECT_DIR/NOTICE" "$PROJECT_DIR/THIRD-PARTY-NOTICES.txt" "$APP_DIR/Contents/Resources/"
cp "$PROJECT_DIR/assets/Info.plist" "$APP_DIR/Contents/Info.plist"
mkdir -p "$APP_DIR/Contents/Resources/Themes"
cp -R "$PROJECT_DIR/assets/themes/." "$APP_DIR/Contents/Resources/Themes/"
# Finder may attach this metadata when the local preview is opened between builds.
/usr/bin/xattr -d com.apple.FinderInfo "$APP_DIR" 2>/dev/null || true
/usr/bin/codesign --force --sign - "$APP_DIR"
printf 'Built %s\n' "$APP_DIR"
