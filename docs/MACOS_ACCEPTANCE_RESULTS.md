# macOS Apple Silicon acceptance results

Last run: 2026-07-17

This report records evidence for the current uncommitted Apple Silicon port. It
does not replace `MACOS_ACCEPTANCE.md`, and pending rows must not be treated as
passed.

## Test candidate

- Base Git HEAD: `cd9796d24cd6779f4b9f39526784dd872d639375`
- Branch: `main`
- Worktree: contains the uncommitted macOS port
- Host: MacBook Pro (`MacBookPro18,1`), Apple M1 Pro, 16 GB
- OS: macOS 27.0, build `26A5378n`
- Architecture: `arm64`
- Runtime manifest: `1.0.0-aarch64-apple-darwin`
- Test models: upstream sherpa-onnx SenseVoice int8 imported as an explicitly
  unverified local custom model, plus the verified Fun-ASR-Nano 2512 catalog
  model

## Release artifacts

- App: `src-tauri/target/aarch64-apple-darwin/release/bundle/macos/雨音输入法.app`
- DMG: `src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/雨音输入法_1.0.0_aarch64.dmg`
- DMG SHA-256: `c6e563b5bb5ca1dfb766ff588d45aa56ba44adcfbbc84ec68c4e2fdb81e350da`
- Runtime ZIP: `artifacts/runtimes/macos/rain-runtime-onnx-cpu-aarch64-apple-darwin.zip`
- Runtime SHA-256: `a22a79653f13a6787f875cc4e9c9a0f46eb946676151d5afce8ffc7b397fd6fb`
- FunASR Runtime TAR: `artifacts/runtimes/macos/rain-runtime-cpu-1.0.0-aarch64-apple-darwin.tar.gz`
- FunASR Runtime SHA-256: `b80fbc63f9318c152f81222186d0b52cc5d9d5de27183a77aa6747964fdb6181`

## Passed evidence

| Area | Evidence |
| --- | --- |
| Rust core | 35 tests passed; one live-network test intentionally ignored |
| Lint | macOS and `x86_64-pc-windows-gnu` Clippy passed with `-D warnings` |
| Native bridge | Objective-C passed `-Wall -Wextra -Werror -Wunguarded-availability` with macOS 12 minimum |
| Worker contracts | Native Worker test passed; five Python Worker tests passed |
| Package | Main executable and Worker are arm64 Mach-O; strict code-sign verification passed; final app contains the hardened-runtime audio-input entitlement; DMG checksum verification passed |
| Package contents | No model weights, Python runtime, CUDA files, or optional Worker are bundled in the base app |
| Mac UI | Native Tauri window rendered Apple Silicon status, macOS menu-bar text, localized accessibility labels, no CUDA option, and cross-platform SVG navigation/status icons without missing-glyph boxes |
| Keyboard | Default is `Super+Shift+Space`, displayed `⌘ ⇧ Space`; Command+Option+R records successfully; Escape preserves the previous shortcut |
| Option key handling | Native WKWebView emitted `®` for Option+R; recorder now normalizes letters and digits from `KeyboardEvent.code` |
| Microphone | CoreAudio 0.5-second input test opened the MacBook Pro microphone successfully |
| Runtime discovery | Release app discovered the locally installed signed arm64 Runtime and reported ready |
| FunASR model-library UI | `/Applications/雨音输入法.app` now shows Fun-ASR-Nano as `已安装` with `2.0 GB / 2.0 GB` and no `RUNTIME_NOT_FOUND` banner |
| ASR | Signed arm64 Worker loaded SenseVoice int8 and transcribed a 5.616-second Chinese sample as `开放时间早上9点至下午5点。` in 161 ms |
| Fun-ASR-Nano | The installed signed arm64 compatibility Worker loaded Fun-ASR-Nano 2512 and transcribed a 3.278-second Chinese sample as `开放时间：早上九点至下午五点。` in 3154 ms |
| Shared compatibility runtime | Fun-ASR-Nano and Paraformer both resolve their macOS CPU dependency to `rain-runtime-cpu`; the packaged Worker exposes both adapters |
| Worker lifecycle | App health check started the managed Worker; quitting Rain terminated both the app and Worker without an orphan process |
| Packaged Python Worker lifecycle | `multiprocessing.freeze_support()` prevents PyInstaller child re-entry; protocol shutdown exited within 30 seconds with one `worker_ready` event and no residual process |
| Clipboard snapshot | Two pasteboard items containing plain text, RTF, and a custom type were restored byte-for-byte |
| Concurrent clipboard copy | A simulated new user copy was preserved instead of being overwritten by Rain's delayed restore |
| Target mismatch | With Rain frontmost and TextEdit recorded as the stale target, paste and typing both returned target-changed status before modifying the clipboard |
| Audio ducking | Default output volume was reduced to 20% and restored to the prior scalar value |
| Overlay bridge | Status window level, all-Spaces/full-screen behavior, click-through visual overlay, interactive cancel window, and hide behavior passed native checks |
| Close behavior | Closing the main window hid it while the Rain process remained resident in the menu bar |
| Privacy diagnosis | TCC logs proved that an enabled-looking Accessibility row can retain a stale ad-hoc CDHash; the accepted recovery is to finish the build, reset only the affected bundle grant, and re-authorize that exact build |

## Pending evidence

| Area | Reason |
| --- | --- |
| Current Accessibility regrant | The exact current app and `/Applications` copy both have CDHash `2813947cfd413ba3b7b7ff9affe6759fc2e708e5`; Accessibility must be physically verified for this build before the next hotkey run. |
| Physical global hotkey | It passed on the preceding ad-hoc build, but must be repeated after authorizing the current final CDHash. |
| End-to-end live speech into TextEdit | TextEdit is prepared with `Rain 跨应用输入测试：`; physical hotkey input is still required. |
| Cross-app matrix | Safari, Chrome, VS Code, Terminal, and Notes still require physical hotkey runs. |
| Target switch during recognition | Native PID rejection is proven, but the complete UI flow still needs a physical recording followed by an app switch. |
| Mixed-display/full-screen placement | Requires an external display, mixed scale factors, Spaces, Stage Manager, and a native full-screen app. |
| Public distribution | Current package is ad-hoc signed. Developer ID signing, notarization, Gatekeeper acceptance, and a clean-machine launch remain pending. |
| Public runtime/model download | GitHub Release runtime manifest, both macOS runtime archives, model ONNX, and tokens have not been uploaded. Local installation is verified, but a clean machine still cannot fetch these release assets. |

## Current local test state

- The main app was not rebuilt while fixing the FunASR compatibility Worker,
  so its CDHash and existing TCC identity were preserved.
- SenseVoice, Fun-ASR-Nano, and their signed runtimes are installed only under
  Rain's Application Support directory; none is present in the app or DMG.
