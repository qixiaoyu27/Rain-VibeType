#!/bin/zsh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ "$(uname -s)" != "Darwin" || "$(uname -m)" != "arm64" ]]; then
  echo "The macOS release must be built on an Apple Silicon Mac." >&2
  exit 1
fi

npm ci
cargo test --all-targets --manifest-path src-tauri/Cargo.toml
cargo test --all-targets --manifest-path native-worker/Cargo.toml
python3 -m unittest worker.test_worker -v
node --check src/main.js
"$ROOT/scripts/build-macos-runtime.sh"
if [[ -n "${APPLE_SIGNING_IDENTITY:-}" ]]; then
  export RAIN_CREATE_UPDATER_ARTIFACTS="${TAURI_SIGNING_PRIVATE_KEY:+true}"
  RELEASE_CONFIG="$(python3 - <<'PY'
import json
import os

print(json.dumps({
    "bundle": {
        "createUpdaterArtifacts": os.environ.get("RAIN_CREATE_UPDATER_ARTIFACTS") == "true",
        "macOS": {"signingIdentity": os.environ["APPLE_SIGNING_IDENTITY"]},
    }
}))
PY
)"
  npm run build -- --target aarch64-apple-darwin --config "$RELEASE_CONFIG"
elif [[ -n "${TAURI_SIGNING_PRIVATE_KEY:-}" ]]; then
  npm run build -- --target aarch64-apple-darwin --config '{"bundle":{"createUpdaterArtifacts":true}}'
else
  npm run build -- --target aarch64-apple-darwin
fi

echo "SHA-256 files:"
for directory in \
  "$ROOT/src-tauri/target/aarch64-apple-darwin/release/bundle/dmg" \
  "$ROOT/src-tauri/target/aarch64-apple-darwin/release/bundle/macos" \
  "$ROOT/artifacts/runtimes/macos"; do
  [[ -d "$directory" ]] || continue
  while IFS= read -r -d '' artifact; do
    checksum="$artifact.sha256"
    (
      cd "$(dirname "$artifact")"
      shasum -a 256 "$(basename "$artifact")" > "$(basename "$checksum")"
    )
    echo "$checksum"
  done < <(find "$directory" -maxdepth 1 -type f \
    \( -name '*.dmg' -o -name '*.zip' -o -name '*.tar.gz' \) \
    ! -name 'rw.*' -print0)
done
