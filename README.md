<div align="center">

<img src="src/assets/rain.svg" width="112" alt="雨音输入法 Logo" />

# 雨音输入法 · Rain Vibetype

### 按住快捷键，说完即写入。

本地优先的桌面语音输入工具。录音、识别、文本整理和写入都在你的电脑上完成。

[![Release](https://img.shields.io/github/v/release/qixiaoyu27/Rain-VibeType?display_name=tag&style=flat-square&color=0e7490)](https://github.com/qixiaoyu27/Rain-VibeType/releases)
[![Downloads](https://img.shields.io/github/downloads/qixiaoyu27/Rain-VibeType/total?style=flat-square&color=0e7490)](https://github.com/qixiaoyu27/Rain-VibeType/releases)
[![Windows](https://img.shields.io/badge/Windows%2011-x64-0e7490?style=flat-square&logo=windows11&logoColor=white)](#下载与快速开始)
[![macOS](https://img.shields.io/badge/macOS-Apple%20Silicon-0e7490?style=flat-square&logo=apple&logoColor=white)](#下载与快速开始)
[![License](https://img.shields.io/github/license/qixiaoyu27/Rain-VibeType?style=flat-square&color=0e7490)](LICENSE)

[下载](#下载与快速开始) · [功能](#核心能力) · [模型](#支持的模型) · [隐私](#隐私与安全边界) · [开发](#开发)

</div>

---

## 下载与快速开始

| 平台 | 系统要求 | 默认快捷键 | 当前版本 |
| --- | --- | --- | --- |
| macOS | macOS 12+、Apple Silicon M1 或更新芯片 | `⌘ + ⇧ + Space` | [v1.1.0 Preview 1](https://github.com/qixiaoyu27/Rain-VibeType/releases/tag/v1.1.0-mac.1) |
| Windows | Windows 11 x64 | `Ctrl + Shift + Space` | [v1.0.0 测试版](https://github.com/qixiaoyu27/Rain-VibeType/releases/tag/v1.0.0) |

### macOS Apple Silicon

1. 下载 [`Rain-VibeType-1.1.0-macOS-arm64.dmg`](https://github.com/qixiaoyu27/Rain-VibeType/releases/download/v1.1.0-mac.1/Rain-VibeType-1.1.0-macOS-arm64.dmg)。
2. 打开 DMG，将“雨音输入法”拖入“应用程序”。
3. 首次启动时允许麦克风；第一次录音前，在“系统设置 → 隐私与安全性 → 辅助功能”中允许雨音输入法。
4. 按引导下载模型，把光标放入任意输入框，按住 `⌘ + ⇧ + Space` 说话。

> [!WARNING]
> macOS 版本目前是 Preview，仅支持 Apple Silicon。安装包使用临时签名，尚未经过 Apple Developer ID 签名和公证；若 Gatekeeper 阻止启动，请在 Finder 中右键应用并选择“打开”，或在“系统设置 → 隐私与安全性”中允许打开。

### Windows 11

1. 下载并安装 [`Rain-Vibetype_1.0.0_x64-setup.exe`](https://github.com/qixiaoyu27/Rain-VibeType/releases/download/v1.0.0/Rain-Vibetype_1.0.0_x64-setup.exe)。
2. 启动 Rain，按引导选择并下载模型。
3. 把光标放入任意输入框，按住 `Ctrl + Shift + Space` 说话。

> [!NOTE]
> Windows v1.0.0 是首个公开测试版，主要用于界面、热键、录音和基础流程验证；安装包未使用商业代码签名证书。请只从本仓库 Releases 下载并核对 SHA-256。

## Rain 是什么

Rain 是一个系统级语音输入工具，不是云端听写服务。它在录音开始时记住目标应用，在本地完成识别，结束后再次确认目标仍然安全，才把文字写回原来的输入框。

```text
全局快捷键 → 内存录音 → 本地识别 → 可选文本整理 → 目标复核 → 安全写入
```

基础安装包不捆绑模型、PyTorch、CUDA 或 llama.cpp。只有在你明确点击下载后，Rain 才会获取所选模型和运行时，并在落盘前校验文件大小与 SHA-256。

## 核心能力

| 模块 | 行为 |
| --- | --- |
| 录音 | 支持按住说话和按键开关两种模式；`Esc` 或悬浮取消按钮可随时中止。 |
| 本地识别 | 默认使用 Rust / sherpa-onnx 的 SenseVoice Worker，也可选择 Fun-ASR Nano 或 Paraformer-zh。 |
| 实时预览 | 可选中英双语流式预览；预览只负责即时反馈，最终结果由主模型独立生成。 |
| 安全写入 | 写入前复核前台应用；目标变化时不误输，结果保留到剪贴板。 |
| 剪贴板保护 | 粘贴前完整快照；如果用户期间复制了新内容，Rain 不会覆盖。 |
| 文本整理 | 可选 Qwen3 0.6B + llama.cpp 本地整理；校验失败时保留原始识别结果。 |
| 桌面体验 | 托盘常驻、开机启动、模型按需加载、中文/English、诊断包和可选崩溃报告。 |

## 支持的模型

| 模型 | 用途 | 特点 |
| --- | --- | --- |
| SenseVoice Small | 默认语音识别 | 中文、English、粤语、日本語、한국어；原生 sherpa-onnx Worker。 |
| Fun-ASR Nano | 多语种语音识别 | 中文、English、日本語和中文方言；能力更完整，占用也更高。 |
| Paraformer-zh | 中英语音识别 | 偏中文场景，可通过 FunASR Worker 运行。 |
| Streaming Zipformer | 实时文字预览 | 轻量中英双语流式模型，不替代最终识别。 |
| Qwen3 0.6B | 可选文本整理 | 通过 llama.cpp 在本地调整标点、分段和轻量表达。 |

模型和运行时独立于应用发布。Rain 只下载当前平台需要的组件；已安装模型可在应用中卸载，删除范围只限 Rain 管理的目录。

## 隐私与安全边界

- 录音和识别文本只在内存与本地模型中处理，不上传、不保存历史。
- 模型和推理组件仅在用户明确发起下载时联网。
- 录音完成后会再次核对目标应用，避免把文字写进错误窗口。
- 剪贴板仅在 Rain 写入的内容仍未被用户替换时恢复。
- 文本整理失败、超时或改变事实时，保留原始识别结果。
- 诊断包和崩溃报告由用户主动触发，不作为默认数据采集渠道。

## 工作方式

Rain 把平台差异收敛在桌面宿主层，上层共用同一套录音、识别和安全写入状态机：

| 层 | 职责 |
| --- | --- |
| `src/` | 设置、模型管理、录音状态和悬浮窗界面。 |
| `src-tauri/` | 全局快捷键、音频输入、窗口、权限、下载校验与安全写入。 |
| `native-worker/` | Rust 原生 sherpa-onnx 语音 Worker。 |
| `worker/` | FunASR / Paraformer Python 兼容 Worker。 |
| `scripts/` | 本地开发、运行时构建和发布脚本。 |

Windows 平台层使用 Win32 API；macOS 平台层使用 AppKit、Accessibility、CoreGraphics 和系统音频能力。

## 常见问题

### macOS 提示无法打开怎么办？

当前 Preview 尚未公证。先确认文件来自本仓库 Release，再在 Finder 中右键“雨音输入法”并选择“打开”；也可以前往“系统设置 → 隐私与安全性”允许打开。不要全局关闭 Gatekeeper。

### Intel Mac 可以使用吗？

不可以。当前 macOS 版本只构建和验证了 Apple Silicon arm64。

### 为什么 Windows 会显示 SmartScreen 提示？

Windows 测试版尚未使用商业代码签名证书。Release 页面提供安装包 SHA-256，确认来源和哈希后再决定是否运行。

### 日常识别需要联网吗？

模型和运行时安装完成后，语音识别与可选文本整理都在本机执行。首次下载组件、检查更新时需要网络连接。

### macOS 源码在哪里？

macOS 实现维护在 [`Mac` 分支](https://github.com/qixiaoyu27/Rain-VibeType/tree/Mac)，包括构建脚本、迁移边界和验收记录。

## 开发

### Windows / `main`

```powershell
npm install
.\scripts\setup-worker.ps1
npm run dev
```

### macOS Apple Silicon / `Mac`

```bash
git switch Mac
npm install
./scripts/run-macos.sh
```

生成 macOS `.app`、DMG 和 Apple Silicon 运行时：

```bash
./scripts/build-macos.sh
```

通用检查：

```bash
cargo test --all-targets --manifest-path src-tauri/Cargo.toml
cargo clippy --all-targets --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --all-targets --manifest-path native-worker/Cargo.toml
python -m unittest worker.test_worker -v
node --check src/main.js
```

macOS 相关文档：

- [迁移边界](https://github.com/qixiaoyu27/Rain-VibeType/blob/Mac/docs/MACOS_MIGRATION.md)
- [验收清单](https://github.com/qixiaoyu27/Rain-VibeType/blob/Mac/docs/MACOS_ACCEPTANCE.md)
- [验收记录](https://github.com/qixiaoyu27/Rain-VibeType/blob/Mac/docs/MACOS_ACCEPTANCE_RESULTS.md)

## 参与项目

欢迎提交 Issue 和 Pull Request。报告问题时请附上操作系统、Rain 版本、复现步骤和经过脱敏的诊断信息；不要上传录音、识别文本、API Key 或其他隐私数据。

## License

Rain Vibetype 采用 [GNU Affero General Public License v3.0](LICENSE)（`AGPL-3.0-only`）。修改并分发本项目，或通过网络提供修改后的版本时，需要按 AGPLv3 提供对应源代码。
