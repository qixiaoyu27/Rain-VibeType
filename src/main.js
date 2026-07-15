const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const dialog = window.__TAURI__.dialog;
const currentWindow = window.__TAURI__.window.getCurrentWindow();

const byId = (id) => document.getElementById(id);
const dictionaries = {
  "zh-CN": {
    brand_subtitle: "氛围输入法 · RAIN-VIBETYPE", home_eyebrow: "本地语音，直接输入", system_status_label: "系统状态", metric_model: "模型", metric_input: "输入", metric_privacy: "隐私", section_capture: "录音采集", section_models: "本地模型", section_system: "系统", section_output: "文字投递", section_safety: "安全保护", section_feedback: "反馈", section_privacy: "隐私", onboarding_welcome: "欢迎使用 Rain氛围输入法",
    nav_home: "概览", nav_voice: "录音与快捷键", nav_models: "模型管理", nav_general: "通用设置", nav_output: "文字输入", nav_feedback: "反馈提示", nav_privacy: "隐私与诊断",
    local_only: "音频与识别文字仅在本机内存中处理", home_title: "开口即写，内容不离开电脑。", home_lede: "按住快捷键说话，松开后 Rain 在本机识别，并安全写回原来的输入位置。", current_hotkey: "当前快捷键", home_model_hint: "模型只从应用管理的本地目录加载。", home_input_hint: "注入前会再次确认输入位置，默认恢复原剪贴板。", no_history: "不保留历史", no_history_hint: "不保存录音，不建立识别文字历史。", recovery_label: "待恢复文字", recovery_title: "目标位置和剪贴板均不可用，请手动复制", copy: "复制",
    voice_title: "录音与快捷键", save: "保存设置", recording_mode: "录音方式", push_to_talk: "按住说话，松开识别", toggle_mode: "按一次开始，再按一次结束", hotkey: "全局快捷键", hotkey_hint: "点击后按下新的组合键；注册失败会自动保留旧快捷键。", duration: "最长录音时长（秒）", input_device: "输入设备", duck_system_audio: "录音时降低电脑声音", duck_system_audio_hint: "开启后，录音期间系统播放音量会降至原来的 20%，结束后自动恢复。", mic_test: "麦克风测试", test_500ms: "测试 0.5 秒",
    models_title: "模型管理", refresh: "刷新", check_model_updates: "检查模型更新", model_catalog_latest: "模型清单已经是最新版本", model_catalog_updated: "模型清单已更新，请确认后再下载新版本", models_note: "下载支持断点续传、SHA-256 校验与原子安装；也可导入模型目录或 ZIP。", runtime_component: "本地推理组件", download_runtime: "下载推荐组件", refresh_runtime: "刷新组件清单", runtime_detecting: "正在检测硬件…", runtime_installed: "已安装", runtime_development: "开发环境", runtime_missing: "未安装", runtime_downloading: "正在下载推理组件", runtime_nvidia_recommended: "检测到 NVIDIA 显卡，推荐安装 GPU 加速版", runtime_cpu_recommended: "未检测到 NVIDIA 显卡，推荐安装 CPU 版", runtime_python_fallback: "当前使用开发环境 Python；可安装推荐组件获得独立运行环境", runtime_managed_ready: "推理组件已校验并安装，语音识别只在本机进行", runtime_manifest_unavailable: "官方组件尚未发布；当前可继续使用开发环境，发布 GitHub Release 后会自动生效", storage_dir: "模型存储目录", choose: "选择", reset: "恢复默认", storage_hint: "更改后只影响后续下载；已安装模型不会自动移动。", device: "推理设备", auto_device: "自动选择", cpu_hint: "CPU 模式功能完整，但大型模型的识别速度可能明显较慢。", load_mode: "模型加载方式", on_demand: "录音时并行加载", resident: "常驻内存", unload_policy: "模型卸载策略", unload_now: "识别后立即卸载", unload_idle: "空闲后卸载", unload_session: "退出时卸载", idle_timeout: "空闲卸载（秒）", python_path: "开发者 Python Worker 路径", python_hint: "仅供源码调试回退使用；正式安装版会优先使用上方已校验的推理组件。", save_model_settings: "保存模型设置", check_worker: "检查 Worker",
    general_title: "通用设置", autostart: "登录 Windows 时启动", autostart_hint: "默认开启，应用启动后驻留系统托盘。", auto_update: "启动时检查更新", auto_update_hint: "只检查签名更新，不会自动安装。", language: "界面语言", follow_system: "跟随系统", updates: "软件更新", check_update: "检查更新", install_update: "安装并重启", theme_light: "浅色模式", theme_dark: "深色模式", switch_theme_light: "切换到浅色模式", switch_theme_dark: "切换到深色模式",
    output_title: "文字输入", injection: "输入方式", clipboard_mode: "剪贴板粘贴（推荐）", typing_mode: "模拟逐字输入", injection_hint: "粘贴兼容性更好；逐字输入不会使用剪贴板，但速度较慢。", restore_clipboard: "粘贴后恢复剪贴板", restore_hint: "若期间检测到用户复制了新内容，Rain 不会覆盖它。", target_validation: "目标窗口二次校验", target_hint: "录音开始时记录输入位置；识别结束后若位置已变化，结果只复制到剪贴板，不会误写到其他窗口。",
    feedback_title: "反馈提示", overlay: "显示状态悬浮条", overlay_hint: "显示录音音量、时长、模型加载、识别和错误状态。", overlay_fullscreen: "全屏应用中也显示", overlay_fullscreen_hint: "关闭后，全屏游戏或演示时只使用声音反馈。", start_sound: "开始录音提示音", stop_sound: "结束录音提示音", error_sound: "错误提示音",
    privacy_title: "隐私与诊断", privacy_banner: "默认不上传，不留录音，不留文字历史。", privacy_detail: "诊断事件只记录错误代码、模型 ID 与耗时。导出前会列出文件并由你选择保存位置。", crash_reports: "匿名崩溃报告", crash_hint: "默认关闭。当前构建只生成本地崩溃标记，不包含音频或识别文字。", diagnostics: "诊断工具", export_diagnostics: "导出诊断包", open_logs: "打开日志目录",
    onboarding_title: "自动配置本地语音识别", onboarding_text: "Rain 会先检测显卡并下载匹配的 CPU 或 NVIDIA 推理组件，再下载 SenseVoice Small 模型；全部校验通过后即可使用。", download_default: "下载并自动配置", skip: "暂时跳过", onboarding_autostart: "Rain 默认随 Windows 登录启动并驻留托盘，可随时在通用设置中关闭。", onboarding_privacy: "模型与推理组件从发布清单指定的 HTTPS 地址下载；语音识别始终在本机进行。", hotkey_conflict: "默认快捷键不可用", record_new_hotkey: "录制新快捷键",
    ready: "可以开始使用", ready_detail: "按下快捷键即可开始说话", needs_model: "尚未配置模型", needs_model_detail: "下载默认模型，或在模型管理页导入已有模型", not_selected: "未选择", secure_paste: "安全粘贴", typing: "逐字输入", system_default: "跟随系统默认设备", saved: "设置已保存", save_failed: "保存失败", copied: "识别结果已复制", checking: "正在检查…", mic_ok: "输入电平", mic_failed: "麦克风测试失败", no_models: "无法读取模型清单", current: "当前使用", current_update: "当前使用 · 可更新", installed: "已安装", custom: "本地自定义", not_installed: "未安装", model_update_available: "可更新", use_model: "使用此模型", download: "下载", download_update: "下载新版本", pause: "暂停", resume: "继续下载", verify: "校验", delete: "删除", import_dir: "导入目录", import_zip: "导入 ZIP", official_source: "官方源", size: "下载 / 安装", hardware: "建议硬件", speed: "速度", languages: "语言", license: "许可证", adapter: "适配器", model_id: "模型 ID", downloading: "正在下载", download_paused: "下载已暂停，可继续", verified: "模型结构或完整性校验通过", confirm_delete: "确定删除该模型及未完成下载吗？", confirm_delete_old: "新版本已经验证并切换成功。是否删除旧版本：", import_done: "模型已导入并设为当前模型", check_failed: "检查失败", export_confirm: "诊断包将包含以下文件，不包含音频或识别文字：", exported: "诊断包已导出", no_path: "已取消选择", update_latest: "当前已经是最新版本", update_available: "发现新版本", update_installing: "正在下载签名更新并安装…", update_not_configured: "此构建尚未配置更新签名公钥", all_feedback_confirm: "你正在关闭所有视觉和声音反馈。录音可能在没有明显提示的情况下进行，确定继续吗？", hotkey_listening: "请按新的组合键…", invalid_hotkey: "快捷键必须包含非修饰键，且不能是 Escape", model_busy: "另一个模型正在下载", model_load_failed: "模型加载失败", cpu_fallback: "GPU 模型加载失败。是否将推理设备改为 CPU 后重试？", crash_summary: "Rain 上次异常退出。以下是将提交的匿名数据摘要，确定发送吗？", crash_sent: "匿名崩溃报告已提交"
  },
  en: {
    brand_subtitle: "VIBE INPUT · WINDOWS 11", home_eyebrow: "LOCAL VOICE, DIRECT INPUT", system_status_label: "SYSTEM STATUS", metric_model: "MODEL", metric_input: "INPUT", metric_privacy: "PRIVACY", section_capture: "CAPTURE", section_models: "LOCAL MODELS", section_system: "SYSTEM", section_output: "TEXT DELIVERY", section_safety: "SAFETY", section_feedback: "FEEDBACK", section_privacy: "PRIVACY", onboarding_welcome: "WELCOME TO RAIN-VIBETYPE",
    nav_home: "Overview", nav_voice: "Recording & hotkey", nav_models: "Models", nav_general: "General", nav_output: "Text input", nav_feedback: "Feedback", nav_privacy: "Privacy & diagnostics",
    local_only: "Audio and recognized text are processed only in local memory", home_title: "Speak to write. Keep every word on your PC.", home_lede: "Hold the hotkey to speak. Rain recognizes locally and safely returns text to the original input.", current_hotkey: "Current hotkey", home_model_hint: "Models load only from app-managed local folders.", home_input_hint: "Rain revalidates the target and restores the clipboard by default.", no_history: "No history", no_history_hint: "Audio is not saved and recognized text history is never created.", recovery_label: "Text to recover", recovery_title: "The target and clipboard were unavailable. Copy this text manually.", copy: "Copy",
    voice_title: "Recording & hotkey", save: "Save settings", recording_mode: "Recording mode", push_to_talk: "Hold to speak, release to transcribe", toggle_mode: "Press once to start and again to stop", hotkey: "Global hotkey", hotkey_hint: "Click and press a new combination. A failed registration keeps the previous hotkey.", duration: "Maximum recording length (seconds)", input_device: "Input device", duck_system_audio: "Lower PC audio while recording", duck_system_audio_hint: "Reduces system playback to 20% of its previous volume while recording, then restores it.", mic_test: "Microphone test", test_500ms: "Test for 0.5 seconds",
    models_title: "Model management", refresh: "Refresh", check_model_updates: "Check model updates", model_catalog_latest: "The model catalog is up to date", model_catalog_updated: "The model catalog was updated; confirm before downloading a new model version", models_note: "Downloads support resume, SHA-256 verification and atomic install. Model folders and ZIP files can also be imported.", runtime_component: "Local inference component", download_runtime: "Download recommended component", refresh_runtime: "Refresh component catalog", runtime_detecting: "Detecting hardware…", runtime_installed: "Installed", runtime_development: "Development", runtime_missing: "Not installed", runtime_downloading: "Downloading inference component", runtime_nvidia_recommended: "NVIDIA GPU detected; the accelerated component is recommended", runtime_cpu_recommended: "No NVIDIA GPU detected; the CPU component is recommended", runtime_python_fallback: "Using the development Python environment; install the recommended standalone component if desired", runtime_managed_ready: "The verified inference component is installed and recognition stays local", runtime_manifest_unavailable: "Official components have not been published yet; they will become available after the GitHub Release is uploaded", storage_dir: "Model storage folder", choose: "Choose", reset: "Use default", storage_hint: "Changes affect future downloads only; installed models are not moved.", device: "Inference device", auto_device: "Auto", cpu_hint: "CPU mode is fully supported, but recognition may be much slower with large models.", load_mode: "Model loading", on_demand: "Load alongside recording", resident: "Keep in memory", unload_policy: "Unload policy", unload_now: "Immediately after recognition", unload_idle: "After idle timeout", unload_session: "When Rain exits", idle_timeout: "Idle timeout (seconds)", python_path: "Developer Python Worker path", python_hint: "Source builds may fall back to Python. Installed builds prefer the verified component above.", save_model_settings: "Save model settings", check_worker: "Check Worker",
    general_title: "General settings", autostart: "Start when signing in to Windows", autostart_hint: "Enabled by default. Rain stays in the system tray.", auto_update: "Check for updates at startup", auto_update_hint: "Checks signed updates only and never installs automatically.", language: "Interface language", follow_system: "Follow system", updates: "Software update", check_update: "Check for updates", install_update: "Install and restart", theme_light: "Light", theme_dark: "Dark", switch_theme_light: "Switch to light mode", switch_theme_dark: "Switch to dark mode",
    output_title: "Text input", injection: "Input method", clipboard_mode: "Clipboard paste (recommended)", typing_mode: "Simulated typing", injection_hint: "Paste is more compatible. Simulated typing avoids the clipboard but is slower.", restore_clipboard: "Restore clipboard after paste", restore_hint: "Rain will not overwrite new content copied while recognition is running.", target_validation: "Target revalidation", target_hint: "Rain records the input target when recording starts. If it changes, text is copied instead of being written into the wrong window.",
    feedback_title: "Feedback", overlay: "Show status overlay", overlay_hint: "Shows audio level, duration, model loading, recognition and errors.", overlay_fullscreen: "Show over full-screen apps", overlay_fullscreen_hint: "Disable to use audio feedback only in games or presentations.", start_sound: "Recording start sound", stop_sound: "Recording stop sound", error_sound: "Error sound",
    privacy_title: "Privacy & diagnostics", privacy_banner: "No upload by default. No audio. No text history.", privacy_detail: "Diagnostic events contain only error codes, model IDs and timings. Files are listed before export and you choose the destination.", crash_reports: "Anonymous crash reports", crash_hint: "Off by default. This build creates a local crash marker only, without audio or recognized text.", diagnostics: "Diagnostic tools", export_diagnostics: "Export diagnostics", open_logs: "Open log folder",
    onboarding_title: "Set up local speech recognition", onboarding_text: "Rain detects your hardware, downloads the matching CPU or NVIDIA component, then installs SenseVoice Small after verification.", download_default: "Download and configure", skip: "Skip for now", onboarding_autostart: "Rain starts with Windows and stays in the tray by default. You can disable this in General settings.", onboarding_privacy: "Components and models use HTTPS locations from release manifests; recognition always runs locally.", hotkey_conflict: "The default hotkey is unavailable", record_new_hotkey: "Record a new hotkey",
    ready: "Ready", ready_detail: "Press the hotkey and start speaking", needs_model: "No model configured", needs_model_detail: "Download the default model or import one in Model management", not_selected: "Not selected", secure_paste: "Secure paste", typing: "Simulated typing", system_default: "System default device", saved: "Settings saved", save_failed: "Could not save", copied: "Recognition result copied", checking: "Checking…", mic_ok: "Input level", mic_failed: "Microphone test failed", no_models: "Could not read model manifest", current: "In use", current_update: "In use · update available", installed: "Installed", custom: "Local custom", not_installed: "Not installed", model_update_available: "Update available", use_model: "Use this model", download: "Download", download_update: "Download new version", pause: "Pause", resume: "Resume", verify: "Verify", delete: "Delete", import_dir: "Import folder", import_zip: "Import ZIP", official_source: "Official source", size: "Download / installed", hardware: "Recommended", speed: "Speed", languages: "Languages", license: "License", adapter: "Adapter", model_id: "Model ID", downloading: "Downloading", download_paused: "Download paused; it can be resumed", verified: "Model structure or integrity verified", confirm_delete: "Delete this model and any partial download?", confirm_delete_old: "The new version is verified and selected. Delete old versions:", import_done: "Model imported and selected", check_failed: "Check failed", export_confirm: "The diagnostic archive will contain these files and no audio or recognized text:", exported: "Diagnostics exported", no_path: "Selection cancelled", update_latest: "You already have the latest version", update_available: "New version available", update_installing: "Downloading and installing the signed update…", update_not_configured: "This build has no update signing public key", all_feedback_confirm: "You are disabling all visual and audio feedback. Recording may occur without an obvious indicator. Continue?", hotkey_listening: "Press a new key combination…", invalid_hotkey: "The hotkey needs a non-modifier key and cannot be Escape", model_busy: "Another model is downloading", model_load_failed: "Model loading failed", cpu_fallback: "GPU model loading failed. Change the inference device to CPU and retry?", crash_summary: "Rain exited unexpectedly last time. Send this anonymous data summary?", crash_sent: "Anonymous crash report sent"
  }
};

const fields = {
  recordingMode: byId("recording-mode"), hotkey: byId("hotkey-recorder"), maxRecordingSeconds: byId("max-recording-seconds"), inputDevice: byId("input-device"), duckSystemAudio: byId("duck-system-audio"),
  modelStorageDir: byId("model-storage-dir"), pythonPath: byId("python-path"), devicePreference: byId("device-preference"), modelLoadMode: byId("model-load-mode"), unloadPolicy: byId("unload-policy"), idleTimeoutSeconds: byId("idle-timeout-seconds"),
  autostart: byId("autostart"), autoCheckUpdates: byId("auto-check-updates"), uiLanguage: byId("ui-language"),
  injectionMethod: byId("injection-method"), restoreClipboard: byId("restore-clipboard"),
  showOverlay: byId("show-overlay"), showOverlayFullscreen: byId("show-overlay-fullscreen"), startSound: byId("start-sound"), stopSound: byId("stop-sound"), errorSound: byId("error-sound"),
  anonymousCrashReports: byId("anonymous-crash-reports")
};

const state = { config: null, system: null, runtime: null, models: [], language: "zh-CN", hotkey: "Ctrl+Shift+Space", hotkeyListening: false, activeDownload: null, activeRuntimeDownload: false, update: null };

async function invokeWhenReady(command, args) {
  let lastError;
  for (let attempt = 0; attempt < 20; attempt += 1) {
    try { return await invoke(command, args); }
    catch (error) {
      lastError = error;
      if (!String(error).includes("state not managed")) throw error;
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
  }
  throw lastError;
}

function resolvedLanguage(value) {
  if (value === "zh-CN" || value === "en") return value;
  return navigator.language.toLowerCase().startsWith("zh") ? "zh-CN" : "en";
}

function t(key) { return dictionaries[state.language]?.[key] || dictionaries["zh-CN"][key] || key; }

function renderThemeToggle() {
  const nextTheme = document.documentElement.dataset.theme === "light" ? "dark" : "light";
  const label = t(`theme_${nextTheme}`);
  const button = byId("theme-toggle");
  byId("theme-label").textContent = label;
  button.title = t(`switch_theme_${nextTheme}`);
  button.setAttribute("aria-label", button.title);
}

function setTheme(theme) {
  document.documentElement.dataset.theme = theme;
  localStorage.setItem("rain-theme", theme);
  renderThemeToggle();
}

function applyLanguage(configured) {
  state.language = resolvedLanguage(configured);
  document.documentElement.lang = state.language;
  document.querySelectorAll("[data-i18n]").forEach((node) => { node.textContent = t(node.dataset.i18n); });
  if (state.config) updateHome();
  renderRuntime();
  renderModels();
  renderThemeToggle();
}

function toast(message) {
  const node = byId("toast");
  node.textContent = message;
  node.classList.add("show");
  clearTimeout(toast.timer);
  toast.timer = setTimeout(() => node.classList.remove("show"), 2600);
}

function showSection(id) {
  const target = id === "model" ? "models" : id;
  document.querySelectorAll(".page").forEach((page) => page.classList.toggle("active", page.id === target));
  document.querySelectorAll(".nav-item").forEach((item) => item.classList.toggle("active", item.dataset.section === target));
}

function prettyHotkey(value) { return value.replace(/\+/g, " + "); }
function formatBytes(bytes) { return `${(bytes / 1073741824).toFixed(bytes >= 1073741824 ? 1 : 2)} GB`; }

function fillConfig(config) {
  state.config = config;
  state.hotkey = config.hotkey;
  fields.recordingMode.value = config.recording_mode;
  fields.hotkey.textContent = prettyHotkey(config.hotkey);
  fields.maxRecordingSeconds.value = String(config.max_recording_seconds);
  fields.inputDevice.value = config.input_device || "";
  fields.duckSystemAudio.checked = config.duck_system_audio;
  fields.modelStorageDir.value = config.model_storage_dir || "";
  fields.pythonPath.value = config.python_path || "python";
  fields.devicePreference.value = config.device_preference;
  fields.modelLoadMode.value = config.model_load_mode;
  fields.unloadPolicy.value = config.unload_policy;
  fields.idleTimeoutSeconds.value = String(config.idle_timeout_seconds);
  fields.autostart.checked = config.autostart;
  fields.autoCheckUpdates.checked = config.auto_check_updates;
  fields.uiLanguage.value = config.ui_language;
  fields.injectionMethod.value = config.injection_method;
  fields.restoreClipboard.checked = config.restore_clipboard;
  fields.showOverlay.checked = config.show_overlay;
  fields.showOverlayFullscreen.checked = config.show_overlay_fullscreen;
  fields.startSound.checked = config.start_sound;
  fields.stopSound.checked = config.stop_sound;
  fields.errorSound.checked = config.error_sound;
  fields.anonymousCrashReports.checked = config.anonymous_crash_reports;
  applyLanguage(config.ui_language);
  updateHome();
}

function fillSystemStatus(status) {
  state.system = status;
  const warning = byId("onboarding-system-error");
  warning.hidden = status.shortcut_ready;
  byId("onboarding-system-error-detail").textContent = status.shortcut_error || "";
}

function renderRuntime() {
  const status = state.runtime;
  if (!status || !byId("runtime-card")) return;
  const recommended = status.components.find((component) => component.id === status.recommended_component_id);
  const managedReady = status.source === "managed";
  const installedRecommended = managedReady && status.active_component_id === status.recommended_component_id;
  const gpu = status.nvidia_detected ? (status.nvidia_name || "NVIDIA GPU") : null;
  byId("runtime-hardware").textContent = gpu
    ? `${gpu} · ${t("runtime_nvidia_recommended")}${recommended ? ` · ${formatBytes(recommended.archive_size)}` : ""}`
    : `${t("runtime_cpu_recommended")}${recommended ? ` · ${formatBytes(recommended.archive_size)}` : ""}`;
  byId("runtime-state").textContent = managedReady ? t("runtime_installed") : status.source === "python" ? t("runtime_development") : t("runtime_missing");
  byId("runtime-state").classList.toggle("ready", status.ready);
  byId("runtime-description").textContent = managedReady
    ? t("runtime_managed_ready")
    : status.source === "python"
      ? recommended ? t("runtime_python_fallback") : t("runtime_manifest_unavailable")
      : recommended?.display_name || t("runtime_manifest_unavailable");
  const button = byId("download-runtime");
  button.disabled = state.activeRuntimeDownload || installedRecommended || !recommended;
  button.textContent = state.activeRuntimeDownload
    ? t("runtime_downloading")
    : installedRecommended
      ? t("runtime_installed")
      : recommended
        ? `${t("download_runtime")} · ${recommended.display_name}`
        : t("download_runtime");
  byId("refresh-runtime").disabled = state.activeRuntimeDownload;
  byId("runtime-download-progress").hidden = !state.activeRuntimeDownload;
}

async function loadRuntime({ refresh = false } = {}) {
  try {
    state.runtime = await invoke(refresh ? "refresh_runtime_status" : "get_runtime_status");
    if (!refresh && !state.runtime.components.length) {
      try { state.runtime = await invoke("refresh_runtime_status"); } catch {}
    }
    renderRuntime();
    updateHome();
    return state.runtime;
  } catch (error) {
    if (byId("runtime-description")) byId("runtime-description").textContent = String(error);
    return null;
  }
}

async function downloadRuntime({ onboarding = false, force = false } = {}) {
  if (state.activeRuntimeDownload) return false;
  if (!force && state.runtime?.ready) return true;
  if (fields.devicePreference.value !== state.config.device_preference && !await saveConfig({ quiet: true })) return false;
  state.activeRuntimeDownload = true;
  renderRuntime();
  if (onboarding) byId("onboarding-progress").hidden = false;
  try {
    state.runtime = await invoke("download_runtime", { componentId: state.runtime?.recommended_component_id || null });
    return true;
  } catch (error) {
    toast(String(error));
    return false;
  } finally {
    state.activeRuntimeDownload = false;
    renderRuntime();
  }
}

function collectConfig() {
  const anyFeedback = fields.showOverlay.checked || fields.startSound.checked || fields.stopSound.checked || fields.errorSound.checked;
  return {
    ...state.config,
    schema_version: 1,
    recording_mode: fields.recordingMode.value,
    hotkey: state.hotkey,
    max_recording_seconds: Number(fields.maxRecordingSeconds.value),
    input_device: fields.inputDevice.value || null,
    duck_system_audio: fields.duckSystemAudio.checked,
    model_storage_dir: fields.modelStorageDir.value.trim() || null,
    python_path: fields.pythonPath.value.trim() || "python",
    device_preference: fields.devicePreference.value,
    model_load_mode: fields.modelLoadMode.value,
    unload_policy: fields.unloadPolicy.value,
    idle_timeout_seconds: Number(fields.idleTimeoutSeconds.value),
    autostart: fields.autostart.checked,
    auto_check_updates: fields.autoCheckUpdates.checked,
    ui_language: fields.uiLanguage.value,
    injection_method: fields.injectionMethod.value,
    restore_clipboard: fields.restoreClipboard.checked,
    show_overlay: fields.showOverlay.checked,
    show_overlay_fullscreen: fields.showOverlayFullscreen.checked,
    start_sound: fields.startSound.checked,
    stop_sound: fields.stopSound.checked,
    error_sound: fields.errorSound.checked,
    feedback_disabled_confirmed: anyFeedback ? false : Boolean(state.config.feedback_disabled_confirmed),
    anonymous_crash_reports: fields.anonymousCrashReports.checked
  };
}

async function saveConfig({ quiet = false } = {}) {
  const config = collectConfig();
  const runtimeChanged = config.device_preference !== state.config.device_preference || config.python_path !== state.config.python_path;
  const allFeedbackOff = !config.show_overlay && !config.start_sound && !config.stop_sound && !config.error_sound;
  if (allFeedbackOff && !config.feedback_disabled_confirmed) {
    if (!window.confirm(t("all_feedback_confirm"))) return null;
    config.feedback_disabled_confirmed = true;
  }
  try {
    let saved = await invoke("save_config", { config });
    fillConfig(saved);
    if (runtimeChanged) await loadRuntime();
    fillSystemStatus(await invoke("get_system_status"));
    if (state.system.shortcut_ready && state.runtime?.ready && saved.model_path && !saved.onboarding_completed) {
      saved.onboarding_completed = true;
      saved = await invoke("save_config", { config: saved });
      fillConfig(saved);
      byId("onboarding").hidden = true;
    }
    if (!quiet) toast(t("saved"));
    return saved;
  } catch (error) {
    toast(`${t("save_failed")}: ${error}`);
    return null;
  }
}

function updateHome() {
  if (!state.config) return;
  const model = state.models.find((item) => item.id === state.config.selected_model_id);
  const modelReady = Boolean(state.config.model_path && ["installed", "custom", "update_available"].includes(model?.state));
  const ready = modelReady && Boolean(state.runtime?.ready);
  byId("status-orb").classList.toggle("error", !ready);
  byId("runtime-status").textContent = ready ? t("ready") : !state.runtime?.ready ? t("runtime_missing") : t("needs_model");
  byId("runtime-detail").textContent = ready ? t("ready_detail") : !state.runtime?.ready ? t("download_runtime") : t("needs_model_detail");
  byId("home-model").textContent = model?.display_name || t("not_selected");
  byId("home-input").textContent = state.config.injection_method === "typing" ? t("typing") : t("secure_paste");
  byId("home-hotkey").textContent = prettyHotkey(state.config.hotkey);
}

function showPendingText(text) {
  byId("recovery").hidden = !text;
  byId("recovery-text").value = text || "";
  if (text) showSection("home");
}

async function loadModels() {
  try {
    state.models = await invoke("list_models");
    renderModels();
    updateHome();
  } catch (error) {
    byId("model-list").innerHTML = `<div class="empty-state">${escapeHtml(t("no_models"))}<small>${escapeHtml(String(error))}</small></div>`;
  }
}

async function checkModelUpdates() {
  const button = byId("check-model-updates");
  button.disabled = true;
  try {
    const result = await invoke("check_model_updates");
    state.models = result.models;
    renderModels();
    updateHome();
    toast(result.changed ? t("model_catalog_updated") : t("model_catalog_latest"));
  } catch (error) {
    toast(String(error));
  } finally {
    button.disabled = false;
  }
}

function escapeHtml(value) {
  return String(value).replace(/[&<>'"]/g, (character) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", "'": "&#39;", '"': "&quot;" }[character]));
}

function modelProgress(modelId) {
  const node = document.querySelector(`[data-progress="${CSS.escape(modelId)}"]`);
  if (!node || state.activeDownload?.modelId !== modelId) return;
  const percent = Math.max(0, Math.min(100, state.activeDownload.percent || 0));
  node.hidden = false;
  node.querySelector("i").style.width = `${percent}%`;
  node.querySelector("span").textContent = `${percent.toFixed(1)}% · ${state.activeDownload.file || ""}`;
}

function renderModels() {
  if (!byId("model-list") || !state.models.length) return;
  byId("model-list").innerHTML = state.models.map((model, index) => {
    const hasUpdate = model.state === "update_available";
    const installed = model.state === "installed" || model.state === "custom" || hasUpdate;
    const custom = model.state === "custom";
    const selected = state.config?.selected_model_id === model.id && installed;
    const active = state.activeDownload?.modelId === model.id;
    const downloadLabel = state.activeDownload?.paused && active ? t("resume") : hasUpdate ? t("download_update") : t("download");
    return `<article class="model-card ${selected ? "selected" : ""}" style="--delay:${index * 55}ms">
      <div class="model-heading"><div class="model-mark">${escapeHtml(model.display_name.slice(0, 2).toUpperCase())}</div><div><h2>${escapeHtml(model.display_name)}</h2><p>${escapeHtml(model.engine)} · ${escapeHtml(model.model_version)}</p></div><span class="state-pill ${installed ? "ready" : ""}">${selected ? (hasUpdate ? t("current_update") : t("current")) : custom ? t("custom") : hasUpdate ? t("model_update_available") : installed ? t("installed") : t("not_installed")}</span></div>
      <dl class="model-meta"><div><dt>${t("model_id")}</dt><dd title="${escapeHtml(model.id)}">${escapeHtml(model.id)}</dd></div><div><dt>${t("languages")}</dt><dd>${escapeHtml(model.languages.join(" / "))}</dd></div><div><dt>${t("size")}</dt><dd>${formatBytes(model.download_size)} / ${formatBytes(model.installed_size)}</dd></div><div><dt>${t("hardware")}</dt><dd title="${escapeHtml(model.recommended_hardware)}">${escapeHtml(model.recommended_hardware)}</dd></div><div><dt>${t("speed")}</dt><dd>${escapeHtml(model.speed_grade)}</dd></div><div><dt>${t("license")}</dt><dd>${escapeHtml(model.license)}</dd></div><div><dt>${t("official_source")}</dt><dd title="${escapeHtml(model.official_source)}">${escapeHtml(model.official_source)}</dd></div><div><dt>${t("adapter")}</dt><dd title="${escapeHtml(model.adapter_compatibility)}">${escapeHtml(model.adapter_compatibility)}</dd></div></dl>
      <div class="download-progress" data-progress="${escapeHtml(model.id)}" ${active ? "" : "hidden"}><i></i><span>${t("downloading")}</span></div>
      <div class="model-actions">
        ${installed && !selected ? `<button class="primary" data-model-action="select" data-model-id="${escapeHtml(model.id)}">${t("use_model")}</button>` : ""}
        ${!installed || hasUpdate ? `<button class="primary" data-model-action="download" data-model-id="${escapeHtml(model.id)}">${downloadLabel}</button>` : ""}
        ${active && !state.activeDownload.paused ? `<button class="secondary" data-model-action="pause" data-model-id="${escapeHtml(model.id)}">${t("pause")}</button>` : ""}
        ${installed ? `<button class="secondary" data-model-action="verify" data-model-id="${escapeHtml(model.id)}">${t("verify")}</button>` : `<button class="secondary" data-model-action="import-dir" data-model-id="${escapeHtml(model.id)}">${t("import_dir")}</button><button class="secondary" data-model-action="import-zip" data-model-id="${escapeHtml(model.id)}">${t("import_zip")}</button>`}
        <button class="ghost danger" data-model-action="delete" data-model-id="${escapeHtml(model.id)}">${t("delete")}</button>
      </div>
    </article>`;
  }).join("");
  if (state.activeDownload) modelProgress(state.activeDownload.modelId);
}

async function downloadModel(modelId, onboarding = false) {
  if (state.activeDownload && state.activeDownload.modelId !== modelId) { toast(t("model_busy")); return; }
  const before = state.models.find((model) => model.id === modelId);
  const updatingSelectedModel = before?.state === "update_available" && state.config?.selected_model_id === modelId;
  const previousVersions = before?.previous_versions || [];
  state.activeDownload = { modelId, percent: state.activeDownload?.percent || 0, file: "", paused: false };
  renderModels();
  if (onboarding) byId("onboarding-progress").hidden = false;
  try {
    state.models = await invoke("download_model", { modelId });
    state.activeDownload = null;
    const config = await invoke("get_config");
    fillConfig(config);
    if (updatingSelectedModel && previousVersions.length && window.confirm(`${t("confirm_delete_old")} ${previousVersions.join(", ")}`)) {
      state.models = await invoke("delete_old_model_versions", { modelId });
    }
    renderModels();
    if (onboarding) {
      fillSystemStatus(await invoke("get_system_status"));
      if (state.system.shortcut_ready) {
        state.config.onboarding_completed = true;
        fillConfig(await invoke("save_config", { config: state.config }));
        byId("onboarding").hidden = true;
      } else {
        toast(t("hotkey_conflict"));
      }
    }
  } catch (error) {
    const paused = String(error).includes("DOWNLOAD_PAUSED");
    if (state.activeDownload) state.activeDownload.paused = paused;
    toast(paused ? t("download_paused") : String(error));
    renderModels();
  }
}

async function setupOnboarding() {
  const button = byId("onboarding-download");
  button.disabled = true;
  try {
    if (!await downloadRuntime({ onboarding: true })) return;
    await downloadModel("sensevoice-small", true);
  } finally {
    button.disabled = false;
  }
}

async function importModel(modelId, directory) {
  const sourcePath = await dialog.open({ directory, multiple: false, filters: directory ? undefined : [{ name: "ZIP", extensions: ["zip"] }] });
  if (!sourcePath) return;
  try {
    const result = await invoke("import_model", { modelId, sourcePath });
    fillConfig(await invoke("get_config"));
    await loadModels();
    toast(result.warning ? `${t("import_done")} · ${result.warning}` : t("import_done"));
  } catch (error) { toast(String(error)); }
}

async function handleModelAction(button) {
  const modelId = button.dataset.modelId;
  const action = button.dataset.modelAction;
  button.disabled = true;
  try {
    if (action === "download") return await downloadModel(modelId);
    if (action === "pause") {
      await invoke("pause_model_download", { modelId });
      if (state.activeDownload) state.activeDownload.paused = true;
      return;
    }
    if (action === "select") {
      fillConfig(await invoke("select_model", { modelId }));
      const previousVersions = state.models.find((model) => model.id === modelId)?.previous_versions || [];
      if (previousVersions.length && window.confirm(`${t("confirm_delete_old")} ${previousVersions.join(", ")}`)) {
        state.models = await invoke("delete_old_model_versions", { modelId });
      }
    }
    if (action === "verify") { await invoke("verify_model", { modelId }); toast(t("verified")); }
    if (action === "delete" && window.confirm(t("confirm_delete"))) {
      state.models = await invoke("delete_model", { modelId });
      fillConfig(await invoke("get_config"));
    }
    if (action === "import-dir") return await importModel(modelId, true);
    if (action === "import-zip") return await importModel(modelId, false);
    await loadModels();
  } catch (error) { toast(String(error)); }
  finally { if (button.isConnected) button.disabled = false; }
}

function hotkeyFromEvent(event) {
  const modifierKeys = new Set(["Control", "Shift", "Alt", "Meta"]);
  if (modifierKeys.has(event.key) || event.key === "Escape") return null;
  const parts = [];
  if (event.ctrlKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  if (event.metaKey) parts.push("Super");
  let key = event.code === "Space" ? "Space" : event.key.length === 1 ? event.key.toUpperCase() : event.key;
  key = ({ ArrowUp: "Up", ArrowDown: "Down", ArrowLeft: "Left", ArrowRight: "Right" })[key] || key;
  parts.push(key);
  return parts.join("+");
}

async function checkForUpdate() {
  const output = byId("update-result");
  byId("install-update").hidden = true;
  output.textContent = t("checking");
  try {
    state.update = await invoke("check_update");
    if (state.update.available) {
      output.textContent = `${t("update_available")}: ${state.update.version}${state.update.notes ? ` · ${state.update.notes}` : ""}`;
      byId("install-update").hidden = false;
    } else output.textContent = `${t("update_latest")} (${state.update.current_version})`;
  } catch (error) {
    output.textContent = String(error).includes("UPDATE_NOT_CONFIGURED") ? t("update_not_configured") : `${t("check_failed")}: ${error}`;
  }
}

async function load() {
  try {
    const [config, systemStatus, devices, pending] = await Promise.all([invokeWhenReady("get_config"), invokeWhenReady("get_system_status"), invokeWhenReady("list_input_devices"), invokeWhenReady("get_pending_text")]);
    fields.inputDevice.replaceChildren(new Option(t("system_default"), ""));
    devices.forEach((name) => fields.inputDevice.add(new Option(name, name)));
    fillConfig(config);
    fillSystemStatus(systemStatus);
    fields.inputDevice.options[0].text = t("system_default");
    showPendingText(pending);
    await loadRuntime();
    await loadModels();
    byId("onboarding").hidden = config.onboarding_completed;
    const crashReport = await invoke("pending_crash_report");
    if (crashReport) {
      if (window.confirm(`${t("crash_summary")}\n\n${JSON.stringify(crashReport, null, 2)}`)) {
        try { await invoke("submit_crash_report"); toast(t("crash_sent")); } catch (error) { toast(String(error)); }
      } else {
        await invoke("dismiss_crash_report");
      }
    }
  } catch (error) {
    byId("runtime-status").textContent = t("check_failed");
    byId("runtime-detail").textContent = String(error);
    byId("status-orb").classList.add("error");
  }
}

document.querySelectorAll(".nav-item").forEach((item) => item.addEventListener("click", () => showSection(item.dataset.section)));
document.querySelectorAll(".save-config").forEach((button) => button.addEventListener("click", () => saveConfig()));
byId("theme-toggle").addEventListener("click", () => setTheme(document.documentElement.dataset.theme === "light" ? "dark" : "light"));
byId("minimize").addEventListener("click", () => currentWindow.minimize());
byId("close").addEventListener("click", () => currentWindow.hide());
byId("refresh-models").addEventListener("click", loadModels);
byId("check-model-updates").addEventListener("click", checkModelUpdates);
byId("download-runtime").addEventListener("click", () => downloadRuntime({ force: true }));
byId("refresh-runtime").addEventListener("click", () => loadRuntime({ refresh: true }));
byId("model-list").addEventListener("click", (event) => { const button = event.target.closest("[data-model-action]"); if (button) handleModelAction(button); });
byId("pick-storage-dir").addEventListener("click", async () => { const path = await dialog.open({ directory: true, multiple: false }); if (path) fields.modelStorageDir.value = path; });
byId("reset-storage-dir").addEventListener("click", () => { fields.modelStorageDir.value = ""; });
byId("ui-language").addEventListener("change", (event) => applyLanguage(event.target.value));
byId("hotkey-recorder").addEventListener("click", () => { state.hotkeyListening = true; fields.hotkey.classList.add("listening"); fields.hotkey.textContent = t("hotkey_listening"); });
window.addEventListener("keydown", (event) => {
  if (!state.hotkeyListening) return;
  event.preventDefault(); event.stopPropagation();
  if (event.key === "Escape") {
    state.hotkeyListening = false; fields.hotkey.classList.remove("listening"); fields.hotkey.textContent = prettyHotkey(state.hotkey); return;
  }
  const hotkey = hotkeyFromEvent(event);
  if (!hotkey) return;
  state.hotkey = hotkey; state.hotkeyListening = false; fields.hotkey.classList.remove("listening"); fields.hotkey.textContent = prettyHotkey(hotkey);
}, true);
byId("test-mic").addEventListener("click", async () => {
  const button = byId("test-mic"); button.disabled = true; byId("mic-result").textContent = t("checking");
  try { const level = await invoke("test_input_level"); byId("mic-meter").style.width = `${Math.max(3, level * 100)}%`; byId("mic-result").textContent = `${t("mic_ok")}: ${Math.round(level * 100)}%`; }
  catch (error) { byId("mic-result").textContent = `${t("mic_failed")}: ${error}`; }
  finally { button.disabled = false; }
});
byId("check-worker").addEventListener("click", async () => {
  const output = byId("worker-result"); output.textContent = t("checking");
  if (!await saveConfig({ quiet: true })) return;
  try { output.textContent = await invoke("check_worker"); } catch (error) { output.textContent = `${t("check_failed")}: ${error}`; }
});
byId("check-update").addEventListener("click", checkForUpdate);
byId("install-update").addEventListener("click", async () => {
  byId("update-result").textContent = t("update_installing"); byId("install-update").disabled = true;
  try { await invoke("install_update"); } catch (error) { byId("update-result").textContent = String(error); byId("install-update").disabled = false; }
});
byId("copy-recovery").addEventListener("click", async () => { try { await invoke("copy_pending_text"); showPendingText(null); toast(t("copied")); } catch (error) { toast(String(error)); } });
byId("onboarding-download").addEventListener("click", setupOnboarding);
byId("fix-hotkey").addEventListener("click", () => { byId("onboarding").hidden = true; showSection("voice"); fields.hotkey.click(); });
byId("onboarding-skip").addEventListener("click", async () => {
  if (!state.system?.shortcut_ready) { byId("onboarding").hidden = true; showSection("voice"); fields.hotkey.click(); return; }
  state.config.onboarding_completed = true; fillConfig(await invoke("save_config", { config: state.config })); byId("onboarding").hidden = true; showSection("models");
});
byId("export-diagnostics").addEventListener("click", async () => {
  try {
    const files = await invoke("diagnostic_files");
    if (!window.confirm(`${t("export_confirm")}\n\n${files.map((file) => `• ${file}`).join("\n")}`)) return;
    const path = await dialog.save({ defaultPath: "rain-diagnostics.zip", filters: [{ name: "ZIP", extensions: ["zip"] }] });
    if (!path) return;
    await invoke("export_diagnostics", { path }); byId("diagnostic-result").textContent = t("exported");
  } catch (error) { byId("diagnostic-result").textContent = String(error); }
});
byId("open-logs").addEventListener("click", async () => { try { await invoke("open_log_directory"); } catch (error) { toast(String(error)); } });

listen("pending-text", ({ payload }) => showPendingText(payload));
listen("navigate", ({ payload }) => showSection(payload));
listen("models-changed", loadModels);
listen("runtime-changed", loadRuntime);
listen("check-update", checkForUpdate);
listen("update-available", ({ payload }) => { state.update = payload; byId("update-result").textContent = `${t("update_available")}: ${payload.version}`; byId("install-update").hidden = false; });
listen("gpu-fallback-required", async () => {
  if (!window.confirm(t("cpu_fallback"))) return;
  fields.devicePreference.value = "cpu";
  showSection("models");
  await saveConfig();
});
listen("model-download-progress", ({ payload }) => {
  if (!state.activeDownload || state.activeDownload.modelId !== payload.model_id) state.activeDownload = { modelId: payload.model_id, paused: false };
  state.activeDownload.percent = payload.total ? payload.downloaded / payload.total * 100 : 0;
  state.activeDownload.file = payload.file;
  modelProgress(payload.model_id);
  if (!byId("onboarding-progress").hidden && payload.model_id === "sensevoice-small") {
    const progress = byId("onboarding-progress"); progress.querySelector("i").style.width = `${state.activeDownload.percent}%`; progress.querySelector("span").textContent = `${state.activeDownload.percent.toFixed(1)}%`;
  }
});
listen("runtime-download-progress", ({ payload }) => {
  const percent = payload.total ? Math.max(0, Math.min(100, payload.downloaded / payload.total * 100)) : 0;
  const progress = byId("runtime-download-progress");
  progress.hidden = false;
  progress.querySelector("i").style.width = `${percent}%`;
  progress.querySelector("span").textContent = `${percent.toFixed(1)}%`;
  if (!byId("onboarding-progress").hidden) {
    const onboardingProgress = byId("onboarding-progress");
    onboardingProgress.querySelector("i").style.width = `${percent}%`;
    onboardingProgress.querySelector("span").textContent = `${t("runtime_downloading")} · ${percent.toFixed(1)}%`;
  }
});

renderThemeToggle();
load();
