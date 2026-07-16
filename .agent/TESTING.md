# Testing

Confirmed on Windows 11 x64 through 2026-07-16:

- `cargo test --all-targets --manifest-path .\src-tauri\Cargo.toml`
  - Passes 33 offline tests and leaves one live ModelScope CDN test ignored by default. Coverage includes incremental audio preview chunks, latest-preview display bounds, missing GitHub Release handling, per-model runtime mappings, native SenseVoice selection, playback ducking, input validation, and text-polish validation.
- `cargo clippy --all-targets --manifest-path .\src-tauri\Cargo.toml -- -D warnings`
  - Passes without warnings; the Win32 `MONITORINFO` values are initialized directly, so no lint allowance is needed.
- `cargo test live_modelscope_large_file_range_request_works --manifest-path .\src-tauri\Cargo.toml -- --ignored --nocapture`
  - Confirms reqwest can follow the real Fun-ASR Nano `model.pt` redirect and read a one-byte Range response. This guards the required ModelScope User-Agent without downloading the large model file.
- `python -m unittest worker.test_worker -v`
  - Passes 4 simulated-model tests covering all three adapter contracts, PCM/empty audio, unload, cancellation, result shapes, and normalized errors.
- `cargo test --all-targets --manifest-path .\native-worker\Cargo.toml` and `cargo clippy --all-targets --manifest-path .\native-worker\Cargo.toml -- -D warnings`
  - Pass the native SenseVoice Worker contract helper test and warning-free static analysis.
- `.\scripts\build-native-runtime.ps1`
  - Builds release native runtime 1.1.0 with SenseVoice and streaming-Zipformer adapters, packages `rain-worker/rain-worker.exe`, and emits component JSON. The verified ZIP is 7,179,349 bytes and the executable is 19,378,176 bytes.
- Native streaming-preview IPC smoke test
  - Loads the official quantized bilingual Zipformer files, starts a preview stream, sends the 16 kHz test WAV in 100 ms binary PCM frames, and finishes the stream. It produced 14 distinct partial texts and a non-empty completed text.
- `.venv-worker\Scripts\python.exe scripts\compare-workers.py --model <model-dir> --audio <audio-file> --native-worker .\native-worker\target\debug\rain-native-worker.exe --python .\.venv-worker\Scripts\python.exe`
  - On the 5.616-second official SenseVoice Chinese sample, native load/inference were 1,844/195 ms and Python load/inference were 14,665/396 ms. The script correctly reports `texts_match: false` for the observed `开放时间…` versus `开饭时间…` output.
- `.venv-worker\Scripts\python.exe scripts\compare-workers.py --model <model-dir> --aishell-samples 200 --native-worker .\native-worker\target\debug\rain-native-worker.exe --python .\.venv-worker\Scripts\python.exe`
  - Downloads a pinned AISHELL-1 subset into the local application cache, reuses one loaded instance of each Worker, computes normalized CER, writes a per-utterance JSON report, and fails the quality gate if native CER exceeds Python by more than 0.5 percentage points.
  - Two complete runs produced identical accuracy: native 8.23% CER (236/2,867, 124 exact), Python 8.48% (243/2,867, 122 exact), 17 normalized-text disagreements, and a passing -0.24-point native gap. Mean inference was 183–186 ms native versus 319–341 ms Python; model load was 2.34 seconds versus 13.2–15.0 seconds.
- Project-isolated Worker health check and real SenseVoice load
  - `.venv-worker\Scripts\python.exe worker\rain_worker.py` reports `runtime_ready: true` with no missing dependencies.
  - A real `load_model` IPC request for the installed SenseVoice model reaches `model_ready` on CPU in about 18 seconds. This validates model loading only, not real speech transcription quality.
- `node --check .\src\main.js`, `node --check .\src\overlay.js`, and `node --check .\src\cancel.js`
  - Validate all frontend JavaScript entry points.
- Frontend structure validation
  - Confirms 85 unique HTML IDs and all 72 literal `byId(...)` references are backed by DOM elements after adding the paired Home status strips.
  - Confirms all eight settings-page description keys exist in both Chinese and English dictionaries; `node --check .\src\main.js` passes.
  - Confirms the text-polish idle field uses a 1–1,440 minute range and converts the default 600 stored seconds to 10 displayed minutes and back.
  - Confirms no explicit runtime download/delete/progress controls remain in the HTML or JavaScript; text-model deletion retains its awaited native confirmation.
- PowerShell AST parsing of `scripts/release.ps1`, `scripts/build-native-runtime.ps1`, and `scripts/build-runtimes.ps1`
  - Confirms all release scripts are syntactically valid; release staging verifies both final SenseVoice and streaming-preview assets against `models.json`.
- `npx tauri build --no-bundle`
  - Builds the optimized Windows application after the streaming-preview, Home layout, and tray-click changes; the rebuilt `src-tauri/target/release/rain-input.exe` starts successfully.
- `cargo build --release --manifest-path .\src-tauri\Cargo.toml`
  - Confirms the optimized Windows core, native-default runtime selection, per-file model URLs, frontend, playback ducking, clipboard code, and Tauri capabilities compile into `src-tauri/target/release/rain-input.exe`.
- Native managed-runtime smoke check
  - The rebuilt release application starts successfully with managed `runtimes/rain-runtime-onnx-cpu/1.1.0`; the runtime supports both final SenseVoice and independent streaming-preview Worker processes and requires no Python process.
- `npx tauri icon .\src\assets\rain.svg --output <temporary-directory>`
  - Confirms the canonical SVG renders into valid Windows PNG and ICO assets; the required checked-in icon sizes are copied from that generated set.
- `cargo check --manifest-path .\src-tauri\Cargo.toml`
  - Passes after the cloud-rain-audio title-bar and Windows icon replacement.
- `cargo build --manifest-path .\src-tauri\Cargo.toml`
  - Rebuilds the development executable with the embedded Windows icon; the restarted process exposes a responsive `雨音输入法` main window.
- Official llama.cpp `b10016` Windows CPU archive verification
  - The 18,271,892-byte archive matches SHA-256 `5322309f2bde31f8c40f7f041f1e3d8fa08603a5e979c7ff9f4057ac18e37ec6`; `llama-server.exe --help` exposes `--no-webui`, `--api-key`, `--sleep-idle-seconds`, `--threads-http`, and `--jinja`. The temporary archive and extraction were removed after verification.
- `npm run build`
  - Previously confirmed the NSIS build flow. After the product rename, the expected unsigned filename is `src-tauri/target/release/bundle/nsis/雨音输入法_1.0.0_x64-setup.exe`; this renamed installer has not yet been rebuilt.
- Native overlay visibility build check
  - The release build compiles Win32 `SetWindowPos(..., SWP_NOACTIVATE | SWP_SHOWWINDOW)` and paired `ShowWindow(SW_HIDE)` for both overlay windows.
- Manual development UI smoke test
  - Confirms Chinese onboarding, model and settings pages, language switching, missing-update-configuration feedback, no-model startup, tray persistence, and Worker shutdown.
  - Confirms that checking model updates with no published GitHub Release keeps the current catalog and shows `模型清单已经是最新版本` instead of the raw `404 Not Found` URL.
  - Confirms the aqua-glass light and dark Overview pages, Models and General navigation, localized theme-toggle labels, live shortcut/model/device values, and restoration to light mode after the check.
  - Confirms shared button press/hover feedback and panel hover highlighting in the running Tauri window; reduced-motion CSS disables those transitions.
  - Confirms the refined surface hierarchy on Overview and General pages in both light and dark modes, including radial backgrounds, primary/secondary panel contrast, button depth, and restored light mode after the check.
  - Confirms the Chinese Text Input page shows the optional Qwen/llama.cpp component state and independent download/delete controls, scrolls without overlap, and saves the new disabled-by-default settings to `config.json`.
  - Confirms both disabled text-component download buttons keep a normal unavailable cursor instead of the Windows wait cursor.
  - `design-qa.md` compares the approved visual source and the 982 × 702 native-window implementation; the full-view and focused-region passes have no P0-P2 findings.
- Settings-save latency and autostart regression check
  - Five unchanged saves completed in 321/148/150/195/150 ms without a runtime-manifest network wait. Disabling and reenabling autostart updated both `config.json` and the Windows Run entry, and the original enabled state was restored.

Release commands:

- `npm install` restores the removed frontend dependency directory; `npm run dev` then starts the Tauri development application after Worker dependencies are installed.
- `.\scripts\build-runtimes.ps1` builds CPU/CUDA component ZIPs and `runtime-manifest.json`; it requires a real HTTPS artifact base URL.
- `.\scripts\release.ps1` builds those components and the lightweight signed NSIS/update artifacts; it intentionally fails unless application/model/runtime endpoints and signing variables are present.

Not yet confirmed:

- Real transcription with each official model and CUDA; SenseVoice CPU model loading is confirmed.
- A complete full-size model download and hash verification; the live CDN Range probe passes, but deliberately transfers only one byte.
- Windows target-application, elevated-process, clipboard-format, microphone-disconnect, and performance matrices.
- CPU/NVIDIA component download and signed installer/update behavior on a clean machine without Python.
- Full Qwen3 0.6B download, real llama.cpp model load, text quality/latency corpus, timeout recovery, and end-to-end injection after polishing.

Use `docs/WINDOWS_ACCEPTANCE.md` as the release gate; do not convert unrun manual items into pass claims.
