# Status

Updated: 2026-07-15

- Windows 11 x64 application code is at version `1.0.0`, is branded `Rain氛围输入法` / `Rain-Vibetype`, and defaults to Simplified Chinese.
- The complete current source tree is published on GitHub `main`.
- The full Windows feature surface from the design document is implemented: tray/hotkey/overlay, in-memory capture, safe target handling, clipboard and typing output, three ASR adapters, model repository, resource policies, autostart, updater, diagnostics, and optional crash reports.
- First launch now detects hardware and, after one explicit click, installs the matching CPU/NVIDIA inference component before SenseVoice Small. Neither transfer starts silently.
- Automated verification currently passes: 22 Tauri Rust tests with one live-network test ignored, 1 native-Worker Rust test, native Clippy, 4 Python Worker tests, frontend syntax checks, and all four release/export script AST checks.
- The base NSIS no longer bundles `worker-dist`, PyTorch, or CUDA. An unsigned local installer was previously produced at about 3.6 MB; generated installers and build outputs were removed on 2026-07-15, and updater-artifact signing remains unavailable without the private release key.
- `scripts/build-runtimes.ps1` produces CPU and CUDA 12.8 PyInstaller ZIPs plus a validated manifest containing final HTTPS URLs, sizes, executable paths, and SHA-256 hashes.
- Windows development UI smoke testing passes for Chinese onboarding, model/settings pages, language switching, unconfigured-updater feedback, and no-model startup.
- The ModelScope LFS `HTTP 403 Forbidden` failure is fixed by setting the shared downloader User-Agent. A live one-byte Range request against SenseVoice `model.pt` now passes through the real CDN; the interrupted `.incomplete` directory remains resumable.
- The configured global shortcut now reacts correctly even when the backend returns canonical modifier names/order; a regression test covers `Ctrl+Shift+Space` versus `Shift+Control+Space`.
- The shortcut-triggered main-window/tray freeze is fixed by moving dynamic Escape shortcut operations off the Tauri main thread.
- Direct cursor insertion captures the foreground process before all runtime work and no longer requires Chromium/Electron `hwndFocus` during capture or completion. NVIDIA probing is cached and hidden, hotkey auto-repeat is ignored, and clipboard restore now uses deep format copies plus a 300 ms paste window.
- Custom minimize and close-to-tray controls are enabled by explicit Tauri window capability permissions.
- The local development executable is configured to use the project-isolated `.venv-worker` (Python 3.10). The runtime card treats this absolute path as a development fallback and correctly recommends NVIDIA for the detected RTX 5060 Ti.
- Official `qixiaoyu27/Rain-VibeType` GitHub Release URLs are embedded in every build. The public repository currently has no Release, so the UI reports that official components are not published while preserving the development Python fallback.
- The voice settings page now includes an opt-in switch that reduces system playback to 20% only while recording and restores the prior master volume afterward.
- The renamed unsigned installer `Rain氛围输入法_1.0.0_x64-setup.exe` builds at about 3.6 MB. The old release binary, installer, WebView cache, and stale autostart entries were removed during cleanup; the current development build is running. End-to-end cursor insertion and playback-ducking behavior still await the user's interactive speech test. Signed updater artifacts still require the private release key.
- An experimental unquantized SenseVoice ONNX export and CPU-only Rust/sherpa-onnx Worker run end to end through the existing JSON+PCM IPC. Its generated roughly 6.7 MiB runtime ZIP was removed after validation and is not in the production manifest; the script can reproduce it.
- On the same 5.616-second official sample, three earlier repetitions plus the checked-in comparison path were stable: native load about 1.6–1.8 seconds and inference about 0.17–0.20 seconds; Python load about 13–15 seconds and inference about 0.37–0.40 seconds. Native returned `开放时间…`; Python returned `开饭时间…`, so quality parity is explicitly unresolved.
- Real SenseVoice CPU transcription is confirmed on both native and Python paths. CUDA fallback, clean-machine packaging, representative-corpus quality, third-party application input, clipboard-format, and full performance matrices still require external/manual validation; see `docs/WINDOWS_ACCEPTANCE.md`.
- Settings-save latency is fixed. Ordinary saves no longer refresh the remote runtime manifest, and unchanged autostart/tray state is skipped. Five post-fix saves completed in about 321 ms, 148 ms, 150 ms, 195 ms, and 150 ms versus a 1,371 ms pre-fix cold save.
- The desktop UI now uses a black-and-white high-contrast design with a title-bar dark/light toggle. Both themes were visually checked in the running Tauri window, and the selected theme survives a page reload through local storage.
- The official repository's existing GNU AGPL v3 `LICENSE` is preserved and package metadata declares `AGPL-3.0-only`.
