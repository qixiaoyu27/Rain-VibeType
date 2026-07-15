# Decisions

## 2026-07-14 — Implement the complete Windows 11 V1.0 surface

Decision:
- Supersede the earlier v0.1-only implementation plan and implement the design document's full Windows 11 x64 surface in version `1.0.0`.

Context:
- The user explicitly requested another full document audit and complete implementation, while narrowing the requested platform to Windows 11.

Rationale:
- Keeping later features as speculative TODOs no longer matched the requested scope.

Consequences:
- Model management, all three adapters, resource policies, clipboard transactions, updater, diagnostics, autostart, bilingual UI, release packaging, and acceptance documentation are first-class parts of the repository.
- Apple Silicon macOS remains outside this checkout's requested implementation and acceptance scope.

## 2026-07-14 — Require explicit model-download consent

Decision:
- Recommend SenseVoice Small on first launch and provide one-click download plus automatic configuration, but never start a model transfer silently.

Context:
- The design explicitly allows skipping model download and says the recommended model is not automatically downloaded.

Rationale:
- A model transfer is large and network-visible; one clear click preserves informed consent without requiring manual setup.

Consequences:
- The app can start and expose settings without a model. Recording routes to Model management until an installed model is selected.

## 2026-07-14 — Use complete clipboard transactions by default

Decision:
- Make clipboard paste the default output mode and retain Unicode simulated input as an option.

Context:
- The design requires complete clipboard preservation and concurrent-change protection.

Rationale:
- Per-format deep copies preserve non-text and multi-format clipboard contents independently of the live clipboard owner, while sequence-number checks avoid overwriting a user's later copy.

Consequences:
- Target changes or injection failures leave the complete recognition result in the clipboard without reclaiming focus.

## 2026-07-15 — Capture the input target before helper processes

Decision:
- Capture the foreground process as the first hotkey action, accept it without requiring `GetGUIThreadInfo.hwndFocus`, and run the cached NVIDIA probe with `CREATE_NO_WINDOW`.

Context:
- Diagnostics repeatedly ended in `INPUT_TARGET_CHANGED`; the external review identified synchronous `nvidia-smi` execution in the hotkey path, and the remaining initial `hwndFocus` gate could discard valid Chromium/Electron targets.

Rationale:
- The foreground process is the stable safety boundary. Runtime probes must not race with or visibly disturb target capture.

Consequences:
- Injection remains blocked after a real application switch, but valid modern controls no longer fall back to clipboard merely because Win32 omits a focused child HWND.
- NVIDIA detection runs once per application process without creating a console window.

## 2026-07-15 — Make system playback ducking opt-in

Decision:
- Add a disabled-by-default recording setting that reduces the Windows default playback endpoint to 20% of its prior master volume and restores it through a recording-scoped guard.

Context:
- Playback captured acoustically by the microphone can reduce recognition quality; the user requested a settings switch rather than unconditional volume changes.

Rationale:
- Windows Core Audio provides the smallest native implementation and does not alter microphone gain.

Consequences:
- Normal stop, cancellation, and handled recording failures restore the saved volume; existing configurations keep the feature off until explicitly enabled.

## 2026-07-14 — Constrain Worker and model filesystem boundaries

Decision:
- Put the Worker in a kill-on-close Windows Job Object and permit model loading/deletion only below the configured managed root with Rain markers.

Context:
- A crashed parent must not leave a Python process, and model cleanup must never recurse into arbitrary user directories.

Rationale:
- OS-enforced lifecycle and canonical path checks provide durable safety invariants.

Consequences:
- Worker startup fails if Job Object assignment fails. Imported content is copied into managed storage before it can be loaded.

## 2026-07-14 — Default the product UI to Simplified Chinese

Decision:
- Set `ui_language` to `zh-CN` for new configurations while keeping `system`, `zh-CN`, and `en` selectable.

Context:
- The user explicitly required a Chinese edition; the design also requires a language selector.

Rationale:
- Chinese-first defaults satisfy the requested product while preserving the documented bilingual capability.

Consequences:
- First launch, settings, tray, overlays, confirmations, and primary errors are Chinese unless the user changes the language.

## 2026-07-14 — Keep model updates independent and reversible

Decision:
- Use a release-configured HTTPS model catalog endpoint that is separate from the signed application updater.

Context:
- The design requires model and application updates to be independent, forbids automatic model replacement, and requires a failed update to leave the old version usable.

Rationale:
- An explicit catalog check plus per-file hashes and versioned installation directories allows the user to control every large transfer and switch.

Consequences:
- A catalog change only exposes a new version. The user must start its download; Rain verifies it before switching, retains the old definition and files, and asks whether to remove the old version after a successful switch.

## 2026-07-14 — Ship CPU and NVIDIA inference as optional components

Decision:
- Keep the Windows base installer small and publish CPU and NVIDIA CUDA Workers as separate, manifest-driven components selected from the user's device preference and detected hardware.

Context:
- Bundling the GPU PyTorch/CUDA stack would make every installer large, including for users without NVIDIA hardware.

Rationale:
- One explicit first-run download preserves user consent, gives NVIDIA systems acceleration, avoids charging CPU-only users for CUDA bytes, and lets runtime components update independently.

Consequences:
- Release infrastructure must publish two HTTPS ZIP artifacts and `runtime-manifest.json` before the base installer.
- Every component is size/SHA-256 verified and atomically installed before Worker switching; missing or mismatched components block recording with a Chinese setup message.
- Source builds retain only an explicit absolute Python executable as a development fallback.

## 2026-07-14 — Validate the stable foreground window, not Chromium child HWNDs

Decision:
- Permit injection when the original foreground top-level window and process are still active, even if its internal focused child HWND changed.

Context:
- Chromium/Electron can recreate internal focus HWNDs without changing the visible input target, causing successful recognition to fall back to clipboard-only output.

Rationale:
- The top-level window/process boundary prevents cross-application injection while allowing text to reach the current cursor in modern controls.

Consequences:
- Moving the cursor to another field inside the same original window directs text to that current cursor; switching windows or processes still forces clipboard fallback.

## 2026-07-15 — Amend cursor validation to the application process boundary

Decision:
- Supersede the top-level-window equality requirement: accept the current cursor when its foreground target belongs to the originally captured process.

Context:
- Chromium/Electron can recreate both child and top-level HWNDs during a recognition session; repeated successful transcriptions still produced `INPUT_TARGET_CHANGED` with no injection errors.

Rationale:
- The user explicitly requires writing to the current cursor. Process identity is the stable Windows boundary that still prevents cross-application injection.

Consequences:
- Cursor movement or window changes inside the same application receive the text; changing applications falls back to clipboard.

## 2026-07-15 — Use Rain氛围输入法 and the official GitHub repository

Decision:
- Use `Rain氛围输入法` as the Chinese product name and `Rain-Vibetype` as the English name.
- Default release downloads to `qixiaoyu27/Rain-VibeType` GitHub Releases.

Context:
- The repository owner supplied the official public repository and final product names.

Rationale:
- Stable repository-owned URLs remove manual endpoint setup while keeping release assets under project control.

Consequences:
- A GitHub Release must contain `latest.json`, `models.json`, `runtime-manifest.json`, and both runtime ZIPs before those URLs become usable.
- The existing Tauri identifier remains unchanged so current configuration and downloaded models are preserved.

## 2026-07-15 — Amend the future platform scope

Decision:
- Keep Windows 11 x64 on Intel/AMD CPUs as the baseline, treat NVIDIA GPUs through CUDA as optional acceleration, and include macOS on Apple Silicon in the preliminary multi-platform scope. NVIDIA CPUs are not included.

Context:
- Initial use suggests the primary model may already be fast enough on CPU, and the product now has a preliminary cross-platform compatibility requirement.

Rationale:
- A CPU-capable baseline covers Intel and AMD Windows systems and gives the macOS port a portable starting point without making NVIDIA hardware mandatory.

Consequences:
- The current Windows V1 remains the implemented release; the earlier statement that Apple Silicon is outside all future scope is superseded.
- CPU sufficiency must be confirmed with repeatable latency and memory benchmarks before removing CUDA support or choosing a replacement inference runtime.
- Do not quantize or convert the current SenseVoice model merely to reduce size or improve speed; retain the existing FunASR/PyTorch path unless a concrete cross-platform packaging or compatibility problem justifies a runtime migration.

## 2026-07-15 — Stage the native SenseVoice runtime behind a parity gate

Decision:
- Keep the current SenseVoice weights and add an unquantized ONNX export plus a Rust/sherpa-onnx CPU Worker that implements the existing IPC contract.
- Keep Python/FunASR as the active fallback and do not add the native component to the production runtime manifest until corpus-level output checks pass.

Context:
- The product needs Intel/AMD Windows and Apple Silicon compatibility without bundling PyTorch, while the existing model is already fast enough and should not be replaced or quantized.

Rationale:
- Reusing the Worker protocol isolates the runtime migration from recording, hotkey, tray, and text-injection code. A small native runtime reduces framework packaging while retaining a reversible baseline for recognition quality.

Consequences:
- The native CPU runtime ZIP is about 6.7 MiB, but the external unquantized ONNX model remains about 894 MiB.
- Runtime publication also needs official `model.onnx` and `tokens.txt` artifacts; local export alone is not a clean-machine distribution strategy.
- A one-sample comparison produced different Chinese text in one word, so production selection remains unchanged pending a representative speech corpus.

## 2026-07-15 — Use a neutral high-contrast interface with local theme persistence

Decision:
- Replace the teal glow/grid visual language with a ChatGPT-like neutral black-and-white system and support explicit dark/light switching from the title bar.

Context:
- The user requested a higher-contrast interface with both dark and light modes.

Rationale:
- Semantic neutral CSS variables keep all current pages consistent without changing business markup or backend configuration.

Consequences:
- The first render follows the operating-system color preference unless `rain-theme` is already stored locally; an explicit selection persists across reloads.
- Error red remains available only for destructive or failed states; ordinary navigation, cards, controls, and progress surfaces stay neutral.
