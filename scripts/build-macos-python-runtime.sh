#!/bin/zsh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUTPUT_DIR="${OUTPUT_DIR:-$ROOT/artifacts/runtimes/macos}"
VERSION="${RUNTIME_VERSION:-1.0.0}"
TORCH_VERSION="${TORCH_VERSION:-2.11.0}"
PYTHON="${PYTHON:-python3}"
DOWNLOAD_BASE="${RUNTIME_DOWNLOAD_BASE:-https://github.com/qixiaoyu27/Rain-VibeType/releases/latest/download}"
COMPONENT_ID="rain-runtime-cpu"
ARCHIVE_NAME="$COMPONENT_ID-$VERSION-aarch64-apple-darwin.tar.gz"
ARCHIVE="$OUTPUT_DIR/$ARCHIVE_NAME"
COMPONENT_FILE="$OUTPUT_DIR/runtime-component-cpu-macos.json"
VENV="$ROOT/.venv-runtime-cpu-macos"
BUILD_ROOT="$ROOT/build/runtimes/macos-cpu"
DIST_ROOT="$BUILD_ROOT/dist"
WORKER_ROOT="$DIST_ROOT/rain-worker"
WORKER="$WORKER_ROOT/rain-worker"

if [[ "$(uname -s)" != "Darwin" || "$(uname -m)" != "arm64" ]]; then
  echo "The FunASR runtime must be built on an Apple Silicon Mac." >&2
  exit 1
fi
if [[ ! "$VERSION" =~ ^[A-Za-z0-9._-]+$ ]]; then
  echo "RUNTIME_VERSION contains unsupported characters." >&2
  exit 1
fi
if [[ "$DOWNLOAD_BASE" != https://* ]]; then
  echo "RUNTIME_DOWNLOAD_BASE must use HTTPS." >&2
  exit 1
fi
case "$VENV" in
  "$ROOT"/*) ;;
  *) echo "Refusing to create a build environment outside the repository." >&2; exit 1 ;;
esac
case "$BUILD_ROOT" in
  "$ROOT"/*) ;;
  *) echo "Refusing to create build output outside the repository." >&2; exit 1 ;;
esac

rm -rf "$VENV" "$BUILD_ROOT"
mkdir -p "$OUTPUT_DIR" "$DIST_ROOT"

"$PYTHON" -m venv "$VENV"
VENV_PYTHON="$VENV/bin/python"
"$VENV_PYTHON" -m pip install --upgrade pip
"$VENV_PYTHON" -m pip install "torch==$TORCH_VERSION" "torchaudio==$TORCH_VERSION"
"$VENV_PYTHON" -m pip install -r "$ROOT/worker/requirements.txt" "pyinstaller>=6.10,<7"

PYINSTALLER_ARGS=(
  --noconfirm
  --clean
  --onedir
  --name rain-worker
  --distpath "$DIST_ROOT"
  --workpath "$BUILD_ROOT/work"
  --specpath "$BUILD_ROOT/spec"
  --collect-all funasr
  --collect-all modelscope
  --collect-all torch
  --collect-all torchaudio
  --collect-all transformers
  --copy-metadata funasr
  --copy-metadata modelscope
  --copy-metadata torch
  --copy-metadata torchaudio
  --copy-metadata transformers
)
SIGNING_IDENTITY="${CODESIGN_IDENTITY:-${APPLE_SIGNING_IDENTITY:-}}"
if [[ -n "$SIGNING_IDENTITY" ]]; then
  PYINSTALLER_ARGS+=(--codesign-identity "$SIGNING_IDENTITY")
fi
"$VENV_PYTHON" -m PyInstaller "${PYINSTALLER_ARGS[@]}" "$ROOT/worker/rain_worker.py"

if [[ ! -x "$WORKER" ]]; then
  echo "The packaged FunASR Worker is missing: $WORKER" >&2
  exit 1
fi
if [[ "$(lipo -archs "$WORKER")" != "arm64" ]]; then
  echo "The packaged FunASR Worker is not arm64." >&2
  exit 1
fi

find "$WORKER_ROOT" -type f -print0 | while IFS= read -r -d '' candidate; do
  if /usr/bin/file -b "$candidate" | /usr/bin/grep -q 'Mach-O'; then
    if ! codesign --verify --strict "$candidate" >/dev/null 2>&1; then
      if [[ -n "$SIGNING_IDENTITY" ]]; then
        codesign --force --timestamp --sign "$SIGNING_IDENTITY" "$candidate"
      else
        codesign --force --sign - "$candidate"
      fi
    fi
    codesign --verify --strict "$candidate"
  fi
done

rm -f "$ARCHIVE"
COPYFILE_DISABLE=1 /usr/bin/tar -C "$DIST_ROOT" -czf "$ARCHIVE" rain-worker
ARCHIVE_SIZE="$(stat -f %z "$ARCHIVE")"
INSTALLED_SIZE="$(find "$WORKER_ROOT" -type f -exec stat -f %z {} \; | awk '{total += $1} END {print total + 0}')"
SHA256="$(shasum -a 256 "$ARCHIVE" | awk '{print $1}')"

cat > "$COMPONENT_FILE" <<EOF
{
  "id": "$COMPONENT_ID",
  "display_name": "FunASR CPU · Apple Silicon",
  "version": "$VERSION-aarch64-apple-darwin",
  "accelerator": "cpu",
  "url": "$DOWNLOAD_BASE/$ARCHIVE_NAME",
  "archive_size": $ARCHIVE_SIZE,
  "installed_size": $INSTALLED_SIZE,
  "sha256": "$SHA256",
  "executable": "rain-worker/rain-worker"
}
EOF

echo "FunASR runtime archive: $ARCHIVE"
echo "FunASR component metadata: $COMPONENT_FILE"
echo "SHA-256: $SHA256"
