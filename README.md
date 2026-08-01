<div align="center">

<img src="src/assets/rain.svg" width="112" alt="雨音输入法 Logo" />

# 雨音输入法 · Rain Vibetype

### 按住快捷键，说完即写入。

一款本地优先的 Windows 11 / macOS 语音输入工具。录音、识别、文本整理和跨应用写入都在你的电脑上完成。

[![Release](https://img.shields.io/github/v/release/qixiaoyu27/Rain-VibeType?display_name=tag&style=flat-square&color=0e7490)](https://github.com/qixiaoyu27/Rain-VibeType/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/qixiaoyu27/Rain-VibeType/total?style=flat-square&color=0e7490)](https://github.com/qixiaoyu27/Rain-VibeType/releases)
[![Windows](https://img.shields.io/badge/Windows%2011-x64-0e7490?style=flat-square&logo=windows11&logoColor=white)](#windows-11)
[![macOS](https://img.shields.io/badge/macOS-Apple%20Silicon-0e7490?style=flat-square&logo=apple&logoColor=white)](#macos-apple-silicon)
[![License](https://img.shields.io/github/license/qixiaoyu27/Rain-VibeType?style=flat-square&color=0e7490)](LICENSE)

[**下载最新版**](https://github.com/qixiaoyu27/Rain-VibeType/releases/latest) · [快速开始](#快速开始) · [功能](#核心能力) · [模型](#可选模型) · [开发](#本地开发)

</div>

---

## 为什么是雨音

| 你在意的事 | 雨音的做法 |
| --- | --- |
| **隐私** | 音频和识别文本只在本机处理，不上传、不保存输入历史。 |
| **说完即写入** | 松开快捷键后将结果写入原光标位置，无需手动复制粘贴。 |
| **不误输** | 录音前记住目标应用，写入前再次复核；目标改变时仅保留到剪贴板。 |
| **轻量安装** | 基础应用不内置模型和大型运行时，只下载你选择的组件。 |
| **自由选择** | 支持多种本地语音模型、实时预览和可选的本地文本整理。 |

```text
全局快捷键 → 内存录音 → 本地识别 → 可选文本整理 → 目标复核 → 安全写入
```

## 快速开始

### Windows 11

1. 从 [Releases](https://github.com/qixiaoyu27/Rain-VibeType/releases/latest) 下载 `Rain-Vibetype_*_x64-setup.exe` 并安装。
2. 启动雨音，按引导下载需要的模型和推理组件。
3. 将光标放到任意输入框，按住 `Ctrl + Shift + Space` 说话，松开后即可写入。

> [!NOTE]
> 当前 Windows 版未使用商业代码签名证书。请只从本仓库 Releases 下载，并在 SmartScreen 提示时先核对文件 SHA-256。

### macOS Apple Silicon

1. 从 [Releases](https://github.com/qixiaoyu27/Rain-VibeType/releases/latest) 下载 `.dmg`，将雨音拖入“应用程序”。
2. 允许麦克风，并在“系统设置 → 隐私与安全性 → 辅助功能”中允许雨音。
3. 按引导下载 Apple Silicon 原生推理组件和模型。
4. 将光标放到任意输入框，按住 `⌘ + ⇧ + Space` 说话，松开后即可写入。

> [!IMPORTANT]
> 当前 macOS 版使用本地签名，尚未经过 Apple 公证。如 Gatekeeper 拦截，核对 SHA-256 后可仅解除雨音自身的隔离标记：
>
> ```bash
> sudo xattr -dr com.apple.quarantine "/Applications/雨音输入法.app"
> open "/Applications/雨音输入法.app"
> ```
>
> 该命令不会关闭系统的全局 Gatekeeper。当前仅支持 Apple Silicon，不支持 Intel Mac。

## 核心能力

| 模块 | 行为 |
| --- | --- |
| 本地识别 | 默认使用 Rust / sherpa-onnx 的 SenseVoice Worker，也可选择 Fun-ASR Nano 或 Paraformer-zh。 |
| 录音模式 | 支持按住说话和按键开关；`Esc` 或悬浮取消按钮可随时中止。 |
| 实时预览 | 可选中英双语流式预览；预览不会取代最终模型识别。 |
| 安全写入 | 默认安全粘贴，也可选 Unicode 模拟输入；目标改变时不误输。 |
| 剪贴板保护 | 写入前完整快照；用户期间复制了新内容时，雨音不会覆盖。 |
| 本地文本整理 | 可选 Qwen3 0.6B + llama.cpp 处理标点、分段和轻量润色；校验失败时保留原文。 |
| 桌面体验 | 托盘常驻、开机启动、模型按需加载、录音时系统音量压低、中英文界面。 |

## 可选模型

| 模型 | 用途 | 特点 |
| --- | --- | --- |
| SenseVoice Small | 默认语音识别 | 支持中文、English、粤语、日本语、한국어；使用原生 sherpa-onnx Worker。 |
| Fun-ASR Nano | 多语种语音识别 | 支持中文、English、日本语和中文方言；能力更完整，资源占用也更高。 |
| Paraformer-zh | 中英语音识别 | 偏中文场景，通过 FunASR Worker 运行。 |
| Streaming Zipformer | 实时文字预览 | 轻量中英双语流式模型，不替代最终识别。 |
| Qwen3 0.6B | 可选文本整理 | 通过 llama.cpp 在本地调整标点、分段和轻量表达。 |

基础应用不捆绑模型、PyTorch、CUDA 或 llama.cpp。用户首次下载某个模型时，雨音会自动获取当前平台所需的最小推理组件，并校验文件大小与 SHA-256。

## 本地性能

默认 SenseVoice 路径由常驻的原生 Worker 执行。模型加载后，识别不经过云端请求，连续输入也不需要重复初始化。

| 测试环境 | 模型与输入 | 结果 |
| --- | --- | --- |
| MacBook Pro · Apple M1 Pro · 16 GB | SenseVoice Small ONNX；5.616 秒中文音频；热模型连续 10 次 | 中位数 **107.5 ms**，约 **52.2× 实时速度** |

单独启动 Worker 时的模型加载耗时为 **790 ms**。数据只统计模型推理，不包含录音、首次下载、应用启动和最终写入；不同硬件、模型和音频会有差异。

## 平台与安全边界

macOS 与 Windows 共用录音、识别、目标复核和剪贴板保护状态机；平台层分别使用 AppKit / Accessibility / CoreGraphics 和 Win32 API。

- 录音和识别文本不上传、不保存历史。
- 模型和推理组件仅在用户明确发起下载时联网。
- 录音完成后会再次核对目标应用，避免把文字写入错误窗口。
- 剪贴板仅在雨音写入的内容仍未被用户替换时恢复。
- 文本整理失败、超时或改变事实时，保留原始识别结果。
- 诊断包由用户主动导出，不作为默认数据采集渠道。

## 本地开发

### Windows 11

```powershell
npm install
.\scripts\setup-worker.ps1
npm run dev
```

`setup-worker.ps1` 会把项目虚拟环境加入当前 PowerShell 会话的 `PATH`，请在同一终端启动应用。Windows 发布构建使用 `scripts/release.ps1`。

### macOS Apple Silicon

```bash
./scripts/run-macos.sh
```

构建 Apple Silicon 原生 Worker、应用和 DMG：

```bash
./scripts/build-macos.sh
```

### 质量检查

```powershell
cargo fmt --check --manifest-path .\src-tauri\Cargo.toml
cargo test --all-targets --manifest-path .\src-tauri\Cargo.toml
cargo clippy --all-targets --manifest-path .\src-tauri\Cargo.toml -- -D warnings
cargo test --all-targets --manifest-path .\native-worker\Cargo.toml
python -m unittest worker.test_worker -v
node .\scripts\check-frontend.mjs
```

## 许可证

Rain Vibetype 采用 [GNU Affero General Public License v3.0](LICENSE)（`AGPL-3.0-only`）。
