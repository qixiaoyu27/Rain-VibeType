<div align="center">

<img src="src/assets/rain.svg" width="112" alt="雨音输入法 Logo" />

# 雨音输入法 · Rain Vibetype

### 按住快捷键，说完即写入。

本地优先的 Windows 11 语音输入工具：录音、识别与文本处理都在本机完成。

[![Release](https://img.shields.io/github/v/release/qixiaoyu27/Rain-VibeType?display_name=tag&style=flat-square&color=0e7490)](https://github.com/qixiaoyu27/Rain-VibeType/releases)
[![Downloads](https://img.shields.io/github/downloads/qixiaoyu27/Rain-VibeType/total?style=flat-square&color=0e7490)](https://github.com/qixiaoyu27/Rain-VibeType/releases)
[![Platform](https://img.shields.io/badge/Windows%2011-x64-0e7490?style=flat-square&logo=windows11&logoColor=white)](#系统要求)
[![License](https://img.shields.io/github/license/qixiaoyu27/Rain-VibeType?style=flat-square&color=0e7490)](LICENSE)

[下载测试版](https://github.com/qixiaoyu27/Rain-VibeType/releases/latest) · [快速开始](#快速开始) · [隐私](#隐私承诺) · [参与开发](#参与开发)

</div>

---

## 为什么是 Rain

| 你在意的事 | Rain 的做法 |
| --- | --- |
| 隐私 | 音频和识别文本只在内存与本地模型中处理，不上传、不保存历史。 |
| 不打断工作 | 录音前捕获目标应用；完成后仅在仍是同一应用时写入，否则安全地保留到剪贴板。 |
| 剪贴板安全 | 粘贴前完整快照；用户期间复制了新内容时，绝不覆盖。 |
| 轻量安装 | 基础安装包不塞入模型、PyTorch、CUDA 或 llama.cpp；仅在你明确点击下载后获取所需组件。 |
| 本地性能 | 默认使用 Rust / sherpa-onnx 的 SenseVoice Worker；Fun-ASR 与 Paraformer 仍可按需选择。 |

```text
全局快捷键 → 内存录音 → 本地识别 → 目标校验 → 安全写入
```

## 快速开始

1. 从 [Releases](https://github.com/qixiaoyu27/Rain-VibeType/releases/latest) 下载 `雨音输入法_*_x64-setup.exe`。
2. 安装后启动 Rain，按引导选择并下载模型。
3. 将光标放到任意输入框，按住 `Ctrl + Shift + Space` 说话；松开后文字会写入当前应用。

> 当前首个 Windows 预览版未使用商业代码签名证书。请只从本仓库 Releases 下载；若 Windows SmartScreen 提示，请先核对发布者和文件 SHA-256，再自行决定是否运行。

## 功能一览

- 按住说话与按键开关两种录音模式；`Esc`、悬浮取消按钮均可中止。
- SenseVoice Small、Fun-ASR Nano、Paraformer-zh 三种本地识别适配器。
- 可选中英双语流式预览：仅显示录音过程的局部文本，最终结果仍由主模型独立完成。
- 默认安全粘贴，也可选择 Unicode 模拟输入。
- 可选 Qwen3 0.6B + llama.cpp 本地文本整理；失败时自动保留原始识别结果。
- 中文默认界面，同时支持跟随系统和 English。
- 托盘常驻、开机启动、模型按需加载与卸载、诊断包和可选崩溃报告。

## 系统要求

- Windows 11 x64
- Intel / AMD CPU；NVIDIA 显卡可选，用于 CUDA 推理组件
- 麦克风、WebView2，以及首次下载模型时的网络连接
- 建议至少预留 10 GB 磁盘空间；实际取决于所选模型与推理组件

## 测试版说明

此仓库的首个 Release 先提供 Windows 安装包，便于体验界面、热键、录音和基础流程。

完整离线识别还依赖单独发布的原生运行时与模型资产。它们不会被静默下载，也不会被打进安装包；发布前会进行 SHA-256 校验与干净机器验证。请以 Release 页面列出的可用资产和说明为准。

## 隐私承诺

- 不保存录音、识别文本、剪贴板内容或窗口标题。
- 模型和推理组件仅在用户明确发起下载时联网。
- 剪贴板恢复只在 Rain 自己写入的内容仍未被用户替换时进行。
- 模型删除仅作用于 Rain 管理的目录。

## 参与开发

```powershell
npm install
.\scripts\setup-worker.ps1
npm run dev
```

常用检查：

```powershell
cargo test --all-targets --manifest-path .\src-tauri\Cargo.toml
cargo clippy --all-targets --manifest-path .\src-tauri\Cargo.toml -- -D warnings
cargo test --all-targets --manifest-path .\native-worker\Cargo.toml
python -m unittest worker.test_worker -v
node --check .\src\main.js
```

macOS Apple Silicon 的迁移边界见 [MACOS_MIGRATION.md](docs/MACOS_MIGRATION.md)。

## License

本项目采用 [GNU Affero General Public License v3.0](LICENSE)（`AGPL-3.0-only`）。
