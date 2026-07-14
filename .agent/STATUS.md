# Status

Updated: 2026-07-15

- Windows 11 x64 application code is at version `1.0.0`, is branded `Rain氛围输入法` / `Rain-Vibetype`, and defaults to Simplified Chinese.
- The complete source tree is published on GitHub `main`; the source publication commit is `38b32946e4c43e5ff015d54274396a5fc36c9b51`.
- The full Windows feature surface from the design document is implemented: tray/hotkey/overlay, in-memory capture, safe target handling, clipboard and typing output, three ASR adapters, model repository, resource policies, autostart, updater, diagnostics, and optional crash reports.
- First launch now detects hardware and, after one explicit click, installs the matching CPU/NVIDIA inference component before SenseVoice Small. Neither transfer starts silently.
- Automated verification currently passes: 21 Rust tests with one live-network test ignored, frontend syntax checks, and both release-script AST checks.
- The base NSIS no longer bundles `worker-dist`, PyTorch, or CUDA. An unsigned local installer was produced at about 3.6 MB; updater-artifact signing remains unavailable without the private release key.
- `scripts/build-runtimes.ps1` produces CPU and CUDA 12.8 PyInstaller ZIPs plus a validated manifest containing final HTTPS URLs, sizes, executable paths, and SHA-256 hashes.
- Windows development UI smoke testing passes for Chinese onboarding, model/settings pages, language switching, unconfigured-updater feedback, and no-model startup.
- The ModelScope LFS `HTTP 403 Forbidden` failure is fixed by setting the shared downloader User-Agent. A live one-byte Range request against SenseVoice `model.pt` now passes through the real CDN; the interrupted `.incomplete` directory remains resumable.
- The configured global shortcut now reacts correctly even when the backend returns canonical modifier names/order; a regression test covers `Ctrl+Shift+Space` versus `Shift+Control+Space`.
- The shortcut-triggered main-window/tray freeze is fixed by moving dynamic Escape shortcut operations off the Tauri main thread.
- Direct cursor insertion now uses the stable application-process boundary so Chromium/Electron may recreate internal and top-level HWNDs without forcing clipboard fallback. The overlay's native no-activate display is paired with native hide, fixing the persistent completion window; the release executable was rebuilt and restarted.
- Custom minimize and close-to-tray controls are enabled by explicit Tauri window capability permissions.
- The local development executable is configured to use the project-isolated `.venv-worker` (Python 3.10). The runtime card treats this absolute path as a development fallback and correctly recommends NVIDIA for the detected RTX 5060 Ti.
- Official `qixiaoyu27/Rain-VibeType` GitHub Release URLs are embedded in every build. The public repository currently has no Release, so the UI reports that official components are not published while preserving the development Python fallback.
- The renamed unsigned installer `Rain氛围输入法_1.0.0_x64-setup.exe` builds at about 3.6 MB and the release executable is running with the new window title. Signed updater artifacts still require the private release key.
- Real model loading is confirmed on CPU. Real transcription, CUDA fallback, clean-machine packaging, third-party application input, clipboard-format, and performance matrices still require external/manual validation; see `docs/WINDOWS_ACCEPTANCE.md`.
- The official repository's existing GNU AGPL v3 `LICENSE` is preserved and package metadata declares `AGPL-3.0-only`.
