# macOS Apple Silicon implementation

## Supported target

- Target triple: `aarch64-apple-darwin`
- Minimum system: macOS 12
- Hardware: Apple Silicon M1 or newer
- Package formats: `.app`, `.dmg`, and a signed Tauri updater archive when a
  release signing key is supplied

The Windows implementation remains intact. Rust selects `platform_windows.rs`
or `platform_macos.rs` with `cfg`, while the recording, recognition, model,
download, text-polish, diagnostics, and frontend state machines stay shared.

## Keyboard mapping

The fresh-install default is platform-specific:

| Windows | macOS |
| --- | --- |
| `Ctrl + Shift + Space` | `Command + Shift + Space` (`⌘⇧Space`) |
| Paste with `Ctrl + V` | Paste with `Command + V` |
| Win/Super modifier | Command modifier |
| Alt modifier | Option modifier |

The hotkey recorder maps the physical macOS Command key to Tauri's `Super`
modifier and renders it as `⌘`. Existing Windows default configuration is
migrated to `Super+Shift+Space` on first macOS load; a subsequently recorded
custom Control-based shortcut is preserved.

Before posting `Command+V`, Rain releases any still-held Shift, Control,
Option, or Command modifiers. This prevents push-to-talk modifiers from
turning the paste into a different shortcut.

## Native macOS platform adapter

`src-tauri/src/platform_macos.m` and `platform_macos.rs` provide:

- Frontmost-application capture and PID revalidation through AppKit and the
  Accessibility API.
- A first-use Accessibility permission prompt. Rain refuses injection and
  keeps text in the clipboard when permission or target validation fails.
- `NSPasteboard` snapshots that preserve every available item/type. The
  original clipboard is restored only when the pasteboard change count still
  belongs to Rain, so a concurrent user copy is never overwritten.
- Unicode simulated typing and `Command+V` through `CGEvent`.
- Non-activating overlay windows on all Spaces and over full-screen apps,
  with a separate interactive cancel window.
- Full-screen detection through the focused AX window and overlay placement
  through the target screen's `visibleFrame`.
- System sounds, safe master/channel audio ducking with guaranteed restore,
  free-disk-space reporting, native confirmation dialogs, and Finder opening.
- Process-group ownership for the ASR and llama.cpp workers so child processes
  cannot outlive Rain.

## CoreAudio recording model

`cpal::Stream` is not `Send` on macOS. The Apple Silicon build therefore owns
the stream on a dedicated audio thread. Tauri state contains only thread-safe
sample/error buffers and a stop handle. This preserves the Windows behavior
without unsafe `Send` assertions.

## Native inference runtimes

- `native-worker/` builds directly as an arm64 Mach-O binary and statically
  includes sherpa-onnx.
- `scripts/build-macos-runtime.sh` signs the worker, preserves executable mode,
  creates the native SenseVoice archive plus an arm64 self-contained FunASR
  CPU runtime for Fun-ASR-Nano and Paraformer, and writes their matching
  SHA-256 runtime manifest.
- macOS fetches
  `runtime-manifest-aarch64-apple-darwin.json`; it never consumes the Windows
  runtime manifest.
- Text cleanup uses the official llama.cpp macOS arm64 archive. Runtime
  extraction supports both ZIP and safe `.tar.gz` archives, including bounded
  relative symlinks required by llama.cpp dylibs.
- Source development can use `RAIN_DEV_NATIVE_WORKER` via
  `scripts/run-macos.sh`; packaged releases never fall back to a system Python.

## Packaging and permissions

`src-tauri/Info.plist` contains the microphone usage description, and
`src-tauri/Entitlements.plist` grants hardened-runtime audio input. The base app
does not contain model weights, Python environments, CUDA components, or the
optional downloaded runtimes.

For public distribution, sign and notarize the app and runtime worker with a
Developer ID. Upload these runtime files beside the `.dmg` release:

- `rain-runtime-onnx-cpu-aarch64-apple-darwin.zip`
- `rain-runtime-cpu-1.0.0-aarch64-apple-darwin.tar.gz`
- `runtime-manifest-aarch64-apple-darwin.json`

The repository's default `-` signing identity is only for local ad-hoc builds.
macOS binds Accessibility and microphone grants to an ad-hoc build's code hash,
so rebuilding it requires granting those permissions again. Use a stable
Developer ID identity for updates that preserve TCC permissions.

`scripts/build-macos.sh` uses `APPLE_SIGNING_IDENTITY` for the app and Worker,
and enables updater artifacts when `TAURI_SIGNING_PRIVATE_KEY` is also set.

## Commands

```bash
./scripts/run-macos.sh
./scripts/build-macos-runtime.sh
npm run build -- --target aarch64-apple-darwin
```

Use [MACOS_ACCEPTANCE.md](MACOS_ACCEPTANCE.md) as the release gate.
