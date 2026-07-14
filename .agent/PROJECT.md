# Project

- Rain氛围输入法 (`Rain-Vibetype`) is a free, local-first voice-input desktop application. Its official public repository is `https://github.com/qixiaoyu27/Rain-VibeType`.
- Current application version is `1.0.0`.
- The repository license is GNU AGPL v3 (`AGPL-3.0-only`), stored in the remote root `LICENSE`.
- Desktop stack: Tauri 2 static frontend, Rust core, and a long-lived Python ASR Worker.
- Rust entry point: `src-tauri/src/main.rs`; frontend: `src/`; Worker: `worker/rain_worker.py`.
- Supported model families: SenseVoice Small, Fun-ASR Nano, and Paraformer-zh.
- The UI defaults to Simplified Chinese and also supports system language and English.
- Audio and recognized-text history must never be persisted or uploaded.
- Model weights are external application data and must not be committed or bundled in the installer.
- Models are downloaded from official sources only after explicit user action.
