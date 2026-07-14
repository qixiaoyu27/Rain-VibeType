# Tasks

- [ ] Run the real-model Windows acceptance matrix.
  - Context: Automated adapter tests use simulated models and cannot prove real framework/model compatibility or output quality.
  - Location: `docs/WINDOWS_ACCEPTANCE.md`, `worker/`, `src-tauri/src/worker.rs`.
  - Source: Design sections 25 and 28.
- [ ] Run the Windows application and clipboard matrices.
  - Context: Word, browsers, VS Code, Electron, elevated targets, images/files/multi-format clipboard data, device disconnects, and timing targets require interactive environments.
  - Location: `docs/WINDOWS_ACCEPTANCE.md`.
  - Source: Design sections 25–28.
- [ ] Produce and validate a signed GitHub Releases build.
  - Context: Official GitHub URLs are configured, but the Release assets, signing keys, optional crash endpoint, and a clean Windows 11 VM are still external release inputs.
  - Location: `scripts/release.ps1`, `scripts/build-runtimes.ps1`, `src-tauri/tauri.conf.json`.
  - Source: Design section 3 and release acceptance.
- [ ] Publish and clean-machine test both inference components.
  - Context: The runtime manager and artifact builder are implemented, but the large CPU/CUDA PyInstaller archives have not been built and uploaded in this checkout.
  - Location: `scripts/build-runtimes.ps1`, `src-tauri/src/runtimes.rs`, `docs/WINDOWS_ACCEPTANCE.md`.
  - Source: Optional inference component packaging decision.
