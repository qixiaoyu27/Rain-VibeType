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

## 2026-07-15 — Adopt the selected aqua-glass desktop layout

Decision:
- Supersede the neutral black-and-white visual direction with the user-selected aqua-glass mock while preserving explicit light/dark switching and the existing left-side navigation.

Context:
- The user selected a concrete visual reference after requesting larger typography, fewer decorative words, and the original left-menu placement.

Rationale:
- Matching one approved source keeps layout, scale, color, and interaction hierarchy consistent across the Overview and settings pages.

Consequences:
- The Overview page uses live shortcut, model, device, privacy, and input values inside the approved status/hero/detail structure.
- Settings and model pages reuse the same glass tokens, Fluent icon family, title bar, and left navigation.

## 2026-07-15 — Adopt the cloud-rain-audio brand mark

Decision:
- Use the selected rounded-square cloud mark with three diagonal rain/audio strokes: the center stroke is longest, the two side strokes are shorter, and the right stroke is amber.

Context:
- The user selected this concept after comparing vertical and diagonal rain-line variants.

Rationale:
- The diagonal strokes communicate both falling rain and audio rhythm more distinctly than a generic vertical equalizer.

Consequences:
- `src/assets/rain.svg` is the source of truth, and Windows application icons are regenerated from it.

## 2026-07-15 — Keep the base installer as a modular skeleton

Decision:
- The base installer contains only the necessary desktop shell and component manager. Models, inference backends, and substantial feature modules are optional downloads with independent install, update, disable, and uninstall lifecycles.

Context:
- The user requires Rain to remain highly modular and freely configurable across hardware and platforms.

Rationale:
- Users should not pay the disk, download, dependency, or hardware cost of capabilities they do not choose.

Consequences:
- Optional downloads always require explicit user action.
- Missing, disabled, or failed optional components must degrade safely and must not break the core recording and text-injection path.
- Lightweight configuration UI may stay in the shell; binary runtimes, model weights, and substantial feature assets belong in signed manifest-driven components.

## 2026-07-15 — Rename the Chinese product to 雨音输入法

Decision:
- Supersede the earlier Chinese display name with `雨音输入法`; use `Rain Vibetype` for human-facing English text while retaining the `Rain-VibeType` repository name.

Context:
- The user selected the shorter Chinese name and requested a verified Chinese/English switch.

Rationale:
- The new name is concise and connects the creator's name with voice input without changing the established Rain identity.

Consequences:
- Tauri product/window metadata, frontend branding, tray tooltip, native dialogs, README, and future installer filenames use `雨音输入法`.
- The Tauri identifier and GitHub URLs stay unchanged, preserving existing settings, models, and update compatibility.

## 2026-07-15 — Add text polishing as two optional CPU components

Decision:
- Add Qwen3 0.6B Q8_0 GGUF and the official llama.cpp Windows CPU runtime as independent optional downloads. Keep the feature disabled by default and preserve the ASR result on every failure.

Context:
- The user requested fast, accurate local text organization while requiring every substantial model and feature to remain modular and removable.

Rationale:
- The current 0.6B model is already small enough for CPU use. A separate CUDA text runtime would add substantial download and maintenance cost without evidence that this pass needs GPU acceleration.

Consequences:
- The installer contains only metadata and UI for the feature; the 639,446,688-byte model and 18,271,892-byte runtime archive transfer only after explicit clicks.
- The validator permits punctuation, whitespace, optional paragraphs, and explicitly enabled filler removal, but rejects changed body characters, numbers, ASCII tokens, or protected terms.
- Missing components, startup failure, a 30-second load timeout, an 8-second request timeout, or validation failure all return the original recognition text to the existing injection path.

## 2026-07-15 — Separate settings by task instead of reducing capability

Decision:
- Keep all settings, but give recording, text insertion, feedback, model library, text cleanup, inference, application, and privacy their own navigation pages.
- Collapse non-current model details with native disclosure controls.

Context:
- Combining several settings domains into a few pages required excessive vertical scrolling.

Rationale:
- Task-level pages preserve modular control while keeping each page short and scannable.

Consequences:
- The left rail has nine entries and becomes independently scrollable on short windows.
- The current or downloading model opens automatically; other models show only their identity and state until expanded.

## 2026-07-15 — Make Home a voice-input test workspace

Decision:
- Show the current hotkey and model once, then use the remaining Home area for an editable test box and a mirrored result box.

Context:
- Input method, privacy, and repeated model summaries did not help users test the product.

Rationale:
- Focusing a native textarea exercises the real global-shortcut and text-injection path without a separate test backend.

Consequences:
- Test text exists only in the page DOM and is not persisted.
- The result box mirrors direct paste, simulated typing, and manual edits through the textarea's native input event.

## 2026-07-15 — Gate native recognition by corpus CER, not exact text identity

Decision:
- Accept the staged native SenseVoice path when its normalized character error rate is no more than 0.5 percentage points worse than Python/FunASR on the chosen corpus; do not require every hypothesis to be identical.

Context:
- The native and Python frontends differ in LFR boundary padding and dither. A deterministic 200-clip AISHELL-1 comparison passed twice even though 17 hypotheses differed.

Rationale:
- Recognition quality against reference transcripts is the product outcome. Maintaining a sherpa-onnx fork solely to reproduce another frontend would add release and cross-platform risk without improving the measured result.

Consequences:
- Keep the current sherpa-onnx frontend while the quality gate passes.
- The clean-read result is preliminary; production selection still needs real microphone, spontaneous-dictation, and noisy-speech data plus official model artifacts.

## 2026-07-15 — Make native SenseVoice the default

Decision:
- New SenseVoice installations use the separately downloaded Rust/sherpa-onnx CPU Worker and unquantized ONNX model. Python/FunASR remains for the other ASR adapters and legacy SenseVoice directories that do not contain native model files.

Context:
- The user accepted the repeated 200-clip CER result and explicitly requested the native path now become the default.

Rationale:
- The native path is smaller, loads faster, recognizes faster, and passed the agreed accuracy gate twice. Reusing the existing Worker protocol and component repository avoids a second application pipeline.

Consequences:
- The base installer remains lightweight; the native runtime and model still require an explicit download.
- Release builds must publish and verify the native runtime ZIP, `model.onnx`, `tokens.txt`, and combined manifests.
- SenseVoice uses CPU even when NVIDIA is present; selecting Fun-ASR Nano or Paraformer-zh continues to use the configured CPU/NVIDIA Python component.

## 2026-07-15 — Make inference runtimes model-owned dependencies

Decision:
- Map every built-in local model to its preferred runtime in the model manifest. A model download automatically installs its missing runtime; the UI has no independent runtime download control.
- Prevent removal while any installed model references a runtime. Deleting the last referencing model automatically removes that runtime.

Context:
- Future built-in models may use different optimized frameworks, and users should choose models rather than manage framework packages manually.

Rationale:
- Deriving runtime ownership from installed model markers keeps the workflow one-click and avoids orphaned multi-gigabyte frameworks without introducing a second package database.

Consequences:
- `models.json` uses schema 2 and runtime mappings are required for every model definition.
- Shared runtimes such as the FunASR CPU/NVIDIA packages remain installed until both Fun-ASR Nano and Paraformer-zh are gone.
- Normal startup does not refresh a remote runtime catalog; the model download action is the explicit consent boundary for both runtime and weights.

## 2026-07-16 — Keep live preview separate from final transcription

Decision:
- Use an optional bilingual streaming Zipformer in a second native Worker for partial overlay text, while SenseVoice remains the sole final transcription whose result is injected.

Context:
- The overlay needs responsive text during recording, but changing or repeatedly rerunning the accepted final model would risk latency and output quality.

Rationale:
- A roughly 57 MiB quantized streaming model produces incremental hypotheses on CPU. Separate Worker state makes preview failure non-fatal and prevents unstable partial hypotheses from contaminating final output.

Consequences:
- The preview component is an explicit optional model download with purpose `asr_preview`; it cannot be selected as the final model.
- Both models reuse native runtime version 1.1.0, whose adapter list includes `sensevoice` and `streaming_zipformer`.
- Missing preview assets, load failure, or decode failure only suppresses preview and records a diagnostic; final SenseVoice transcription and injection continue unchanged.
