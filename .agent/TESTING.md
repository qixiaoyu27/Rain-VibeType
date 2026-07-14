# Testing

Confirmed on Windows 11 x64 on 2026-07-14:

- `cargo test --all-targets --manifest-path .\src-tauri\Cargo.toml`
  - Passes 21 offline tests and leaves one live ModelScope CDN test ignored by default. Coverage includes the same-process current-cursor injection boundary, CPU/NVIDIA automatic selection, rejection of non-HTTPS or unsafe runtime manifests, and official GitHub Release endpoint defaults.
- `cargo test live_modelscope_large_file_range_request_works --manifest-path .\src-tauri\Cargo.toml -- --ignored --nocapture`
  - Confirms reqwest can follow the real SenseVoice `model.pt` redirect and read a one-byte Range response. This guards the required ModelScope User-Agent and reproduces the former CDN 403 without downloading the 0.9 GB file.
- `python -m unittest worker.test_worker -v`
  - Passes 4 simulated-model tests covering all three adapter contracts, PCM/empty audio, unload, cancellation, result shapes, and normalized errors.
- Project-isolated Worker health check and real SenseVoice load
  - `.venv-worker\Scripts\python.exe worker\rain_worker.py` reports `runtime_ready: true` with no missing dependencies.
  - A real `load_model` IPC request for the installed SenseVoice model reaches `model_ready` on CPU in about 18 seconds. This validates model loading only, not real speech transcription quality.
- `node --check .\src\main.js`, `node --check .\src\overlay.js`, and `node --check .\src\cancel.js`
  - Validate all frontend JavaScript entry points.
- PowerShell AST parsing of `scripts/release.ps1` and `scripts/build-runtimes.ps1`
  - Confirms both release scripts are syntactically valid. A runnable assertion checks the three official manifest filenames and the `qixiaoyu27/Rain-VibeType` runtime base URL.
- `cargo build --release --manifest-path .\src-tauri\Cargo.toml`
  - Confirms the optimized Windows core and Tauri capability schema compile and produce `src-tauri/target/release/rain-input.exe`.
- `npm run build`
  - Produces `src-tauri/target/release/bundle/nsis/Rain氛围输入法_1.0.0_x64-setup.exe`; without `TAURI_SIGNING_PRIVATE_KEY`, only the subsequent updater-signing step fails as intended.
- Native overlay visibility build check
  - The release build compiles Win32 `SetWindowPos(..., SWP_NOACTIVATE | SWP_SHOWWINDOW)` and paired `ShowWindow(SW_HIDE)` for both overlay windows.
- Manual development UI smoke test
  - Confirms Chinese onboarding, model and settings pages, language switching, missing-update-configuration feedback, no-model startup, tray persistence, and Worker shutdown.

Release commands:

- `npm run dev` starts the Tauri development application after Worker dependencies are installed.
- `.\scripts\build-runtimes.ps1` builds CPU/CUDA component ZIPs and `runtime-manifest.json`; it requires a real HTTPS artifact base URL.
- `.\scripts\release.ps1` builds those components and the lightweight signed NSIS/update artifacts; it intentionally fails unless application/model/runtime endpoints and signing variables are present.

Not yet confirmed:

- Real transcription with each official model and CUDA; SenseVoice CPU model loading is confirmed.
- A complete full-size model download and hash verification; the live CDN Range probe passes, but deliberately transfers only one byte.
- Windows target-application, elevated-process, clipboard-format, microphone-disconnect, and performance matrices.
- CPU/NVIDIA component download and signed installer/update behavior on a clean machine without Python.

Use `docs/WINDOWS_ACCEPTANCE.md` as the release gate; do not convert unrun manual items into pass claims.
