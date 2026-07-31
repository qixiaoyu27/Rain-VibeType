const { invoke } = window.__TAURI__.core;
const { listen, emit } = window.__TAURI__.event;
const dialog = window.__TAURI__.dialog;
const currentWindow = window.__TAURI__.window.getCurrentWindow();
const isMac = /Mac|iPhone|iPad/.test(navigator.platform);
const platformDefaultHotkey = isMac ? "Super+Shift+Space" : "Ctrl+Shift+Space";

const byId = (id) => document.getElementById(id);
const dictionaries = {
  "zh-CN": {
    nav_app_settings: "应用设置", app_settings_title: "应用设置", nav_model_library: "模型库", model_library_title: "模型库", nav_text_polish: "文本整理", text_polish_page_title: "文本整理", nav_runtime_settings: "推理设置", runtime_settings_title: "推理设置", output_page_title: "文字写入", feedback_page_title: "状态反馈", voice_page_description: "设置录音方式、全局快捷键和麦克风输入。", output_page_description: "选择识别文字的写入方式与剪贴板保护策略。", feedback_page_description: "调整状态浮窗、显示透明度和操作提示音。", models_page_description: "下载、切换和管理本地语音模型。", text_polish_page_description: "使用本地小模型整理标点、分段、停顿词和表达。", runtime_page_description: "选择推理设备并管理模型的加载与卸载。", app_page_description: "设置开机启动、界面语言和软件更新。", privacy_page_description: "管理本地诊断、崩溃报告与日志导出。", home_test_input_label: "输入框", home_test_input_placeholder: "点击这里，然后按住快捷键说话", text_polish_idle_minutes: "空闲休眠（分钟）",
    brand_name: "雨音输入法", section_safety: "安全保护",
    nav_home: "首页", nav_voice: "录音与快捷键", nav_output: "文字写入", nav_feedback: "状态反馈", nav_privacy: "隐私与诊断", current_device: "当前推理设备", current_model: "当前模型", home_instruction: "点击输入框，按住 {hotkey} 说话，松开后直接输入",
    current_hotkey: "当前快捷键", recovery_label: "待恢复文字", recovery_title: "目标位置和剪贴板均不可用，请手动复制", copy: "复制",
    voice_title: "录音与快捷键", recording_mode: "录音方式", push_to_talk: "按住说话，松开识别", toggle_mode: "按一次开始，再按一次结束", hotkey: "全局快捷键", hotkey_hint: "点击后按下新的组合键；注册失败会自动保留旧快捷键。", duration: "最长录音时长（秒）", input_device: "输入设备", duck_system_audio: "录音时降低电脑声音", duck_system_audio_hint: "开启后，录音期间系统播放音量会降至原来的 20%，结束后自动恢复。", mic_test: "麦克风测试", test_500ms: "测试 0.5 秒",
    refresh: "刷新", check_model_updates: "检查模型更新", model_catalog_latest: "模型清单已经是最新版本", model_catalog_updated: "模型清单已更新，请确认后再下载新版本", runtime_missing: "未安装", runtime_downloading: "正在下载推理组件", storage_dir: "模型存储目录", choose: "选择", reset: "恢复默认", storage_hint: "更改目录前请先删除已安装模型；模型不会自动迁移。", device: "推理设备", auto_device: "自动选择", cpu_hint: "CPU 模式功能完整，但大型模型的识别速度可能明显较慢。", load_mode: "模型加载方式", on_demand: "录音时并行加载", resident: "常驻内存", unload_policy: "模型卸载策略", unload_now: "识别后立即卸载", unload_idle: "空闲后卸载", unload_session: "退出时卸载", idle_timeout: "空闲卸载（秒）", python_path: "开发者 Python Worker 路径", python_hint: "仅供源码调试回退使用；正式安装版会优先使用上方已校验的推理组件。", check_worker: "检查 Worker",
    autostart: "登录 Windows 时启动", autostart_hint: "默认开启，应用启动后驻留系统托盘。", auto_update: "启动时检查更新", auto_update_hint: "只检查签名更新，不会自动安装。", language: "界面语言", follow_system: "跟随系统", updates: "软件更新", check_update: "检查更新", install_update: "安装并重启", theme_light: "浅色模式", theme_dark: "深色模式", switch_theme_light: "切换到浅色模式", switch_theme_dark: "切换到深色模式",
    injection: "输入方式", clipboard_mode: "剪贴板粘贴（推荐）", typing_mode: "模拟逐字输入", injection_hint: "粘贴兼容性更好；逐字输入不会使用剪贴板，但速度较慢。", restore_clipboard: "粘贴后恢复剪贴板", restore_hint: "若期间检测到用户复制了新内容，Rain 不会覆盖它。", remove_terminal_period: "去除结尾句号", remove_terminal_period_hint: "开启后自动去除识别结果末尾的“。”或“.”，其他标点保持不变。", target_validation: "目标窗口二次校验", target_hint: "录音开始时记录输入位置；识别结束后若位置已变化，结果只复制到剪贴板，不会误写到其他窗口。", text_polish: "文本整理", text_polish_hint: "识别后在本机整理标点、分段和表达。", text_polish_components: "可选组件", text_polish_rewrite: "文本润色", text_polish_rewrite_hint: "轻度调整措辞、语序和重复表达，同时保护原意、数字、英文和受保护词。", text_polish_paragraphs: "自动分段", text_polish_fillers: "删除停顿词", text_polish_fillers_hint: "仅处理“嗯、呃、额”等明确停顿词。", text_polish_terms: "受保护词", text_polish_terms_placeholder: "每行一个专名", download_text_model: "下载文本模型", delete_text_model: "删除文本模型", text_polish_ready: "已就绪 · Qwen3 0.6B · CPU", text_polish_missing_both: "需要下载文本模型和 CPU 推理框架", text_polish_missing_model: "需要下载文本模型", text_polish_missing_runtime: "需要下载 CPU 推理框架", text_polish_disabled: "组件已就绪，功能未开启", confirm_delete_text_model: "删除文本整理模型？",
    overlay: "显示状态悬浮条", overlay_hint: "显示录音音量、时长、模型加载、识别和错误状态。", overlay_transparency: "浮窗透明度", overlay_transparency_hint: "只调整背景，文字和按钮保持清晰。", overlay_fullscreen: "全屏应用中也显示", overlay_fullscreen_hint: "关闭后，全屏游戏或演示时只使用声音反馈。", start_sound: "开始录音提示音", stop_sound: "结束录音提示音", error_sound: "错误提示音",
    privacy_title: "隐私与诊断", privacy_banner: "默认不上传，不留录音，不留文字历史。", privacy_detail: "诊断事件只记录错误代码、模型 ID 与耗时。导出前会列出文件并由你选择保存位置。", crash_reports: "匿名崩溃报告", crash_hint: "默认关闭。当前构建只生成本地崩溃标记，不包含音频或识别文字。", diagnostics: "诊断工具", export_diagnostics: "导出诊断包", open_logs: "打开日志目录",
    onboarding_title: "自动配置本地语音识别", onboarding_text: "Rain 会下载轻量原生推理组件和 SenseVoice Small 原生模型；全部校验通过后即可使用。", download_default: "下载并自动配置", skip: "暂时跳过", onboarding_autostart: "Rain 默认随 Windows 登录启动并驻留托盘，可随时在通用设置中关闭。", onboarding_privacy: "模型与推理组件从发布清单指定的 HTTPS 地址下载；语音识别始终在本机进行。", hotkey_conflict: "默认快捷键不可用", record_new_hotkey: "录制新快捷键",
    ready: "可以开始使用", needs_model: "尚未配置模型", needs_model_detail: "下载默认模型，或在模型管理页导入已有模型", not_selected: "未选择", typing: "逐字输入", system_default: "跟随系统默认设备", saved: "设置已保存", save_failed: "保存失败", copied: "识别结果已复制", checking: "正在检查…", mic_ok: "输入电平", mic_failed: "麦克风测试失败", no_models: "无法读取模型清单", current: "当前使用", current_update: "当前使用 · 可更新", installed: "已安装", preview_enabled: "实时预览已启用", custom: "本地自定义", not_installed: "未安装", model_update_available: "可更新", use_model: "使用此模型", download: "下载", download_update: "下载新版本", pause: "暂停", resume: "继续下载", verify: "校验", delete: "删除", import_dir: "导入目录", import_zip: "导入 ZIP", official_source: "官方源", size: "下载 / 安装", hardware: "建议硬件", speed: "速度", languages: "语言", license: "许可证", adapter: "适配器", model_id: "模型 ID", downloading: "正在下载", download_paused: "下载已暂停，可继续", verified: "模型结构或完整性校验通过", confirm_delete: "确定删除该模型及未完成下载吗？", confirm_delete_old: "新版本已经验证并切换成功。是否删除旧版本：", import_done: "模型已导入并设为当前模型", check_failed: "检查失败", export_confirm: "诊断包将包含以下文件，不包含音频或识别文字：", exported: "诊断包已导出", update_latest: "当前已经是最新版本", update_available: "发现新版本", update_installing: "正在下载签名更新并安装…", update_not_configured: "此构建尚未配置更新签名公钥", all_feedback_confirm: "你正在关闭所有视觉和声音反馈。录音可能在没有明显提示的情况下进行，确定继续吗？", hotkey_listening: "请按新的组合键…", model_busy: "另一个模型正在下载", cpu_fallback: "GPU 模型加载失败。是否将推理设备改为 CPU 后重试？", crash_summary: "Rain 上次异常退出。以下是将提交的匿名数据摘要，确定发送吗？", crash_sent: "匿名崩溃报告已提交"
  },
  en: {
    nav_app_settings: "App settings", app_settings_title: "App settings", nav_model_library: "Model library", model_library_title: "Model library", nav_text_polish: "Text cleanup", text_polish_page_title: "Text cleanup", nav_runtime_settings: "Inference", runtime_settings_title: "Inference settings", output_page_title: "Text insertion", feedback_page_title: "Status feedback", voice_page_description: "Configure recording behavior, the global hotkey, and microphone input.", output_page_description: "Choose how recognized text is inserted and how the clipboard is protected.", feedback_page_description: "Adjust the status overlay, its transparency, and notification sounds.", models_page_description: "Download, switch, and manage local speech models.", text_polish_page_description: "Use a local model to clean punctuation, paragraphs, filler words, and phrasing.", runtime_page_description: "Choose the inference device and control model loading and unloading.", app_page_description: "Configure startup, interface language, and software updates.", privacy_page_description: "Manage local diagnostics, crash reports, and log exports.", home_test_input_label: "Input", home_test_input_placeholder: "Click here, then hold the hotkey and speak", text_polish_idle_minutes: "Idle sleep (minutes)",
    brand_name: "Rain Vibetype", section_safety: "SAFETY",
    nav_home: "Home", nav_voice: "Recording & hotkey", nav_output: "Text input", nav_feedback: "Feedback", nav_privacy: "Privacy & diagnostics", current_device: "Current inference device", current_model: "Current model", home_instruction: "Click the input box, hold {hotkey} to speak, then release to type",
    current_hotkey: "Current hotkey", recovery_label: "Text to recover", recovery_title: "The target and clipboard were unavailable. Copy this text manually.", copy: "Copy",
    voice_title: "Recording & hotkey", recording_mode: "Recording mode", push_to_talk: "Hold to speak, release to transcribe", toggle_mode: "Press once to start and again to stop", hotkey: "Global hotkey", hotkey_hint: "Click and press a new combination. A failed registration keeps the previous hotkey.", duration: "Maximum recording length (seconds)", input_device: "Input device", duck_system_audio: "Lower PC audio while recording", duck_system_audio_hint: "Reduces system playback to 20% of its previous volume while recording, then restores it.", mic_test: "Microphone test", test_500ms: "Test for 0.5 seconds",
    refresh: "Refresh", check_model_updates: "Check model updates", model_catalog_latest: "The model catalog is up to date", model_catalog_updated: "The model catalog was updated; confirm before downloading a new model version", runtime_missing: "Not installed", runtime_downloading: "Downloading inference component", storage_dir: "Model storage folder", choose: "Choose", reset: "Use default", storage_hint: "Delete installed models before changing the folder; models are not migrated automatically.", device: "Inference device", auto_device: "Auto", cpu_hint: "CPU mode is fully supported, but recognition may be much slower with large models.", load_mode: "Model loading", on_demand: "Load alongside recording", resident: "Keep in memory", unload_policy: "Unload policy", unload_now: "Immediately after recognition", unload_idle: "After idle timeout", unload_session: "When Rain exits", idle_timeout: "Idle timeout (seconds)", python_path: "Developer Python Worker path", python_hint: "Source builds may fall back to Python. Installed builds prefer the verified component above.", check_worker: "Check Worker",
    autostart: "Start when signing in to Windows", autostart_hint: "Enabled by default. Rain stays in the system tray.", auto_update: "Check for updates at startup", auto_update_hint: "Checks signed updates only and never installs automatically.", language: "Interface language", follow_system: "Follow system", updates: "Software update", check_update: "Check for updates", install_update: "Install and restart", theme_light: "Light", theme_dark: "Dark", switch_theme_light: "Switch to light mode", switch_theme_dark: "Switch to dark mode",
    injection: "Input method", clipboard_mode: "Clipboard paste (recommended)", typing_mode: "Simulated typing", injection_hint: "Paste is more compatible. Simulated typing avoids the clipboard but is slower.", restore_clipboard: "Restore clipboard after paste", restore_hint: "Rain will not overwrite new content copied while recognition is running.", remove_terminal_period: "Remove final period", remove_terminal_period_hint: "Removes a final “。” or “.” from recognized text while preserving other punctuation.", target_validation: "Target revalidation", target_hint: "Rain records the input target when recording starts. If it changes, text is copied instead of being written into the wrong window.", text_polish: "Text polishing", text_polish_hint: "Polish punctuation, paragraphs, and phrasing locally after recognition.", text_polish_components: "Optional components", text_polish_rewrite: "Rewrite for clarity", text_polish_rewrite_hint: "Lightly improves wording, order, and repetition while preserving meaning, numbers, English text, and protected terms.", text_polish_paragraphs: "Automatic paragraphs", text_polish_fillers: "Remove hesitation words", text_polish_fillers_hint: "Only removes clear hesitation words.", text_polish_terms: "Protected terms", text_polish_terms_placeholder: "One proper name per line", download_text_model: "Download text model", delete_text_model: "Delete text model", text_polish_ready: "Ready · Qwen3 0.6B · CPU", text_polish_missing_both: "Download the text model and CPU inference framework", text_polish_missing_model: "Download the text model", text_polish_missing_runtime: "Download CPU inference framework", text_polish_disabled: "Components are ready; the feature is off", confirm_delete_text_model: "Delete the text polishing model?",
    overlay: "Show status overlay", overlay_hint: "Shows audio level, duration, model loading, recognition and errors.", overlay_transparency: "Overlay transparency", overlay_transparency_hint: "Only the background changes; text and controls stay clear.", overlay_fullscreen: "Show over full-screen apps", overlay_fullscreen_hint: "Disable to use audio feedback only in games or presentations.", start_sound: "Recording start sound", stop_sound: "Recording stop sound", error_sound: "Error sound",
    privacy_title: "Privacy & diagnostics", privacy_banner: "No upload by default. No audio. No text history.", privacy_detail: "Diagnostic events contain only error codes, model IDs and timings. Files are listed before export and you choose the destination.", crash_reports: "Anonymous crash reports", crash_hint: "Off by default. This build creates a local crash marker only, without audio or recognized text.", diagnostics: "Diagnostic tools", export_diagnostics: "Export diagnostics", open_logs: "Open log folder",
    onboarding_title: "Set up local speech recognition", onboarding_text: "Rain downloads the lightweight native runtime and native SenseVoice Small model, then enables them after verification.", download_default: "Download and configure", skip: "Skip for now", onboarding_autostart: "Rain starts with Windows and stays in the tray by default. You can disable this in General settings.", onboarding_privacy: "Components and models use HTTPS locations from release manifests; recognition always runs locally.", hotkey_conflict: "The default hotkey is unavailable", record_new_hotkey: "Record a new hotkey",
    ready: "Ready", needs_model: "No model configured", needs_model_detail: "Download the default model or import one in Model management", not_selected: "Not selected", typing: "Simulated typing", system_default: "System default device", saved: "Settings saved", save_failed: "Could not save", copied: "Recognition result copied", checking: "Checking…", mic_ok: "Input level", mic_failed: "Microphone test failed", no_models: "Could not read model manifest", current: "In use", current_update: "In use · update available", installed: "Installed", preview_enabled: "Live preview enabled", custom: "Local custom", not_installed: "Not installed", model_update_available: "Update available", use_model: "Use this model", download: "Download", download_update: "Download new version", pause: "Pause", resume: "Resume", verify: "Verify", delete: "Delete", import_dir: "Import folder", import_zip: "Import ZIP", official_source: "Official source", size: "Download / installed", hardware: "Recommended", speed: "Speed", languages: "Languages", license: "License", adapter: "Adapter", model_id: "Model ID", downloading: "Downloading", download_paused: "Download paused; it can be resumed", verified: "Model structure or integrity verified", confirm_delete: "Delete this model and any partial download?", confirm_delete_old: "The new version is verified and selected. Delete old versions:", import_done: "Model imported and selected", check_failed: "Check failed", export_confirm: "The diagnostic archive will contain these files and no audio or recognized text:", exported: "Diagnostics exported", update_latest: "You already have the latest version", update_available: "New version available", update_installing: "Downloading and installing the signed update…", update_not_configured: "This build has no update signing public key", all_feedback_confirm: "You are disabling all visual and audio feedback. Recording may occur without an obvious indicator. Continue?", hotkey_listening: "Press a new key combination…", model_busy: "Another model is downloading", cpu_fallback: "GPU model loading failed. Change the inference device to CPU and retry?", crash_summary: "Rain exited unexpectedly last time. Send this anonymous data summary?", crash_sent: "Anonymous crash report sent"
  }
};

Object.assign(dictionaries["zh-CN"], {
  minimize: "最小化", close: "关闭", settings_navigation: "设置导航",
  runtime_auto_install_hint: "下载或修复当前模型时，会自动安装所需的推理框架。",
  repair_model_components: "修复模型组件",
  download_text_model: "下载并自动配置",
  text_polish_components: "文本模型与自动依赖",
  text_polish_missing_both: "点击下载，雨音会自动安装文本模型和所需推理框架",
  text_polish_missing_runtime: "推理框架缺失，点击即可自动修复",
  confirm_delete_text_model: "删除文本整理模型？如果它是最后一个使用当前推理框架的模型，框架也会自动删除。",
  onboarding_text: "下载 SenseVoice Small 后，雨音会自动安装并配置它需要的轻量原生推理框架。"
});
Object.assign(dictionaries.en, {
  minimize: "Minimize", close: "Close", settings_navigation: "Settings navigation",
  runtime_auto_install_hint: "Downloading or repairing the current model automatically installs its required inference runtime.",
  repair_model_components: "Repair model components",
  download_text_model: "Download and configure",
  text_polish_components: "Text model and automatic dependencies",
  text_polish_missing_both: "Download once to install the text model and its required inference runtime automatically",
  text_polish_missing_runtime: "The inference runtime is missing; click to repair it automatically",
  confirm_delete_text_model: "Delete the text polishing model? If it is the last model using its inference runtime, that runtime will also be removed automatically.",
  onboarding_text: "Download SenseVoice Small and Rain will automatically install and configure its lightweight native inference runtime."
});

if (isMac) {
  Object.assign(dictionaries["zh-CN"], {
    autostart: "登录 macOS 时启动",
    autostart_hint: "默认开启，应用启动后驻留菜单栏。",
    onboarding_autostart: "Rain 默认随 macOS 登录启动并驻留菜单栏，可随时在通用设置中关闭。",
    cpu_hint: "Apple Silicon 原生模式功能完整，SenseVoice 使用 ARM 优化 CPU；文本整理可使用 Metal。"
  });
  Object.assign(dictionaries.en, {
    autostart: "Start when signing in to macOS",
    autostart_hint: "Enabled by default. Rain stays in the menu bar.",
    onboarding_autostart: "Rain starts with macOS and stays in the menu bar by default. You can disable this in General settings.",
    cpu_hint: "Native Apple Silicon mode is fully supported: SenseVoice uses the ARM-optimized CPU path and text cleanup can use Metal."
  });
}

const fields = {
  recordingMode: byId("recording-mode"), hotkey: byId("hotkey-recorder"), maxRecordingSeconds: byId("max-recording-seconds"), inputDevice: byId("input-device"), duckSystemAudio: byId("duck-system-audio"),
  modelStorageDir: byId("model-storage-dir"), pythonPath: byId("python-path"), devicePreference: byId("device-preference"), modelLoadMode: byId("model-load-mode"), unloadPolicy: byId("unload-policy"), idleTimeoutSeconds: byId("idle-timeout-seconds"),
  autostart: byId("autostart"), autoCheckUpdates: byId("auto-check-updates"), uiLanguage: byId("ui-language"),
  injectionMethod: byId("injection-method"), restoreClipboard: byId("restore-clipboard"), removeTerminalPeriod: byId("remove-terminal-period"),
  textPolishEnabled: byId("text-polish-enabled"), textPolishRewrite: byId("text-polish-rewrite"), textPolishRemoveFillers: byId("text-polish-remove-fillers"), textPolishParagraphs: byId("text-polish-paragraphs"), textPolishProtectedTerms: byId("text-polish-protected-terms"), textPolishIdleTimeout: byId("text-polish-idle-timeout"),
  showOverlay: byId("show-overlay"), overlayTransparency: byId("overlay-transparency"), showOverlayFullscreen: byId("show-overlay-fullscreen"), startSound: byId("start-sound"), stopSound: byId("stop-sound"), errorSound: byId("error-sound"),
  anonymousCrashReports: byId("anonymous-crash-reports")
};

const state = { config: null, system: null, runtime: null, textPolish: null, models: [], language: "zh-CN", hotkey: platformDefaultHotkey, hotkeyListening: false, activeDownload: null, activeTextModelDownload: false };

if (isMac) {
  document.documentElement.dataset.platform = "macos";
  fields.devicePreference.querySelector('option[value="cuda"]')?.remove();
}
fields.hotkey.textContent = prettyHotkey(platformDefaultHotkey);
byId("home-hotkey").textContent = prettyHotkey(platformDefaultHotkey);

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
  document.querySelectorAll("[data-i18n-placeholder]").forEach((node) => { node.placeholder = t(node.dataset.i18nPlaceholder); });
  document.querySelectorAll("[data-i18n-aria]").forEach((node) => { node.setAttribute("aria-label", t(node.dataset.i18nAria)); });
  if (state.config) updateHome();
  renderModels();
  renderTextPolish();
  renderThemeToggle();
  emit("ui-language-changed", state.language).catch(() => {});
}

function toast(message) {
  const node = byId("toast");
  node.textContent = message;
  node.classList.add("show");
  clearTimeout(toast.timer);
  toast.timer = setTimeout(() => node.classList.remove("show"), 2600);
}

let onboardingReturnFocus;
function setOnboardingVisible(visible) {
  const onboarding = byId("onboarding");
  const shell = document.querySelector(".window-shell");
  if (visible) {
    if (!onboarding.hidden) return;
    const active = document.activeElement;
    onboardingReturnFocus = active instanceof HTMLElement && shell.contains(active) ? active : byId("home-test-input");
    shell.inert = true;
    onboarding.hidden = false;
    requestAnimationFrame(() => byId("onboarding-download").focus());
    return;
  }
  onboarding.hidden = true;
  shell.inert = false;
  const target = onboardingReturnFocus;
  onboardingReturnFocus = null;
  if (target?.isConnected) target.focus();
}

function showSection(id) {
  const target = id === "model" ? "models" : id;
  document.querySelectorAll(".page").forEach((page) => page.classList.toggle("active", page.id === target));
  document.querySelectorAll(".nav-item").forEach((item) => item.classList.toggle("active", item.dataset.section === target));
  document.querySelector(".content").scrollTop = 0;
}

function prettyHotkey(value) {
  const keys = value.split("+");
  if (!isMac) return keys.join(" + ");
  const symbols = { Super: "⌘", Meta: "⌘", Shift: "⇧", Alt: "⌥", Ctrl: "⌃", Control: "⌃" };
  return keys.map((key) => symbols[key] || key).join(" ");
}
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
  fields.removeTerminalPeriod.checked = config.remove_terminal_period;
  fields.textPolishEnabled.checked = config.text_polish_enabled;
  fields.textPolishRewrite.checked = config.text_polish_rewrite;
  fields.textPolishRemoveFillers.checked = config.text_polish_remove_fillers;
  fields.textPolishParagraphs.checked = config.text_polish_paragraphs;
  fields.textPolishProtectedTerms.value = (config.text_polish_protected_terms || []).join("\n");
  fields.textPolishIdleTimeout.value = String((config.text_polish_idle_timeout_seconds || 600) / 60);
  fields.showOverlay.checked = config.show_overlay;
  fields.overlayTransparency.value = String(Math.round((1 - config.overlay_opacity) * 100));
  byId("overlay-transparency-value").textContent = `${fields.overlayTransparency.value}%`;
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
  const autostartError = byId("autostart-error");
  autostartError.textContent = status.autostart_error || "";
  autostartError.hidden = status.autostart_ready || !status.autostart_error;
}

async function loadRuntime() {
  try {
    state.runtime = await invoke("get_runtime_status");
    renderModels();
    updateHome();
    return state.runtime;
  } catch {
    return null;
  }
}

function renderTextPolish() {
  if (!byId("text-polish-status")) return;
  const hasUpdate = state.textPolish?.model?.state === "update_available";
  const modelReady = ["installed", "custom", "update_available"].includes(state.textPolish?.model?.state);
  const runtimeReady = Boolean(state.textPolish?.runtime?.ready);
  const ready = modelReady && runtimeReady;
  byId("text-polish-status").textContent = ready
    ? fields.textPolishEnabled.checked ? t("text_polish_ready") : t("text_polish_disabled")
    : !modelReady && !runtimeReady ? t("text_polish_missing_both")
      : !modelReady ? t("text_polish_missing_model") : t("text_polish_missing_runtime");
  byId("download-text-model").disabled = ready && !hasUpdate && !state.activeTextModelDownload;
  byId("download-text-model").textContent = state.activeTextModelDownload ? t("pause") : hasUpdate ? t("download_update") : modelReady && !runtimeReady ? t("repair_model_components") : t("download_text_model");
  byId("delete-text-model").disabled = !modelReady || state.activeTextModelDownload;
  byId("text-polish-model-progress").hidden = !state.activeTextModelDownload;
}

async function loadTextPolish() {
  try {
    state.textPolish = await invoke("get_text_polish_status");
  } catch (error) {
    state.textPolish = null;
    byId("text-polish-status").textContent = `${t("check_failed")}: ${error}`;
  }
  renderTextPolish();
  return state.textPolish;
}

async function downloadTextModel() {
  if (state.activeTextModelDownload) {
    try { await invoke("pause_model_download", { modelId: "qwen3-0-6b-text" }); } catch (error) { toast(String(error)); }
    return;
  }
  state.activeTextModelDownload = true;
  renderTextPolish();
  try {
    await invoke("download_model", { modelId: "qwen3-0-6b-text" });
  } catch (error) {
    toast(String(error));
  } finally {
    state.activeTextModelDownload = false;
    await loadTextPolish();
    renderModels();
  }
}

async function deleteTextComponent(command, confirmation, title) {
  if (!await dialog.confirm(t(confirmation), { title: t(title), kind: "warning" })) return;
  try {
    await invoke(command);
    await loadTextPolish();
  } catch (error) {
    toast(String(error));
  }
}

function collectConfig() {
  const anyFeedback = fields.showOverlay.checked || fields.startSound.checked || fields.stopSound.checked || fields.errorSound.checked;
  return {
    ...state.config,
    schema_version: 2,
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
    remove_terminal_period: fields.removeTerminalPeriod.checked,
    text_polish_enabled: fields.textPolishEnabled.checked,
    text_polish_rewrite: fields.textPolishRewrite.checked,
    text_polish_remove_fillers: fields.textPolishRemoveFillers.checked,
    text_polish_paragraphs: fields.textPolishParagraphs.checked,
    text_polish_protected_terms: [...new Set(fields.textPolishProtectedTerms.value.split(/[\n,，]/).map((term) => term.trim()).filter(Boolean))],
    text_polish_idle_timeout_seconds: Math.round(Number(fields.textPolishIdleTimeout.value) * 60),
    show_overlay: fields.showOverlay.checked,
    overlay_opacity: Number((1 - Number(fields.overlayTransparency.value) / 100).toFixed(2)),
    show_overlay_fullscreen: fields.showOverlayFullscreen.checked,
    start_sound: fields.startSound.checked,
    stop_sound: fields.stopSound.checked,
    error_sound: fields.errorSound.checked,
    feedback_disabled_confirmed: anyFeedback ? false : Boolean(state.config.feedback_disabled_confirmed),
    anonymous_crash_reports: fields.anonymousCrashReports.checked
  };
}

async function saveConfig({ quiet = false } = {}) {
  if (!state.config) return null;
  const config = collectConfig();
  const runtimeChanged = config.device_preference !== state.config.device_preference || config.python_path !== state.config.python_path;
  const allFeedbackOff = !config.show_overlay && !config.start_sound && !config.stop_sound && !config.error_sound;
  if (allFeedbackOff && !config.feedback_disabled_confirmed) {
    if (!window.confirm(t("all_feedback_confirm"))) { fillConfig(state.config); return null; }
    config.feedback_disabled_confirmed = true;
  }
  try {
    let saved = await invoke("save_config", { config });
    state.config = saved;
    if (runtimeChanged) await loadRuntime();
    fillSystemStatus(await invoke("get_system_status"));
    if (state.system.shortcut_ready && state.runtime?.ready && saved.model_path && !saved.onboarding_completed) {
      saved.onboarding_completed = true;
      saved = await invoke("save_config", { config: saved });
      state.config = saved;
      setOnboardingVisible(false);
    }
    if (!quiet) toast(t("saved"));
    return saved;
  } catch (error) {
    toast(`${t("save_failed")}: ${error}`);
    try { fillConfig(await invoke("get_config")); } catch { fillConfig(state.config); }
    return null;
  }
}

const configControls = [...document.querySelectorAll(".settings-grid input:not([readonly]), .settings-grid select, .settings-grid textarea")];
let configSaveTimer;
let configSaveQueue = Promise.resolve(true);

function configControlsAreValid() {
  return configControls.every((control) => control.checkValidity() && (control.type !== "number" || control.value !== ""));
}

function enqueueConfigSave() {
  configSaveQueue = configSaveQueue.catch(() => null).then(() => saveConfig({ quiet: true }));
  return configSaveQueue;
}

function scheduleConfigSave() {
  if (!state.config || !configControlsAreValid()) return;
  clearTimeout(configSaveTimer);
  configSaveTimer = setTimeout(enqueueConfigSave, 300);
}

function flushConfigSave() {
  clearTimeout(configSaveTimer);
  if (!state.config || !configControlsAreValid()) return Promise.resolve(null);
  return enqueueConfigSave();
}

const waveformHistory = [];
let waveformState = "idle";
let waveformLevel = 0;
let waveformTick = 0;

function normalizedAudioLevel(level) {
  return Math.min(1, Math.sqrt(Math.max(0, Number(level) || 0)) * 2.4);
}

console.assert(normalizedAudioLevel(-1) === 0 && normalizedAudioLevel(1) === 1);

function drawHomeWaveform(advance = true) {
  const canvas = byId("home-waveform");
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  if (!width || !height) return;
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  if (canvas.width !== Math.round(width * dpr) || canvas.height !== Math.round(height * dpr)) {
    canvas.width = Math.round(width * dpr);
    canvas.height = Math.round(height * dpr);
  }
  const count = Math.max(32, Math.floor(width / 8));
  while (waveformHistory.length < count) waveformHistory.unshift(0);
  if (waveformHistory.length > count) waveformHistory.splice(0, waveformHistory.length - count);
  const active = waveformState === "recording";
  const level = normalizedAudioLevel(waveformLevel);
  if (advance) {
    waveformTick += 1;
    if (active) {
      const variation = .68 + .32 * ((Math.sin(waveformTick * 1.73) + 1) / 2);
      waveformHistory.push(Math.max(.035, level * variation));
      waveformHistory.shift();
    } else {
      waveformHistory.fill(0);
    }
  }
  const context = canvas.getContext("2d");
  context.setTransform(dpr, 0, 0, dpr, 0, 0);
  context.clearRect(0, 0, width, height);
  const gradient = context.createLinearGradient(0, 0, width, 0);
  gradient.addColorStop(0, "#16b8c4");
  gradient.addColorStop(.7, "#168bbd");
  gradient.addColorStop(1, "#d69a62");
  context.strokeStyle = gradient;
  context.lineWidth = 3;
  context.lineCap = "round";
  const middle = height / 2;
  waveformHistory.forEach((value, index) => {
    const x = (index + .5) * width / count;
    const barHeight = 2 + value * (height - 10);
    context.beginPath();
    context.moveTo(x, middle - barHeight / 2);
    context.lineTo(x, middle + barHeight / 2);
    context.stroke();
  });
  document.querySelector(".home-mic-orb").style.transform = `scale(${active ? 1 + level * .045 : 1})`;
}

function updateHome() {
  if (!state.config) return;
  const model = state.models.find((item) => item.id === state.config.selected_model_id);
  const modelReady = Boolean(state.config.model_path && ["installed", "custom", "update_available"].includes(model?.state));
  const ready = modelReady && Boolean(state.runtime?.ready);
  const modelName = model?.display_name || t("not_selected");
  const hotkey = prettyHotkey(state.config.hotkey);
  byId("status-orb").classList.toggle("error", !ready);
  byId("runtime-status").textContent = ready ? t("ready") : !state.runtime?.ready ? t("runtime_missing") : t("needs_model");
  byId("runtime-detail").textContent = ready ? t("home_instruction").replace("{hotkey}", hotkey) : !state.runtime?.ready ? t("runtime_auto_install_hint") : t("needs_model_detail");
  byId("home-device").textContent = !state.runtime ? t("checking") : state.runtime.recommended_accelerator === "nvidia" ? state.runtime.nvidia_name || "NVIDIA GPU" : isMac ? "Apple Silicon · Native" : "CPU";
  byId("home-model").textContent = modelName;
  byId("home-hotkey").textContent = hotkey;
  byId("home-recording-mode").textContent = state.config.recording_mode === "toggle" ? t("toggle_mode") : t("push_to_talk");
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
    await loadTextPolish();
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
  const file = state.activeDownload.file?.startsWith("runtime:") ? t("runtime_downloading") : state.activeDownload.file || "";
  node.querySelector("span").textContent = `${percent.toFixed(1)}% · ${file}`;
}

function renderModels() {
  if (!byId("model-list") || !state.models.length) return;
  byId("model-list").innerHTML = state.models.map((model, index) => {
    const selectable = model.purpose === "asr";
    const preview = model.purpose === "asr_preview";
    const hasUpdate = model.state === "update_available";
    const installed = model.state === "installed" || model.state === "custom" || hasUpdate;
    const custom = model.state === "custom";
    const selected = selectable && state.config?.selected_model_id === model.id && installed;
    const active = state.activeDownload?.modelId === model.id;
    const runtimeMissing = selected && !state.runtime?.ready;
    const downloadLabel = state.activeDownload?.paused && active ? t("resume") : runtimeMissing ? t("repair_model_components") : hasUpdate ? t("download_update") : t("download");
    return `<details class="model-card ${selected ? "selected" : ""}" style="--delay:${index * 55}ms" ${selected || active ? "open" : ""}>
      <summary class="model-heading"><div class="model-mark">${escapeHtml(model.display_name.slice(0, 2).toUpperCase())}</div><div><h2>${escapeHtml(model.display_name)}</h2><p>${escapeHtml(model.engine)} · ${escapeHtml(model.model_version)}</p></div><span class="state-pill ${installed ? "ready" : ""}">${selected ? (hasUpdate ? t("current_update") : t("current")) : custom ? t("custom") : hasUpdate ? t("model_update_available") : installed ? t(preview ? "preview_enabled" : "installed") : t("not_installed")}</span></summary>
      <div class="model-card-body">
        <dl class="model-meta"><div><dt>${t("model_id")}</dt><dd title="${escapeHtml(model.id)}">${escapeHtml(model.id)}</dd></div><div><dt>${t("languages")}</dt><dd>${escapeHtml(model.languages.join(" / "))}</dd></div><div><dt>${t("size")}</dt><dd>${formatBytes(model.download_size)} / ${formatBytes(model.installed_size)}</dd></div><div><dt>${t("hardware")}</dt><dd title="${escapeHtml(model.recommended_hardware)}">${escapeHtml(model.recommended_hardware)}</dd></div><div><dt>${t("speed")}</dt><dd>${escapeHtml(model.speed_grade)}</dd></div><div><dt>${t("license")}</dt><dd>${escapeHtml(model.license)}</dd></div><div><dt>${t("official_source")}</dt><dd title="${escapeHtml(model.official_source)}">${escapeHtml(model.official_source)}</dd></div><div><dt>${t("adapter")}</dt><dd title="${escapeHtml(model.adapter_compatibility)}">${escapeHtml(model.adapter_compatibility)}</dd></div></dl>
        <div class="download-progress" data-progress="${escapeHtml(model.id)}" ${active ? "" : "hidden"}><i></i><span>${t("downloading")}</span></div>
        <div class="model-actions">
          ${selectable && installed && !selected ? `<button class="primary" data-model-action="select" data-model-id="${escapeHtml(model.id)}">${t("use_model")}</button>` : ""}
          ${!installed || hasUpdate || runtimeMissing ? `<button class="primary" data-model-action="download" data-model-id="${escapeHtml(model.id)}">${downloadLabel}</button>` : ""}
          ${active && !state.activeDownload.paused ? `<button class="secondary" data-model-action="pause" data-model-id="${escapeHtml(model.id)}">${t("pause")}</button>` : ""}
          ${installed ? `<button class="secondary" data-model-action="verify" data-model-id="${escapeHtml(model.id)}">${t("verify")}</button>` : selectable ? `<button class="secondary" data-model-action="import-dir" data-model-id="${escapeHtml(model.id)}">${t("import_dir")}</button><button class="secondary" data-model-action="import-zip" data-model-id="${escapeHtml(model.id)}">${t("import_zip")}</button>` : ""}
          <button class="ghost danger" data-model-action="delete" data-model-id="${escapeHtml(model.id)}">${t("delete")}</button>
        </div>
      </div>
    </details>`;
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
    await invoke("download_model", { modelId });
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
        setOnboardingVisible(false);
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
    if (fields.devicePreference.value !== state.config.device_preference && !await flushConfigSave()) return;
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
      await invoke("delete_model", { modelId });
      fillConfig(await invoke("get_config"));
    }
    if (action === "import-dir") return await importModel(modelId, true);
    if (action === "import-zip") return await importModel(modelId, false);
  } catch (error) { toast(String(error)); }
  finally { if (button.isConnected) button.disabled = false; }
}

function hotkeyFromEvent(event) {
  const modifierKeys = new Set(["Control", "Shift", "Alt", "Meta"]);
  if (modifierKeys.has(event.key) || event.key === "Escape") return null;
  const parts = [];
  if (isMac && event.metaKey) parts.push("Super");
  if (event.ctrlKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  if (!isMac && event.metaKey) parts.push("Super");
  let key;
  if (/^Key[A-Z]$/.test(event.code)) key = event.code.slice(3);
  else if (/^Digit[0-9]$/.test(event.code)) key = event.code.slice(5);
  else key = event.code === "Space" ? "Space" : event.key.length === 1 ? event.key.toUpperCase() : event.key;
  key = ({ ArrowUp: "Up", ArrowDown: "Down", ArrowLeft: "Left", ArrowRight: "Right" })[key] || key;
  parts.push(key);
  return parts.join("+");
}

async function checkForUpdate() {
  const output = byId("update-result");
  byId("install-update").hidden = true;
  output.textContent = t("checking");
  try {
    const update = await invoke("check_update");
    if (update.available) {
      output.textContent = `${t("update_available")}: ${update.version}${update.notes ? ` · ${update.notes}` : ""}`;
      byId("install-update").hidden = false;
    } else output.textContent = `${t("update_latest")} (${update.current_version})`;
  } catch (error) {
    output.textContent = String(error).includes("UPDATE_NOT_CONFIGURED") ? t("update_not_configured") : `${t("check_failed")}: ${error}`;
  }
}

async function load() {
  let config;
  try {
    config = await invokeWhenReady("get_config");
    fillConfig(config);
    setOnboardingVisible(!config.onboarding_completed);
  } catch (error) {
    byId("runtime-status").textContent = t("check_failed");
    byId("runtime-detail").textContent = String(error);
    byId("status-orb").classList.add("error");
    return;
  }

  fields.inputDevice.replaceChildren(new Option(t("system_default"), ""));
  const [systemResult, devicesResult, pendingResult] = await Promise.allSettled([
    invokeWhenReady("get_system_status"),
    invokeWhenReady("list_input_devices"),
    invokeWhenReady("get_pending_text"),
    loadRuntime(),
    loadModels(),
    loadTextPolish()
  ]);
  if (systemResult.status === "fulfilled") fillSystemStatus(systemResult.value);
  if (devicesResult.status === "fulfilled") {
    devicesResult.value.forEach((name) => fields.inputDevice.add(new Option(name, name)));
  } else {
    byId("mic-result").textContent = `${t("mic_failed")}: ${devicesResult.reason}`;
  }
  if (config.input_device && ![...fields.inputDevice.options].some((option) => option.value === config.input_device)) {
    fields.inputDevice.add(new Option(config.input_device, config.input_device));
  }
  fields.inputDevice.value = config.input_device || "";
  if (pendingResult.status === "fulfilled") showPendingText(pendingResult.value);

  try {
    const crashReport = await invoke("pending_crash_report");
    if (crashReport) {
      if (window.confirm(`${t("crash_summary")}\n\n${JSON.stringify(crashReport, null, 2)}`)) {
        try { await invoke("submit_crash_report"); toast(t("crash_sent")); } catch (error) { toast(String(error)); }
      } else {
        await invoke("dismiss_crash_report");
      }
    }
  } catch (error) { toast(String(error)); }
}

document.querySelectorAll(".nav-item").forEach((item) => item.addEventListener("click", () => showSection(item.dataset.section)));
configControls.forEach((control) => control.addEventListener("input", scheduleConfigSave));
fields.overlayTransparency.addEventListener("input", () => {
  byId("overlay-transparency-value").textContent = `${fields.overlayTransparency.value}%`;
});
byId("theme-toggle").addEventListener("click", () => setTheme(document.documentElement.dataset.theme === "light" ? "dark" : "light"));
byId("minimize").addEventListener("click", () => currentWindow.minimize());
byId("close").addEventListener("click", () => currentWindow.hide());
byId("refresh-models").addEventListener("click", loadModels);
byId("check-model-updates").addEventListener("click", checkModelUpdates);
byId("download-text-model").addEventListener("click", downloadTextModel);
byId("delete-text-model").addEventListener("click", () => deleteTextComponent("delete_text_model", "confirm_delete_text_model", "delete_text_model"));
fields.textPolishEnabled.addEventListener("change", renderTextPolish);
byId("model-list").addEventListener("click", (event) => { const button = event.target.closest("[data-model-action]"); if (button) handleModelAction(button); });
byId("pick-storage-dir").addEventListener("click", async () => { const path = await dialog.open({ directory: true, multiple: false }); if (path) { fields.modelStorageDir.value = path; scheduleConfigSave(); } });
byId("reset-storage-dir").addEventListener("click", () => { fields.modelStorageDir.value = ""; scheduleConfigSave(); });
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
  state.hotkey = hotkey; state.hotkeyListening = false; fields.hotkey.classList.remove("listening"); fields.hotkey.textContent = prettyHotkey(hotkey); scheduleConfigSave();
}, true);
byId("test-mic").addEventListener("click", async () => {
  const button = byId("test-mic"); button.disabled = true; byId("mic-result").textContent = t("checking");
  try { const level = await invoke("test_input_level"); byId("mic-meter").style.width = `${Math.max(3, level * 100)}%`; byId("mic-result").textContent = `${t("mic_ok")}: ${Math.round(level * 100)}%`; }
  catch (error) { byId("mic-result").textContent = `${t("mic_failed")}: ${error}`; }
  finally { button.disabled = false; }
});
byId("check-worker").addEventListener("click", async () => {
  const output = byId("worker-result"); output.textContent = t("checking");
  if (!await flushConfigSave()) return;
  try { output.textContent = await invoke("check_worker"); } catch (error) { output.textContent = `${t("check_failed")}: ${error}`; }
});
byId("check-update").addEventListener("click", checkForUpdate);
byId("install-update").addEventListener("click", async () => {
  byId("update-result").textContent = t("update_installing"); byId("install-update").disabled = true;
  try { await invoke("install_update"); } catch (error) { byId("update-result").textContent = String(error); byId("install-update").disabled = false; }
});
byId("copy-recovery").addEventListener("click", async () => { try { await invoke("copy_pending_text"); showPendingText(null); toast(t("copied")); } catch (error) { toast(String(error)); } });
byId("onboarding-download").addEventListener("click", setupOnboarding);
byId("fix-hotkey").addEventListener("click", () => { setOnboardingVisible(false); showSection("voice"); fields.hotkey.click(); });
byId("onboarding-skip").addEventListener("click", async () => {
  if (!state.system?.shortcut_ready) { setOnboardingVisible(false); showSection("voice"); fields.hotkey.click(); return; }
  state.config.onboarding_completed = true; fillConfig(await invoke("save_config", { config: state.config })); setOnboardingVisible(false); showSection("models");
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
listen("update-available", ({ payload }) => { byId("update-result").textContent = `${t("update_available")}: ${payload.version}`; byId("install-update").hidden = false; });
listen("gpu-fallback-required", async () => {
  if (!window.confirm(t("cpu_fallback"))) return;
  fields.devicePreference.value = "cpu";
  showSection("models");
  await flushConfigSave();
});
listen("model-download-progress", ({ payload }) => {
  if (payload.model_id === "qwen3-0-6b-text") {
    state.activeTextModelDownload = true;
    const percent = payload.total ? Math.max(0, Math.min(100, payload.downloaded / payload.total * 100)) : 0;
    const progress = byId("text-polish-model-progress");
    progress.hidden = false;
    progress.querySelector("i").style.width = `${percent}%`;
    const file = payload.file?.startsWith("runtime:") ? t("runtime_downloading") : payload.file || "";
    progress.querySelector("span").textContent = `${percent.toFixed(1)}% · ${file}`;
    renderTextPolish();
    return;
  }
  if (!state.activeDownload || state.activeDownload.modelId !== payload.model_id) state.activeDownload = { modelId: payload.model_id, paused: false };
  state.activeDownload.percent = payload.total ? payload.downloaded / payload.total * 100 : 0;
  state.activeDownload.file = payload.file;
  modelProgress(payload.model_id);
  if (!byId("onboarding-progress").hidden && payload.model_id === "sensevoice-small") {
    const progress = byId("onboarding-progress");
    const label = payload.file?.startsWith("runtime:") ? t("runtime_downloading") : payload.file || t("downloading");
    progress.querySelector("i").style.width = `${state.activeDownload.percent}%`;
    progress.querySelector("span").textContent = `${label} · ${state.activeDownload.percent.toFixed(1)}%`;
  }
});
listen("audio-level", ({ payload }) => {
  waveformState = payload.state;
  waveformLevel = payload.level;
  if (!document.hidden) drawHomeWaveform();
});
window.addEventListener("resize", () => drawHomeWaveform(false));
document.addEventListener("visibilitychange", () => {
  if (!document.hidden) drawHomeWaveform(false);
});

renderThemeToggle();
drawHomeWaveform();
load();
