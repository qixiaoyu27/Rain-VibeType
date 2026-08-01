# macOS Apple Silicon acceptance gate

Record the tested macOS version, Mac model/chip, Rain commit, runtime manifest
version, and model version for every release candidate.

## Automated gate

```bash
cargo test --all-targets --manifest-path src-tauri/Cargo.toml
cargo clippy --all-targets --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --all-targets --manifest-path native-worker/Cargo.toml
python3 -m unittest worker.test_worker -v
node --check src/main.js
./scripts/build-macos-runtime.sh
npm run build -- --target aarch64-apple-darwin
```

Verify the produced main executable and native Worker are both arm64:

```bash
file src-tauri/target/aarch64-apple-darwin/release/bundle/macos/*.app/Contents/MacOS/*
file native-worker/target/release/rain-native-worker
```

## Installation and permission flow

- Install from a clean `.dmg` into `/Applications`.
- Confirm the first microphone request uses Rain's localized explanation.
- Trigger the hotkey before Accessibility permission is granted. Rain must
  prompt, must not inject text, and must explain the System Settings path.
- Grant Accessibility permission and confirm recording works without restart;
  if macOS requires a restart, the UI must state that clearly.
- Remove each permission and verify Rain fails closed with text recoverable
  from the clipboard or recovery panel.

## Keyboard and recording

- The default shortcut displays as `⌘ ⇧ Space` and registers as
  `Super+Shift+Space`.
- Test push-to-talk press/release and toggle mode.
- Record custom shortcuts using Command, Option, Control, Shift, function
  keys, arrows, and Space. Escape must cancel shortcut recording and active
  speech capture.
- Hold modifiers slightly longer than the speech key; paste must still be
  plain `Command+V`, never Paste and Match Style or another chord.
- Verify microphone selection, disconnect handling, 0.5-second level test,
  maximum-duration stop, cancel button, and tray/menu-bar pause.

## Target safety and text injection

Test secure paste and simulated typing in:

- TextEdit
- Safari
- Chromium/Chrome
- VS Code
- Terminal
- Notes or another AppKit text editor

For each app:

- Start and finish without switching apps: text enters the original target.
- Switch to a different process while recognizing: no text enters either app;
  the result is copied only.
- Switch between windows in the same process: behavior remains process-safe
  and does not redirect to a different application.
- Revoke Accessibility during a run: injection stops safely.

## Clipboard integrity

Before recognition, copy and verify restoration of:

- Plain and rich text
- An image
- Finder files
- Multiple pasteboard items
- App-specific pasteboard types when available

Copy new content while recognition is finishing. Rain must not overwrite that
new content. If a complete snapshot cannot be made, Rain must refuse the paste
before replacing the clipboard.

## Overlay and macOS window behavior

- Overlay appears at the bottom center of the target screen without activating
  Rain or moving keyboard focus.
- The visual overlay is click-through; the cancel window remains clickable.
- Test the built-in Retina display, an external display, mixed scale factors,
  Spaces, Stage Manager, and a native full-screen app.
- Respect the “show in full-screen apps” setting.
- Terminal status hides after its timeout and no invisible window captures
  clicks.

## Audio, models, and workers

- Start/stop/error sounds play through the current output device.
- Audio ducking lowers to 20% and restores the exact prior volume after stop,
  cancel, microphone failure, model failure, and app quit.
- Download and verify the Apple Silicon runtime manifest and ZIP.
- Load SenseVoice, transcribe a real recording, unload, and reload.
- Verify optional streaming preview and llama.cpp Metal text cleanup.
- Quit while the ASR worker and text-polish server are running; no child
  process may remain.

## Release artifact

- `.app` and `.dmg` contain no model weights, Python environment, CUDA files,
  or downloaded optional runtimes.
- `codesign --verify --deep --strict` succeeds for the installed app.
- `spctl --assess --type execute` succeeds after notarization.
- Launch the notarized app on a clean Apple Silicon Mac and repeat the core
  hotkey, permission, clipboard, and quit checks.
