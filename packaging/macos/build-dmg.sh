#!/usr/bin/env bash
# Builds a .dmg containing the BrowserRouter binaries (br, br-daemon,
# br-settings) and an /Applications symlink for drag-to-install.
#
# Usage: packaging/macos/build-dmg.sh [version]
# Expects release binaries at target/release/{br,br-daemon,br-settings}
# (run `cargo build --workspace --release` first).

set -euo pipefail

VERSION="${1:-0.0.0}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RELEASE_DIR="$ROOT_DIR/target/release"
STAGING_DIR="$(mktemp -d)"
DMG_NAME="br-macos-${VERSION}.dmg"
APP_DIR="$STAGING_DIR/BrowserRouter"

mkdir -p "$APP_DIR/bin"
for bin in br br-daemon br-settings; do
    cp "$RELEASE_DIR/$bin" "$APP_DIR/bin/"
done

cat > "$APP_DIR/install.sh" <<'EOF'
#!/usr/bin/env bash
# Installs BrowserRouter binaries to /usr/local/bin.
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
sudo install -m 755 "$DIR"/bin/br "$DIR"/bin/br-daemon "$DIR"/bin/br-settings /usr/local/bin/
echo "Installed br, br-daemon, br-settings to /usr/local/bin."
echo "Run 'br-settings' to finish onboarding and set br as your default browser."
EOF
chmod +x "$APP_DIR/install.sh" "$APP_DIR/bin/"*

hdiutil create -volname "BrowserRouter" -srcfolder "$STAGING_DIR" -ov -format UDZO "$ROOT_DIR/$DMG_NAME"
rm -rf "$STAGING_DIR"

echo "Created $DMG_NAME"
