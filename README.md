# Rain氛围输入法 / Rain-Vibetype（Windows 11 x64）

Rain氛围输入法（Rain-Vibetype）是一款免费、本地优先的 Windows 11 语音输入工具，官方仓库为 [qixiaoyu27/Rain-VibeType](https://github.com/qixiaoyu27/Rain-VibeType)。按下全局快捷键录音，语音由本机模型识别；默认通过完整剪贴板快照安全粘贴到录音开始时的原输入位置，目标变化时只复制结果，不抢回焦点。

软件默认使用简体中文。设置中仍可按设计文档切换为“跟随系统”或 English。

## 已实现功能

- Tauri 2 + Rust 桌面核心、系统托盘、无焦点底部浮条和独立取消按钮。
- 按住说话与开关录音两种模式；默认快捷键 `Ctrl + Shift + Space`，支持录制新快捷键和注册失败回滚。
- `Esc`、浮条取消按钮可在录音、等待模型和识别阶段取消；迟到结果不会提交。
- 麦克风音频只在内存中处理，统一为单声道 16 kHz PCM 16-bit，不使用 VAD，不保存录音或识别历史。
- 录音开始时保存窗口、焦点控件和进程身份，识别完成后重新验证。
- 默认安全粘贴会保存所有剪贴板格式、写入识别结果、粘贴，并仅在用户没有复制新内容时恢复原剪贴板；也可选择 Unicode 模拟输入。
- 长期存活的 Python Worker、JSON 控制帧 + 二进制 PCM、请求 ID、超时、崩溃熔断和 Windows Job Object 生命周期约束。
- SenseVoice Small、Fun-ASR Nano、Paraformer-zh 三个统一适配器；自动检测 NVIDIA 显卡并选择独立 GPU/CPU 推理组件，支持经确认的 GPU→CPU 回退。
- 基础安装包不内置 PyTorch 或 CUDA。首次配置由用户明确点击后下载匹配组件，支持断点续传、SHA-256 校验和原子安装。
- 版本化模型清单；应用内下载、暂停、断点续传、逐文件 SHA-256 校验、原子安装、目录/ZIP 导入、校验和安全删除。
- 独立 HTTPS 模型清单更新；不会自动替换模型，新版本验证成功后才切换，并由用户决定是否删除旧版本。
- 首次启动推荐 SenseVoice Small，但不会静默下载。用户点击“下载并自动配置”后，下载、校验、选择模型均由应用完成。
- 常驻、随录音加载、立即卸载、空闲卸载和退出时卸载策略。
- 默认开机启动、可选自动检查签名更新、一次性诊断包和默认关闭的匿名崩溃报告。
- 简体中文默认界面，并提供系统语言和 English 选项。

## 开发运行

要求：Windows 11 x64、Rust MSVC 工具链、Node.js、Python 3.11 和 WebView2。

```powershell
.\scripts\setup-worker.ps1
npm install
npm run dev
```

开发运行不会内置模型。首次启动后可以跳过模型下载，也可以在引导页一键下载默认模型，或在“模型管理”中安装其他模型、导入模型目录/ZIP。

## 验证

```powershell
cargo test --all-targets --manifest-path .\src-tauri\Cargo.toml
python -m unittest worker.test_worker -v
node --check .\src\main.js
node --check .\src\overlay.js
node --check .\src\cancel.js
```

自动测试不下载真实模型。真实模型推理、各 Windows 应用输入兼容性、剪贴板格式矩阵和干净机器安装需要按 [Windows 验收清单](docs/WINDOWS_ACCEPTANCE.md) 执行。

## 发布 Windows 安装包

发布流程分别生成轻量 Tauri NSIS 基础安装包、PyInstaller CPU 推理组件和 NVIDIA CUDA 推理组件。PyTorch、CUDA 运行库与模型权重都不会进入基础安装包。

程序内置的默认发布地址为 `https://github.com/qixiaoyu27/Rain-VibeType/releases/latest/download`，会使用其中的 `latest.json`、`models.json`、`runtime-manifest.json` 和推理组件 ZIP。以下同名环境变量可覆盖默认地址：

- `RAIN_UPDATE_ENDPOINT`：GitHub Releases 更新清单 HTTPS 地址。
- `RAIN_UPDATE_PUBLIC_KEY`：Tauri 更新签名公钥。
- `RAIN_MODEL_MANIFEST_ENDPOINT`：与应用更新分离的模型清单 HTTPS 地址。
- `RAIN_RUNTIME_MANIFEST_ENDPOINT`：CPU/NVIDIA 推理组件清单 HTTPS 地址。
- `RAIN_RUNTIME_ARTIFACT_BASE_URL`：推理组件 ZIP 的 HTTPS 发布目录；默认使用官方仓库 Releases。
- `TAURI_SIGNING_PRIVATE_KEY`：Tauri 更新签名私钥。
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`：私钥有密码时设置。
- `RAIN_CRASH_REPORT_ENDPOINT`：可选的匿名崩溃报告 HTTPS 地址。

```powershell
.\scripts\release.ps1
```

CPU/NVIDIA 组件 ZIP 与含真实大小、SHA-256 的清单输出到 `artifacts\runtimes`，发布这些文件后，应用会根据“自动/CPU/CUDA”设置下载对应版本。NSIS 安装包及带签名的更新产物输出到 `src-tauri\target\release\bundle\nsis`。没有真实发布地址和签名密钥时，开发版仍可通过绝对 Python 路径回退，但不能当作正式发布包。

## 隐私边界

- 音频、识别文本、剪贴板内容和窗口标题不写入日志或诊断包。
- 默认无遥测；模型下载只由用户明确触发。
- 更新检查与模型下载是独立流程。
- 删除模型只允许作用于应用管理的模型根目录。

三个模型的来源、版本、文件大小和哈希位于 `src-tauri/resources/models.json`；发布前应重新核对官方来源是否仍与清单一致。

## 许可证

本项目采用 GNU Affero General Public License v3.0（`AGPL-3.0-only`），完整条款见仓库根目录 `LICENSE`。
