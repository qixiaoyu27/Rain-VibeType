#!/bin/zsh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUTPUT_DIR="${OUTPUT_DIR:-$ROOT/artifacts/runtimes/macos}"
VERSION="${RUNTIME_VERSION:-1.1.0}"
COMPONENT_ID="rain-runtime-onnx-cpu"
ARCHIVE_NAME="rain-runtime-onnx-cpu-aarch64-apple-darwin.zip"
ARCHIVE="$OUTPUT_DIR/$ARCHIVE_NAME"
MANIFEST="$OUTPUT_DIR/runtime-manifest-aarch64-apple-darwin.json"
COMPAT_COMPONENT_FILE="$OUTPUT_DIR/runtime-component-cpu-macos.json"
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

if [[ "$(uname -s)" != "Darwin" || "$(uname -m)" != "arm64" ]]; then
  echo "The Apple Silicon runtime must be built on an arm64 Mac." >&2
  exit 1
fi

cd "$ROOT"
cargo build --release --manifest-path native-worker/Cargo.toml

mkdir -p "$OUTPUT_DIR" "$STAGE/rain-worker"
cp native-worker/target/release/rain-native-worker "$STAGE/rain-worker/rain-native-worker"
chmod 755 "$STAGE/rain-worker/rain-native-worker"

SIGNING_IDENTITY="${CODESIGN_IDENTITY:-${APPLE_SIGNING_IDENTITY:-}}"
if [[ -n "$SIGNING_IDENTITY" ]]; then
  codesign --force --options runtime --timestamp --sign "$SIGNING_IDENTITY" "$STAGE/rain-worker/rain-native-worker"
else
  codesign --force --sign - "$STAGE/rain-worker/rain-native-worker"
fi

rm -f "$ARCHIVE"
(
  cd "$STAGE"
  /usr/bin/zip -X -q -r "$ARCHIVE" rain-worker
)

ARCHIVE_SIZE="$(stat -f %z "$ARCHIVE")"
INSTALLED_SIZE="$(stat -f %z "$STAGE/rain-worker/rain-native-worker")"
DOWNLOAD_BASE="${RUNTIME_DOWNLOAD_BASE:-https://github.com/qixiaoyu27/Rain-VibeType/releases/latest/download}"
SHA256="$(shasum -a 256 "$ARCHIVE" | awk '{print $1}')"
COMPAT_COMPONENT_JSON=""
if [[ "${RAIN_SKIP_COMPAT_RUNTIME:-0}" != "1" ]]; then
  OUTPUT_DIR="$OUTPUT_DIR" \
    RUNTIME_VERSION="$VERSION" \
    RUNTIME_DOWNLOAD_BASE="$DOWNLOAD_BASE" \
    "$ROOT/scripts/build-macos-python-runtime.sh"
  COMPAT_COMPONENT_JSON="$(< "$COMPAT_COMPONENT_FILE")"
fi

cat > "$MANIFEST" <<EOF
{
  "schema_version": 1,
  "manifest_version": "$VERSION-aarch64-apple-darwin",
  "components": [
    {
      "id": "$COMPONENT_ID",
      "display_name": "Rain 原生 SenseVoice · Apple Silicon",
      "version": "$VERSION-aarch64-apple-darwin",
      "accelerator": "cpu",
      "url": "$DOWNLOAD_BASE/$ARCHIVE_NAME",
      "archive_size": $ARCHIVE_SIZE,
      "installed_size": $INSTALLED_SIZE,
      "sha256": "$SHA256",
      "executable": "rain-worker/rain-native-worker"
    }${COMPAT_COMPONENT_JSON:+,
$COMPAT_COMPONENT_JSON}
  ]
}
EOF

echo "Runtime archive: $ARCHIVE"
echo "Runtime manifest: $MANIFEST"
echo "SHA-256: $SHA256"
