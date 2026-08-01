<div align="center">

<img src="src/assets/rain.svg" width="112" alt="雨音输入法 Logo" />

# 雨音输入法 · Rain Vibetype

### 按住快捷键，说完即写入。

本地优先的桌面语音输入工具。录音、识别、文本整理和写入都在你的电脑上完成。

[![Release](https://img.shields.io/github/v/release/qixiaoyu27/Rain-VibeType?display_name=tag&style=flat-square&color=0e7490)](https://github.com/qixiaoyu27/Rain-VibeType/releases)
[![Downloads](https://img.shields.io/github/downloads/qixiaoyu27/Rain-VibeType/total?style=flat-square&color=0e7490)](https://github.com/qixiaoyu27/Rain-VibeType/releases)
[![Windows](https://img.shields.io/badge/Windows%2011-x64-0e7490?style=flat-square&logo=windows11&logoColor=white)](#获取版本)
[![macOS](https://img.shields.io/badge/macOS-Apple%20Silicon-0e7490?style=flat-square&logo=apple&logoColor=white)](#获取版本)
[![License](https://img.shields.io/github/license/qixiaoyu27/Rain-VibeType?style=flat-square&color=0e7490)](LICENSE)

[跨平台发布页](https://github.com/qixiaoyu27/Rain-VibeType/releases/tag/v1.1.0-mac.1) · [本地速度](#本地识别有多快) · [功能](#核心能力) · [模型](#支持的模型) · [隐私](#隐私与安全边界)

</div>

---

## 为什么是 Rain

| 你在意的事 | Rain 的做法 |
| --- | --- |
| 隐私 | 音频和识别文本只在内存与本地模型中处理，不上传、不保存历史。 |
| 速度 | 默认使用原生 Rust / sherpa-onnx Worker；模型加载后无需等待云端请求。 |
| 不打断工作 | 录音前记住目标应用，完成后仅在目标仍安全时写入，否则保留到剪贴板。 |
| 剪贴板安全 | 粘贴前完整快照；用户期间复制了新内容时，Rain 不会覆盖。 |
| 轻量安装 | 基础应用不捆绑模型和大型运行时，只下载你明确选择的组件。 |
| 灵活选择 | 支持多种本地识别模型、实时预览和可选的本地文本整理。 |

## 本地识别有多快

Rain 的默认 SenseVoice 路径由原生 Rust / sherpa-onnx Worker 执行。模型加载后，识别不经过云端请求，也不受网络延迟影响。

### Apple Silicon 实测

| 测试环境 | 模型与输入 | 结果 |
| --- | --- | --- |
| MacBook Pro · Apple M1 Pro · 16 GB | v1.1.0 macOS Preview 1 发布的 SenseVoice Small ONNX；5.616 秒中文音频；连续 10 次热模型推理 | 中位数 **107.5 ms**，范围 **107–110 ms**，约 **52.2× 实时速度** |

本次单独启动 Worker 后的模型加载耗时为 **790 ms**，10 次识别文本完全一致。以上数字只统计模型推理，不包含录音时长、首次下载、应用启动和最终写入；不同芯片、模型与音频内容会有差异。

速度来自四个简单边界：

- 默认模型使用原生 arm64 Worker，不需要把音频发送到远端服务。
- Worker 独立运行并复用已加载模型，连续输入不必反复初始化。
- 录音数据通过本地二进制 IPC 传递，避免 Base64 和网络往返。
- 实时预览使用独立的轻量流式模型，不阻塞最终识别模型。

Fun-ASR Nano 和 Paraformer 更偏向模型能力与兼容性，速度和内存占用不能直接套用上述 SenseVoice 数据。

## 核心能力

| 模块 | Rain 的行为 |
| --- | --- |
| 本地识别 | 默认使用 Rust / sherpa-onnx 的 SenseVoice Worker，也可选择 Fun-ASR Nano 或 Paraformer-zh。 |
| 两种录音模式 | 支持按住说话和按键开关；`Esc` 或悬浮取消按钮可随时中止。 |
| 实时文字预览 | 可选中英双语流式预览；预览只负责即时反馈，最终结果由主模型独立生成。 |
| 安全写入 | 录音前记住目标应用，写入前再次复核；目标变化时不误输，结果保留到剪贴板。 |
| 剪贴板保护 | 粘贴前完整快照；如果用户期间复制了新内容，Rain 不会覆盖。 |
| 本地文本整理 | 可选 Qwen3 0.6B + llama.cpp 调整标点、分段和轻量表达；校验失败时保留原文。 |
| 桌面体验 | 全局快捷键、托盘常驻、开机启动、模型按需加载、中文/English 和诊断包。 |

```text
全局快捷键 → 内存录音 → 本地识别 → 可选文本整理 → 目标复核 → 安全写入
```

## 支持的模型

| 模型 | 用途 | 特点 |
| --- | --- | --- |
| SenseVoice Small | 默认语音识别 | 中文、English、粤语、日本語、한국어；原生 sherpa-onnx Worker。 |
| Fun-ASR Nano | 多语种语音识别 | 中文、English、日本語和中文方言；能力更完整，占用也更高。 |
| Paraformer-zh | 中英语音识别 | 偏中文场景，通过 FunASR Worker 运行。 |
| Streaming Zipformer | 实时文字预览 | 轻量中英双语流式模型，不替代最终识别。 |
| Qwen3 0.6B | 可选文本整理 | 通过 llama.cpp 在本地处理标点、分段和轻量表达。 |

基础应用不捆绑模型、PyTorch、CUDA 或 llama.cpp。只有在你明确点击下载后，Rain 才会获取当前平台需要的组件，并在安装前校验文件大小与 SHA-256。

## 获取版本

Windows 与 macOS 安装包、SHA-256、版本说明和已知限制统一放在 [跨平台发布页](https://github.com/qixiaoyu27/Rain-VibeType/releases/tag/v1.1.0-mac.1)，历史版本仍可在 [Releases](https://github.com/qixiaoyu27/Rain-VibeType/releases) 查看。

| 平台 | 系统要求 | 默认快捷键 | 发布状态 |
| --- | --- | --- | --- |
| macOS | macOS 12+、Apple Silicon M1 或更新芯片 | `⌘ + ⇧ + Space` | [v1.1.0 Preview 1](https://github.com/qixiaoyu27/Rain-VibeType/releases/tag/v1.1.0-mac.1) |
| Windows | Windows 11 x64 | `Ctrl + Shift + Space` | [v1.0.0 测试版](https://github.com/qixiaoyu27/Rain-VibeType/releases/tag/v1.1.0-mac.1) |

### macOS 首次启动

> [!IMPORTANT]
> macOS Preview 使用临时签名，尚未经过 Apple Developer ID 签名和公证。请先确认应用来自本仓库 [Releases](https://github.com/qixiaoyu27/Rain-VibeType/releases) 并核对 SHA-256；安装到“应用程序”后，在终端执行：
>
> ```bash
> sudo xattr -dr com.apple.quarantine "/Applications/雨音输入法.app"
> open "/Applications/雨音输入法.app"
> ```
>
> 这两条命令只解除“雨音输入法”自身的隔离标记，不会关闭系统的全局 Gatekeeper。当前 macOS 版本仅支持 Apple Silicon，不支持 Intel Mac。

## 隐私与安全边界

- 录音和识别文本只在内存与本地模型中处理，不上传、不保存历史。
- 模型和推理组件仅在用户明确发起下载时联网。
- 录音完成后会再次核对目标应用，避免把文字写进错误窗口。
- 剪贴板仅在 Rain 写入的内容仍未被用户替换时恢复。
- 文本整理失败、超时或改变事实时，保留原始识别结果。
- 诊断包由用户主动导出，不作为默认数据采集渠道。

## License

Rain Vibetype 采用 [GNU Affero General Public License v3.0](LICENSE)（`AGPL-3.0-only`）。
