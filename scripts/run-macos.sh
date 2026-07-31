#!/bin/zsh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

if [[ "$(uname -s)" != "Darwin" || "$(uname -m)" != "arm64" ]]; then
  echo "Rain macOS development requires an Apple Silicon Mac (arm64)." >&2
  exit 1
fi

cd "$ROOT"
cargo build --manifest-path native-worker/Cargo.toml

if [[ ! -d node_modules ]]; then
  npm ci
fi

export RAIN_DEV_NATIVE_WORKER="$ROOT/native-worker/target/debug/rain-native-worker"
exec npm run dev
