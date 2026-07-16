# macOS (Apple Silicon) migration handoff

## Scope

This repository is a Windows 11 x64 application today.  Port it to macOS on
Apple Silicon (`aarch64-apple-darwin`) without changing the local-first
privacy model: audio, recognized text, clipboard contents, and window titles
must remain on-device and must not be persisted.

Do not attempt to cross-compile the Windows binary.  Build and test on a Mac.
Keep the existing Windows implementation working; introduce platform modules
behind `cfg` rather than replacing Windows code.

## Reuse unchanged

- The static frontend in `src/` and all Tauri command names/events.
- The recording state machine in `src-tauri/src/main.rs`.
- `audio.rs`, `config.rs`, `models.rs`, manifest validation, resumable HTTPS
  downloads, hashes, atomic installs, and managed-root deletion rules.
- The line-delimited JSON plus binary PCM Worker protocol in `worker.rs` and
  `native-worker/`.  `native-worker` uses `sherpa-onnx` and is the preferred
  SenseVoice path; compile it natively for Apple Silicon.
- The Python Worker only as a compatibility implementation.  Do not make a
  system `python` a packaged-runtime fallback.

## Replace the Windows adapter

`src-tauri/src/platform_windows.rs` is intentionally Windows-only.  Split its
public API into a small platform facade and `platform_macos.rs`; keep callers
in `main.rs`, `models.rs`, `runtimes.rs`, `worker.rs`, and `text_polish.rs`
unchanged where possible.

| Current behavior | macOS implementation target |
| --- | --- |
| Foreground process capture/revalidation | Accessibility API (`AXUIElement`) and frontmost application PID. Ask for Accessibility permission before recording. |
| Clipboard paste and restore | `NSPasteboard`; restore only if Rain still owns the change count. Preserve available pasteboard types and never overwrite a user copy. |
| Unicode simulated typing | Accessibility keyboard events (`CGEvent`) after the same target-PID check. Clipboard fallback is required if permission or target validation fails. |
| Click-through overlay / no activation | AppKit/NSWindow behavior; retain a separate interactive cancel window if the visual overlay is non-interactive. |
| Work area / fullscreen | `NSScreen.visibleFrame` and macOS fullscreen state. |
| Sounds | `NSSound` or Core Audio. |
| Playback ducking | Core Audio on the default output device; restore original volume on every exit path. It may be deferred behind an unavailable status if safe per-app ducking is not practical. |
| Kill-on-close Worker job | Process-group lifecycle: terminate the spawned child on shutdown and ensure children cannot outlive Rain. |
| Free disk space | `std::fs`/`statfs` implementation for macOS. |

Do not fake accessibility permissions. Surface a localized setup/error state,
keep the text in the clipboard, and never redirect text to a different app.

## Platform-sensitive Rust work

1. Make `windows` and `windows-sys` target-specific dependencies in
   `src-tauri/Cargo.toml`.
2. Gate `mod platform_windows;` and `windows_subsystem` in `main.rs`, then add
   a macOS implementation exposing the same minimum functions/types.
3. Gate `std::os::windows::*`, creation flags, Job Object fields, and
   `nvidia-smi` probing in `worker.rs`, `text_polish.rs`, `runtimes.rs`,
   `models.rs`, and `config.rs`.
4. On Apple Silicon, report the native CPU/Metal-capable component as the
   recommendation. Do not expose NVIDIA/CUDA as an available macOS choice.
5. Change `TEXT_RUNTIME_COMPONENT_ID` and
   `text-runtime-manifest.json` to use an Apple Silicon llama.cpp artifact
   when text polishing is enabled; retain its localhost API key and timeouts.

## Packaging and release

- Update `src-tauri/tauri.conf.json` from Windows-only `nsis` output to a
  macOS `.dmg` or `.app` release target. Keep the bundle free of model weights,
  Python environments, CUDA, and optional runtime binaries.
- Publish separate `aarch64-apple-darwin` native Worker and model/runtime
  manifest entries. Existing Windows ZIPs and hashes are not portable.
- Configure Tauri updater artifacts and signing for macOS separately. Do not
  reuse unsigned Windows update metadata.
- The `scripts/*.ps1` files are Windows release helpers. Replace only the
  release steps required on macOS with shell scripts or a CI job; keep artifact
  schema and SHA-256 validation compatible.

## Suggested acceptance gate

Run `cargo test --all-targets --manifest-path src-tauri/Cargo.toml` and
`cargo test --all-targets --manifest-path native-worker/Cargo.toml` on an
Apple Silicon Mac. Then manually verify:

1. First-run microphone and Accessibility permission flows.
2. Global shortcut, recording/cancel, and overlay without stealing focus.
3. Injection into TextEdit, Safari/Chromium, VS Code, and a different-process
   switch (clipboard-only fallback).
4. Clipboard restoration for text, images, files, and a concurrent user copy.
5. Native SenseVoice load/transcription and optional streaming preview.
6. App quit with both native and text-polish Workers running.
7. A clean-machine `.dmg` install and optional runtime download.

`docs/WINDOWS_ACCEPTANCE.md` remains the Windows release gate. Add a separate
macOS acceptance document rather than weakening its Windows-specific checks.
