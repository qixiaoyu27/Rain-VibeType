#!/bin/zsh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MANIFEST="${1:-$ROOT/artifacts/runtimes/macos/runtime-manifest-aarch64-apple-darwin.json}"
COMPONENT_ID="${2:-rain-runtime-cpu}"
RUNTIME_ROOT="${RAIN_RUNTIME_ROOT:-$HOME/Library/Application Support/io.github.rain.voice-input/runtimes}"

if [[ "$(uname -s)" != "Darwin" || "$(uname -m)" != "arm64" ]]; then
  echo "Local runtime installation requires an Apple Silicon Mac." >&2
  exit 1
fi
if [[ ! -f "$MANIFEST" ]]; then
  echo "Runtime manifest is missing: $MANIFEST" >&2
  exit 1
fi

COMPONENT_FIELDS="$(python3 - "$MANIFEST" "$COMPONENT_ID" <<'PY'
import json
import pathlib
import re
import sys

manifest = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
requested_id = sys.argv[2]
identifier = re.compile(r"[A-Za-z0-9._-]+")
if requested_id in {".", ".."} or identifier.fullmatch(requested_id) is None:
    raise SystemExit("Component id contains unsupported characters")
component = next((item for item in manifest["components"] if item["id"] == requested_id), None)
if component is None:
    raise SystemExit(f"Component {requested_id} is not present in the manifest")
version = str(component["version"])
if version in {".", ".."} or identifier.fullmatch(version) is None:
    raise SystemExit("Component version contains unsupported characters")
executable = str(component["executable"])
if (
    not executable
    or executable.startswith("/")
    or any(part in {"", ".", ".."} for part in executable.split("/"))
):
    raise SystemExit("Component executable must be a relative path without traversal")
print("\t".join(str(component[key]) for key in ("version", "url", "sha256", "executable")))
PY
)"
IFS=$'\t' read -r VERSION URL SHA256 EXECUTABLE <<< "$COMPONENT_FIELDS"
ARCHIVE="$(dirname "$MANIFEST")/$(basename "$URL")"
if [[ ! -f "$ARCHIVE" ]]; then
  echo "Runtime archive is missing: $ARCHIVE" >&2
  exit 1
fi
if [[ "$(shasum -a 256 "$ARCHIVE" | awk '{print $1}')" != "$SHA256" ]]; then
  echo "Runtime archive checksum mismatch." >&2
  exit 1
fi

mkdir -p "$RUNTIME_ROOT"
SAFE_PATHS="$(python3 - "$RUNTIME_ROOT" "$COMPONENT_ID" "$VERSION" "$$" <<'PY'
import pathlib
import sys

root = pathlib.Path(sys.argv[1]).expanduser().resolve()
target = (root / sys.argv[2] / sys.argv[3]).resolve()
staging = (root / sys.argv[2] / f"{sys.argv[3]}.install-{sys.argv[4]}").resolve()
for label, path in (("target", target), ("staging", staging)):
    try:
        path.relative_to(root)
    except ValueError:
        raise SystemExit(f"Runtime {label} escapes the runtime root")
    if path == root:
        raise SystemExit(f"Runtime {label} cannot be the runtime root")
print(f"{target}\t{staging}")
PY
)"
IFS=$'\t' read -r TARGET STAGING <<< "$SAFE_PATHS"
trap 'rm -rf "$STAGING"' EXIT
mkdir -p "$(dirname "$TARGET")" "$STAGING"
case "$ARCHIVE" in
  *.tar.gz|*.tgz) /usr/bin/tar -C "$STAGING" -xzf "$ARCHIVE" ;;
  *.zip) /usr/bin/ditto -x -k "$ARCHIVE" "$STAGING" ;;
  *) echo "Unsupported runtime archive: $ARCHIVE" >&2; exit 1 ;;
esac
if [[ ! -x "$STAGING/$EXECUTABLE" ]]; then
  echo "Runtime executable is missing after extraction: $EXECUTABLE" >&2
  exit 1
fi
codesign --verify --strict "$STAGING/$EXECUTABLE"

python3 - "$MANIFEST" "$COMPONENT_ID" "$STAGING/.rain-runtime.json" <<'PY'
import json
import pathlib
import sys

manifest = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
component = next(item for item in manifest["components"] if item["id"] == sys.argv[2])
marker = {
    "schema_version": 1,
    "manifest_version": manifest["manifest_version"],
    "component": component,
}
pathlib.Path(sys.argv[3]).write_text(json.dumps(marker, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY

if [[ -e "$TARGET" ]]; then
  rm -rf "$TARGET"
fi
mv "$STAGING" "$TARGET"
cp "$MANIFEST" "$RUNTIME_ROOT/.runtime-manifest.json"
trap - EXIT

echo "Installed $COMPONENT_ID at $TARGET"
