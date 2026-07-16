# Project

- 雨音输入法 (`Rain Vibetype`) is a free, local-first voice-input desktop application. Its official public repository is `https://github.com/qixiaoyu27/Rain-VibeType`.
- Current application version is `1.0.0`.
- The repository license is GNU AGPL v3 (`AGPL-3.0-only`), stored in the remote root `LICENSE`.
- Desktop stack: Tauri 2 static frontend, Rust core, and a long-lived Worker protocol. SenseVoice defaults to the Rust/sherpa-onnx Worker; Python/FunASR remains for Fun-ASR Nano, Paraformer-zh, and legacy SenseVoice installations without native files.
- Preliminary platform targets are Windows 11 x64 on Intel/AMD CPUs, optional NVIDIA GPU acceleration through CUDA, and macOS on Apple Silicon; NVIDIA CPUs are not a target.
- The native SenseVoice CPU path passed a repeatable 200-clip AISHELL-1 clean-read gate against Python/FunASR: 8.23% versus 8.48% CER, with about 0.18 seconds mean inference per clip. Real voice-input/noisy-speech and cross-platform validation is still pending.
- Rust entry point: `src-tauri/src/main.rs`; frontend: `src/`; default SenseVoice Worker: `native-worker/src/main.rs`; compatibility Worker: `worker/rain_worker.py`.
- Supported final ASR model families: SenseVoice Small, Fun-ASR Nano, and Paraformer-zh. An optional streaming Zipformer model provides live preview only; optional text polishing uses Qwen3 0.6B GGUF through llama.cpp.
- The UI defaults to Simplified Chinese and also supports system language and English.
- User-facing copy calls optional execution packages `推理框架`; Rust and manifest internals keep the conventional `runtime` name.
- Audio and recognized-text history must never be persisted or uploaded.
- Model weights are external application data and must not be committed or bundled in the installer.
- Models are downloaded from official sources only after explicit user action.
