#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod config;
mod diagnostics;
mod models;
mod platform_windows;
mod runtimes;
mod text_polish;
mod worker;

use config::Config;
use models::{DownloadProgress, ImportResult, ModelCard, ModelDefinition, ModelRepository};
use platform_windows::InputTarget;
use runtimes::{RuntimeRepository, RuntimeStatus};
use serde::Serialize;
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::Duration,
};
use tauri::{
    menu::{MenuBuilder, SubmenuBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, PhysicalPosition, State, WindowEvent,
};
use tauri_plugin_autostart::ManagerExt as AutostartExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tauri_plugin_updater::UpdaterExt;
use worker::WorkerClient;

const ESCAPE: &str = "escape";
const OFFICIAL_UPDATE_ENDPOINT: &str =
    "https://github.com/qixiaoyu27/Rain-VibeType/releases/latest/download/latest.json";
const OFFICIAL_MODEL_MANIFEST_ENDPOINT: &str =
    "https://github.com/qixiaoyu27/Rain-VibeType/releases/latest/download/models.json";
const OFFICIAL_RUNTIME_MANIFEST_ENDPOINT: &str =
    "https://github.com/qixiaoyu27/Rain-VibeType/releases/latest/download/runtime-manifest.json";
const TEXT_MODEL_ID: &str = "qwen3-0-6b-text";
const PREVIEW_MODEL_ID: &str = "streaming-zipformer-preview";
const TEXT_RUNTIME_COMPONENT_ID: &str = "llama-text-cpu-win-x64";
const TEXT_RUNTIME_MANIFEST: &str = include_str!("../resources/text-runtime-manifest.json");

fn config_uses_english(config: &Config) -> bool {
    config.ui_language == "en"
        || (config.ui_language == "system" && platform_windows::system_prefers_english())
}

fn app_uses_english(app: &AppHandle) -> bool {
    app.state::<AppState>()
        .config
        .lock()
        .map(|config| config_uses_english(&config))
        .unwrap_or(false)
}

fn text<'a>(english: bool, chinese: &'a str, english_text: &'a str) -> &'a str {
    if english {
        english_text
    } else {
        chinese
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Idle,
    Recording,
    WaitingForModel,
    Transcribing,
    Injecting,
}

fn can_start_recording(phase: Phase) -> bool {
    phase == Phase::Idle
}

fn can_cancel(phase: Phase) -> bool {
    matches!(
        phase,
        Phase::Recording | Phase::WaitingForModel | Phase::Transcribing
    )
}

fn accepts_transcription(runtime: &Runtime, request_id: &str) -> bool {
    runtime.phase == Phase::Transcribing && runtime.request_id.as_deref() == Some(request_id)
}

fn unload_timer_is_current(idle: bool, scheduled_epoch: u64, current_epoch: u64) -> bool {
    idle && scheduled_epoch == current_epoch
}

struct Runtime {
    phase: Phase,
    request_id: Option<String>,
    recording: Option<audio::Recording>,
    system_audio_ducker: Option<platform_windows::SystemAudioDucker>,
    target: Option<InputTarget>,
    pending_text: Option<String>,
    model_load_error: Option<String>,
    preview_text: String,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            request_id: None,
            recording: None,
            system_audio_ducker: None,
            target: None,
            pending_text: None,
            model_load_error: None,
            preview_text: String::new(),
        }
    }
}

struct ActiveDownload {
    model_id: String,
    paused: Arc<AtomicBool>,
}

struct AppState {
    config: Mutex<Config>,
    config_path: PathBuf,
    default_model_root: PathBuf,
    runtime_root: PathBuf,
    text_runtime_root: PathBuf,
    runtime: Mutex<Runtime>,
    worker: Arc<Mutex<WorkerClient>>,
    preview_worker: Arc<Mutex<WorkerClient>>,
    text_polisher: Arc<Mutex<text_polish::TextPolisher>>,
    diagnostics: diagnostics::Diagnostics,
    system_status: Mutex<SystemStatus>,
    active_download: Mutex<Option<ActiveDownload>>,
    shortcut_paused: AtomicBool,
    overlay_visible: AtomicBool,
    overlay_epoch: AtomicU64,
    unload_epoch: AtomicU64,
    escape_shortcut: mpsc::Sender<bool>,
}

#[derive(Clone, Serialize)]
struct OverlayStatus<'a> {
    state: &'a str,
    title: &'a str,
    detail: String,
    level: f32,
    opacity: f64,
}

#[derive(Clone, Serialize)]
struct UpdateInfo {
    available: bool,
    current_version: String,
    version: Option<String>,
    notes: Option<String>,
    published_at: Option<String>,
}

#[derive(Clone, Serialize)]
struct ModelUpdateInfo {
    changed: bool,
    manifest_version: String,
    models: Vec<ModelCard>,
}

#[derive(Clone, Serialize)]
struct TextPolishStatus {
    enabled: bool,
    ready: bool,
    model: ModelCard,
    runtime: RuntimeStatus,
}

#[derive(Clone, Default, Serialize)]
struct SystemStatus {
    shortcut_ready: bool,
    shortcut_error: Option<String>,
    autostart_ready: bool,
    autostart_error: Option<String>,
}

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> Result<Config, String> {
    state
        .config
        .lock()
        .map(|value| value.clone())
        .map_err(|_| "配置状态损坏".into())
}

#[tauri::command]
fn get_system_status(state: State<'_, AppState>) -> Result<SystemStatus, String> {
    state
        .system_status
        .lock()
        .map(|value| value.clone())
        .map_err(|_| "系统状态损坏".into())
}

#[tauri::command]
fn save_config(
    config: Config,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Config, String> {
    if state.runtime.lock().map_err(|_| "运行状态损坏")?.phase != Phase::Idle {
        return Err("录音或识别期间不能修改设置".into());
    }
    config.validate()?;
    let previous = state.config.lock().map_err(|_| "配置状态损坏")?.clone();
    let shortcut_needs_registration = config.hotkey != previous.hotkey
        || !state
            .system_status
            .lock()
            .map_err(|_| "系统状态损坏")?
            .shortcut_ready;
    if shortcut_needs_registration {
        if let Err(error) = app.global_shortcut().register(config.hotkey.as_str()) {
            let message = format!("快捷键注册失败，已保留原快捷键：{error}");
            if let Ok(mut status) = state.system_status.lock() {
                status.shortcut_ready = false;
                status.shortcut_error = Some(message.clone());
            }
            return Err(message);
        }
    }
    let autostart_needs_update = config.autostart != previous.autostart
        || !state
            .system_status
            .lock()
            .map_err(|_| "系统状态损坏")?
            .autostart_ready;
    let autostart = app.autolaunch();
    if autostart_needs_update {
        let result = if config.autostart {
            autostart.enable()
        } else {
            autostart.disable()
        };
        if let Err(error) = result {
            if shortcut_needs_registration {
                let _ = app.global_shortcut().unregister(config.hotkey.as_str());
            }
            if let Ok(mut status) = state.system_status.lock() {
                status.autostart_ready = false;
                status.autostart_error = Some(error.to_string());
            }
            return Err(format!("无法更新开机启动设置：{error}"));
        }
    }
    if let Err(error) = config::save(&state.config_path, &config) {
        if autostart_needs_update {
            let _ = if previous.autostart {
                autostart.enable()
            } else {
                autostart.disable()
            };
        }
        if shortcut_needs_registration {
            let _ = app.global_shortcut().unregister(config.hotkey.as_str());
        }
        return Err(error);
    }
    if config.hotkey != previous.hotkey {
        let _ = app.global_shortcut().unregister(previous.hotkey.as_str());
    }
    if let Ok(mut status) = state.system_status.lock() {
        status.shortcut_ready = true;
        status.shortcut_error = None;
        status.autostart_ready = true;
        status.autostart_error = None;
    }
    let model_changed = config.model_path != previous.model_path
        || config.device_preference != previous.device_preference
        || config.selected_model_id != previous.selected_model_id;
    let tray_changed = config.ui_language != previous.ui_language
        || config.selected_model_id != previous.selected_model_id;
    let stop_text_polisher = (!config.text_polish_enabled && previous.text_polish_enabled)
        || config.text_polish_idle_timeout_seconds != previous.text_polish_idle_timeout_seconds;
    *state.config.lock().map_err(|_| "配置状态损坏")? = config.clone();
    configure_worker_runtime(&state, &config)?;
    if model_changed {
        reload_selected_model(&app);
    }
    if tray_changed {
        let _ = setup_tray(&app);
    }
    if stop_text_polisher {
        let polisher = state.text_polisher.clone();
        thread::spawn(move || {
            if let Ok(mut polisher) = polisher.lock() {
                polisher.stop();
            }
        });
    }
    Ok(config)
}

#[tauri::command]
fn list_input_devices() -> Result<Vec<String>, String> {
    audio::input_devices()
}

#[tauri::command]
async fn test_input_level(state: State<'_, AppState>) -> Result<f32, String> {
    let device = state
        .config
        .lock()
        .map_err(|_| "配置状态损坏")?
        .input_device
        .clone();
    tauri::async_runtime::spawn_blocking(move || audio::measure_input_level(device.as_deref()))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn check_worker(state: State<'_, AppState>) -> Result<String, String> {
    let config = state.config.lock().map_err(|_| "配置状态损坏")?.clone();
    let status = configure_worker_runtime(&state, &config)?;
    if !status.ready {
        return Err("RUNTIME_NOT_INSTALLED：请先安装推荐的本地推理组件".into());
    }
    let python_path = config.python_path;
    let worker = state.worker.clone();
    tauri::async_runtime::spawn_blocking(move || {
        worker
            .lock()
            .map_err(|_| "Worker 状态损坏".to_string())?
            .check(&python_path)
    })
    .await
    .map_err(|error| error.to_string())?
}

fn runtime_manifest_endpoint() -> &'static str {
    option_env!("RAIN_RUNTIME_MANIFEST_ENDPOINT")
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(OFFICIAL_RUNTIME_MANIFEST_ENDPOINT)
}

fn runtime_repository(state: &AppState) -> Result<RuntimeRepository, String> {
    RuntimeRepository::new(state.runtime_root.clone())
}

fn text_runtime_repository(state: &AppState) -> Result<RuntimeRepository, String> {
    RuntimeRepository::new_with_embedded(state.text_runtime_root.clone(), TEXT_RUNTIME_MANIFEST)
}

#[derive(Clone, Debug)]
struct ResolvedRuntimeDependency {
    repository: String,
    component_id: String,
    preference: String,
}

fn resolve_runtime_dependency(
    model: &ModelDefinition,
    device_preference: &str,
) -> Result<ResolvedRuntimeDependency, String> {
    let preference = if model.runtime.repository == "text" {
        "cpu".to_owned()
    } else {
        device_preference.to_owned()
    };
    let accelerator = runtimes::selected_accelerator(&preference);
    let component_id = model.runtime.component_for(&accelerator).ok_or_else(|| {
        format!(
            "RUNTIME_NOT_MAPPED：模型 {} 缺少适用的推理框架映射",
            model.id
        )
    })?;
    Ok(ResolvedRuntimeDependency {
        repository: model.runtime.repository.clone(),
        component_id: component_id.to_owned(),
        preference,
    })
}

fn mapped_runtime_repository(
    repository: &str,
    speech_root: PathBuf,
    text_root: PathBuf,
) -> Result<RuntimeRepository, String> {
    match repository {
        "speech" => RuntimeRepository::new(speech_root),
        "text" => RuntimeRepository::new_with_embedded(text_root, TEXT_RUNTIME_MANIFEST),
        _ => Err(format!(
            "RUNTIME_NOT_MAPPED：不支持的推理框架仓库 {repository}"
        )),
    }
}

fn cleanup_unreferenced_runtimes(
    state: &AppState,
    repository: &ModelRepository,
    deleted_model: &ModelDefinition,
) -> Result<(), String> {
    let component_ids = deleted_model
        .runtime
        .components
        .values()
        .cloned()
        .collect::<HashSet<_>>();
    for component_id in component_ids {
        if repository.models_using_runtime(&component_id).is_empty() {
            mapped_runtime_repository(
                &deleted_model.runtime.repository,
                state.runtime_root.clone(),
                state.text_runtime_root.clone(),
            )?
            .remove(&component_id)?;
        }
    }
    Ok(())
}

fn managed_speech_model_cards(repository: &ModelRepository) -> Vec<ModelCard> {
    repository
        .list()
        .into_iter()
        .filter(|model| matches!(model.definition.purpose.as_str(), "asr" | "asr_preview"))
        .collect()
}

fn text_polish_status(state: &AppState) -> Result<TextPolishStatus, String> {
    let enabled = state
        .config
        .lock()
        .map_err(|_| "配置状态损坏")?
        .text_polish_enabled;
    let model = model_repository(state)?
        .list()
        .into_iter()
        .find(|model| model.definition.id == TEXT_MODEL_ID)
        .ok_or("TEXT_POLISH_MODEL_NOT_CONFIGURED：缺少文本整理模型定义")?;
    let dependency = resolve_runtime_dependency(&model.definition, "cpu")?;
    let runtime = text_runtime_repository(state)?.status_for_component(
        "cpu",
        "",
        Some(&dependency.component_id),
        false,
    );
    let model_ready = matches!(
        model.state.as_str(),
        "installed" | "custom" | "update_available"
    );
    Ok(TextPolishStatus {
        enabled,
        ready: model_ready && runtime.ready,
        model,
        runtime,
    })
}

fn configure_worker_runtime(state: &AppState, config: &Config) -> Result<RuntimeStatus, String> {
    let repository = runtime_repository(state)?;
    let model_id = if config.selected_model_id.is_empty() {
        "sensevoice-small"
    } else {
        &config.selected_model_id
    };
    let model = model_repository(state)?.definition(model_id)?.clone();
    let dependency = resolve_runtime_dependency(&model, &config.device_preference)?;
    if dependency.repository != "speech" {
        return Err("当前模型不是语音识别模型".into());
    }
    let allow_python_fallback = !prefers_native_sensevoice(config, &model.adapter_type);
    let status = repository.status_for_component(
        &config.device_preference,
        &config.python_path,
        Some(&dependency.component_id),
        allow_python_fallback,
    );
    let executable = status
        .active_executable
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| state.runtime_root.join("runtime-not-installed.exe"));
    state
        .worker
        .lock()
        .map_err(|_| "Worker 状态损坏")?
        .set_bundled_worker(executable);
    Ok(status)
}

#[tauri::command]
fn get_runtime_status(state: State<'_, AppState>) -> Result<RuntimeStatus, String> {
    let config = state.config.lock().map_err(|_| "配置状态损坏")?.clone();
    configure_worker_runtime(&state, &config)
}

#[tauri::command]
async fn refresh_runtime_status(state: State<'_, AppState>) -> Result<RuntimeStatus, String> {
    let endpoint = runtime_manifest_endpoint();
    let root = state.runtime_root.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut repository = RuntimeRepository::new(root)?;
        repository.refresh_manifest(endpoint)?;
        Ok::<(), String>(())
    })
    .await
    .map_err(|error| error.to_string())??;
    get_runtime_status(state)
}

#[tauri::command]
fn get_text_polish_status(state: State<'_, AppState>) -> Result<TextPolishStatus, String> {
    text_polish_status(&state)
}

#[tauri::command]
fn delete_text_model(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<TextPolishStatus, String> {
    if state.runtime.lock().map_err(|_| "运行状态损坏")?.phase != Phase::Idle {
        return Err("录音或识别期间不能删除文本整理模型".into());
    }
    state
        .text_polisher
        .lock()
        .map_err(|_| "文本整理状态损坏")?
        .stop();
    let repository = model_repository(&state)?;
    let deleted_model = repository.definition(TEXT_MODEL_ID)?.clone();
    repository.delete(TEXT_MODEL_ID)?;
    cleanup_unreferenced_runtimes(&state, &repository, &deleted_model)?;
    let _ = app.emit("text-polish-changed", ());
    let _ = app.emit("runtime-changed", ());
    text_polish_status(&state)
}

#[tauri::command]
fn delete_text_runtime(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<TextPolishStatus, String> {
    if state.runtime.lock().map_err(|_| "运行状态损坏")?.phase != Phase::Idle {
        return Err("录音或识别期间不能删除文本整理推理框架".into());
    }
    let models = model_repository(&state)?.models_using_runtime(TEXT_RUNTIME_COMPONENT_ID);
    if !models.is_empty() {
        return Err(format!(
            "RUNTIME_IN_USE：推理框架正被已安装模型使用：{}",
            models.join("、")
        ));
    }
    state
        .text_polisher
        .lock()
        .map_err(|_| "文本整理状态损坏")?
        .stop();
    text_runtime_repository(&state)?.remove(TEXT_RUNTIME_COMPONENT_ID)?;
    let _ = app.emit("text-polish-changed", ());
    text_polish_status(&state)
}

#[tauri::command]
fn list_models(state: State<'_, AppState>) -> Result<Vec<ModelCard>, String> {
    Ok(managed_speech_model_cards(&model_repository(&state)?))
}

#[tauri::command]
async fn check_model_updates(state: State<'_, AppState>) -> Result<ModelUpdateInfo, String> {
    let endpoint = option_env!("RAIN_MODEL_MANIFEST_ENDPOINT")
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(OFFICIAL_MODEL_MANIFEST_ENDPOINT);
    let repository = model_repository(&state)?;
    let root = repository.root().to_owned();
    let changed =
        tauri::async_runtime::spawn_blocking(move || repository.refresh_manifest(endpoint))
            .await
            .map_err(|error| error.to_string())??;
    let repository = ModelRepository::new(root)?;
    Ok(ModelUpdateInfo {
        changed,
        manifest_version: repository.manifest_version().to_owned(),
        models: managed_speech_model_cards(&repository),
    })
}

fn download_model_with_runtime(
    repository: ModelRepository,
    model_id: &str,
    paused: &AtomicBool,
    speech_root: PathBuf,
    text_root: PathBuf,
    device_preference: &str,
    app: &AppHandle,
) -> Result<PathBuf, String> {
    let model = repository.definition(model_id)?.clone();
    let dependency = resolve_runtime_dependency(&model, device_preference)?;
    let mut runtime_repository =
        mapped_runtime_repository(&dependency.repository, speech_root, text_root)?;
    let mut runtime_total = 0;
    if !runtime_repository.is_installed(&dependency.component_id) {
        if dependency.repository == "speech"
            && runtime_repository
                .component(&dependency.component_id)
                .is_none()
        {
            runtime_repository.refresh_manifest(runtime_manifest_endpoint())?;
        }
        runtime_total = runtime_repository
            .component(&dependency.component_id)
            .ok_or_else(|| {
                format!(
                    "RUNTIME_NOT_FOUND：推理框架 {} 尚未在官方清单中发布",
                    dependency.component_id
                )
            })?
            .archive_size;
        let model_total = model.download_size;
        runtime_repository.download(
            Some(&dependency.component_id),
            &dependency.preference,
            dependency.component_id == runtimes::NATIVE_SENSEVOICE_COMPONENT,
            |progress| {
                let _ = app.emit(
                    "model-download-progress",
                    DownloadProgress {
                        model_id: model.id.clone(),
                        downloaded: progress.downloaded,
                        total: runtime_total.saturating_add(model_total),
                        file: format!("runtime:{}", progress.component_id),
                    },
                );
            },
        )?;
    }
    repository.download(model_id, paused, |progress| {
        let _ = app.emit(
            "model-download-progress",
            DownloadProgress {
                model_id: progress.model_id,
                downloaded: runtime_total.saturating_add(progress.downloaded),
                total: runtime_total.saturating_add(progress.total),
                file: progress.file,
            },
        );
    })
}

#[tauri::command]
async fn download_model(
    model_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<ModelCard>, String> {
    let repository = model_repository(&state)?;
    let purpose = repository.definition(&model_id)?.purpose.clone();
    let is_asr_model = purpose == "asr";
    let is_text_model = purpose == "text_polish";
    if is_text_model {
        state
            .text_polisher
            .lock()
            .map_err(|_| "文本整理状态损坏")?
            .stop();
    }
    let root = repository.root().to_owned();
    let runtime_root = state.runtime_root.clone();
    let text_runtime_root = state.text_runtime_root.clone();
    let device_preference = state
        .config
        .lock()
        .map_err(|_| "配置状态损坏")?
        .device_preference
        .clone();
    let paused = Arc::new(AtomicBool::new(false));
    {
        let mut active = state.active_download.lock().map_err(|_| "下载状态损坏")?;
        if active.is_some() {
            return Err("已有模型正在下载".into());
        }
        *active = Some(ActiveDownload {
            model_id: model_id.clone(),
            paused: paused.clone(),
        });
    }
    let app_for_progress = app.clone();
    let download_id = model_id.clone();
    let joined = tauri::async_runtime::spawn_blocking(move || {
        download_model_with_runtime(
            repository,
            &download_id,
            &paused,
            runtime_root,
            text_runtime_root,
            &device_preference,
            &app_for_progress,
        )
    })
    .await;
    state
        .active_download
        .lock()
        .map_err(|_| "下载状态损坏")?
        .take();
    let result = joined.map_err(|error| error.to_string())?;
    let path = result?;

    let mut selected_after_download = false;
    if is_asr_model && state.runtime.lock().map_err(|_| "运行状态损坏")?.phase == Phase::Idle
    {
        let mut config = state.config.lock().map_err(|_| "配置状态损坏")?;
        if config.selected_model_id.is_empty() || config.selected_model_id == model_id {
            config.selected_model_id = model_id;
            config.model_path = path.to_string_lossy().into_owned();
            config::save(&state.config_path, &config)?;
            selected_after_download = true;
        }
    }
    if selected_after_download {
        reload_selected_model(&app);
    }
    let _ = app.emit("models-changed", ());
    let _ = app.emit("runtime-changed", ());
    if is_text_model {
        let _ = app.emit("text-polish-changed", ());
    }
    let _ = setup_tray(&app);
    Ok(managed_speech_model_cards(&ModelRepository::new(root)?))
}

#[tauri::command]
fn pause_model_download(model_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let active = state.active_download.lock().map_err(|_| "下载状态损坏")?;
    let active = active.as_ref().ok_or("没有正在下载的模型")?;
    if active.model_id != model_id {
        return Err("指定模型没有正在下载".into());
    }
    active.paused.store(true, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
async fn verify_model(model_id: String, state: State<'_, AppState>) -> Result<String, String> {
    let repository = model_repository(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        repository
            .verify(&model_id)
            .map(|path| path.to_string_lossy().into_owned())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn import_model(
    model_id: String,
    source_path: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ImportResult, String> {
    if state.runtime.lock().map_err(|_| "运行状态损坏")?.phase != Phase::Idle {
        return Err("录音、加载或识别期间不能导入模型".into());
    }
    let repository = model_repository(&state)?;
    if repository.definition(&model_id)?.purpose != "asr" {
        return Err("文本整理模型只支持官方校验下载".into());
    }
    let result = tauri::async_runtime::spawn_blocking(move || {
        repository.import(&model_id, std::path::Path::new(&source_path))
    })
    .await
    .map_err(|error| error.to_string())??;
    let mut config = state.config.lock().map_err(|_| "配置状态损坏")?;
    config.selected_model_id = result.model_id.clone();
    config.model_path = result.model_path.clone();
    config::save(&state.config_path, &config)?;
    drop(config);
    reload_selected_model(&app);
    let _ = app.emit("models-changed", ());
    let _ = setup_tray(&app);
    preload_current_model(&app);
    Ok(result)
}

#[tauri::command]
fn select_model(
    model_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Config, String> {
    select_model_internal(&app, &state, &model_id)
}

fn select_model_internal(
    app: &AppHandle,
    state: &AppState,
    model_id: &str,
) -> Result<Config, String> {
    if state.runtime.lock().map_err(|_| "运行状态损坏")?.phase != Phase::Idle {
        return Err("录音、加载或识别期间不能切换模型".into());
    }
    let repository = model_repository(state)?;
    if repository.definition(model_id)?.purpose != "asr" {
        return Err("该模型不是语音识别模型".into());
    }
    let path = repository.installed_path(model_id)?;
    let mut config = state.config.lock().map_err(|_| "配置状态损坏")?;
    config.selected_model_id = model_id.to_owned();
    config.model_path = path.to_string_lossy().into_owned();
    config::save(&state.config_path, &config)?;
    let saved = config.clone();
    drop(config);
    reload_selected_model(app);
    let _ = app.emit("models-changed", ());
    let _ = app.emit("runtime-changed", ());
    let _ = setup_tray(app);
    preload_current_model(app);
    Ok(saved)
}

#[tauri::command]
fn delete_model(
    model_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<ModelCard>, String> {
    if state.runtime.lock().map_err(|_| "运行状态损坏")?.phase != Phase::Idle {
        return Err("录音、加载或识别期间不能删除模型".into());
    }
    let repository = model_repository(&state)?;
    let deleted_model = repository.definition(&model_id)?.clone();
    let is_text_model = deleted_model.purpose == "text_polish";
    match deleted_model.purpose.as_str() {
        "text_polish" => state
            .text_polisher
            .lock()
            .map_err(|_| "文本整理状态损坏")?
            .stop(),
        "asr_preview" => {
            let mut worker = state
                .preview_worker
                .lock()
                .map_err(|_| "实时预览 Worker 状态损坏")?;
            let _ = worker.unload();
            worker.shutdown();
        }
        "asr" => {
            let mut worker = state.worker.lock().map_err(|_| "Worker 状态损坏")?;
            let _ = worker.unload();
            worker.shutdown();
        }
        _ => {}
    }
    let selected_config = state.config.lock().map_err(|_| "配置状态损坏")?.clone();
    let selected_path = (selected_config.selected_model_id == model_id)
        .then(|| PathBuf::from(selected_config.model_path));
    if let Some(path) = selected_path.as_deref() {
        repository.delete_managed_path(path)?;
    }
    repository.delete(&model_id)?;
    let cleanup_result = cleanup_unreferenced_runtimes(&state, &repository, &deleted_model);
    let mut config = state.config.lock().map_err(|_| "配置状态损坏")?;
    if config.selected_model_id == model_id {
        config.selected_model_id.clear();
        config.model_path.clear();
        config::save(&state.config_path, &config)?;
    }
    drop(config);
    let _ = app.emit("models-changed", ());
    let _ = app.emit("runtime-changed", ());
    if is_text_model {
        let _ = app.emit("text-polish-changed", ());
    }
    let _ = setup_tray(&app);
    cleanup_result?;
    Ok(managed_speech_model_cards(&repository))
}

#[tauri::command]
fn delete_old_model_versions(
    model_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ModelCard>, String> {
    if state.runtime.lock().map_err(|_| "运行状态损坏")?.phase != Phase::Idle {
        return Err("录音、加载或识别期间不能删除旧模型".into());
    }
    let repository = model_repository(&state)?;
    repository.delete_previous_versions(&model_id)?;
    Ok(managed_speech_model_cards(&repository))
}

#[tauri::command]
fn cancel_current(app: AppHandle) -> Result<(), String> {
    cancel(&app)
}

#[tauri::command]
fn get_pending_text(state: State<'_, AppState>) -> Result<Option<String>, String> {
    Ok(state
        .runtime
        .lock()
        .map_err(|_| "运行状态损坏")?
        .pending_text
        .clone())
}

#[tauri::command]
fn copy_pending_text(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let text = state
        .runtime
        .lock()
        .map_err(|_| "运行状态损坏")?
        .pending_text
        .clone()
        .ok_or("没有待恢复的文字")?;
    platform_windows::copy_text(&text)?;
    let mut runtime = state.runtime.lock().map_err(|_| "运行状态损坏")?;
    if runtime.pending_text.as_deref() == Some(&text) {
        runtime.pending_text = None;
    }
    let _ = app.emit("pending-text", Option::<String>::None);
    Ok(())
}

#[tauri::command]
fn diagnostic_files() -> Vec<&'static str> {
    diagnostics::Diagnostics::file_list()
}

#[tauri::command]
fn export_diagnostics(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let config = state.config.lock().map_err(|_| "配置状态损坏")?.clone();
    state
        .diagnostics
        .export(std::path::Path::new(&path), &config)
}

#[tauri::command]
fn open_log_directory(state: State<'_, AppState>) -> Result<(), String> {
    std::fs::create_dir_all(state.diagnostics.log_dir()).map_err(|error| error.to_string())?;
    std::process::Command::new("explorer.exe")
        .arg(state.diagnostics.log_dir())
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("无法打开日志目录：{error}"))
}

#[tauri::command]
fn pending_crash_report(state: State<'_, AppState>) -> Result<Option<serde_json::Value>, String> {
    let config = state.config.lock().map_err(|_| "配置状态损坏")?.clone();
    Ok(state.diagnostics.pending_crash_report(&config))
}

#[tauri::command]
async fn submit_crash_report(state: State<'_, AppState>) -> Result<(), String> {
    let endpoint = option_env!("RAIN_CRASH_REPORT_ENDPOINT")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or("CRASH_REPORT_NOT_CONFIGURED：发布构建未配置匿名崩溃报告地址")?
        .to_owned();
    let config = state.config.lock().map_err(|_| "配置状态损坏")?.clone();
    let report = state
        .diagnostics
        .pending_crash_report(&config)
        .ok_or("没有待提交的崩溃报告")?;
    tauri::async_runtime::spawn_blocking(move || {
        let response = reqwest::blocking::Client::new()
            .post(endpoint)
            .json(&report)
            .send()
            .map_err(|error| format!("CRASH_REPORT_FAILED：{error}"))?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "CRASH_REPORT_FAILED：服务器返回 {}",
                response.status()
            ))
        }
    })
    .await
    .map_err(|error| error.to_string())??;
    state.diagnostics.clear_crash_report()
}

#[tauri::command]
fn dismiss_crash_report(state: State<'_, AppState>) -> Result<(), String> {
    state.diagnostics.clear_crash_report()
}

fn update_configuration() -> Result<(reqwest::Url, String), String> {
    let endpoint = option_env!("RAIN_UPDATE_ENDPOINT")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(OFFICIAL_UPDATE_ENDPOINT);
    let public_key = option_env!("RAIN_UPDATE_PUBLIC_KEY")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or("UPDATE_NOT_CONFIGURED：发布构建未配置更新签名公钥")?;
    let endpoint = endpoint
        .parse::<reqwest::Url>()
        .map_err(|error| format!("更新清单地址无效：{error}"))?;
    Ok((endpoint, public_key.to_owned()))
}

async fn query_update(app: &AppHandle) -> Result<UpdateInfo, String> {
    let (endpoint, public_key) = update_configuration()?;
    let updater = app
        .updater_builder()
        .endpoints(vec![endpoint])
        .map_err(|error| format!("无法配置更新地址：{error}"))?
        .pubkey(public_key)
        .build()
        .map_err(|error| format!("无法初始化更新器：{error}"))?;
    let current_version = app.package_info().version.to_string();
    let update = updater
        .check()
        .await
        .map_err(|error| format!("UPDATE_CHECK_FAILED：{error}"))?;
    Ok(match update {
        Some(update) => UpdateInfo {
            available: true,
            current_version,
            version: Some(update.version.clone()),
            notes: update.body.clone(),
            published_at: update.date.map(|date| date.to_string()),
        },
        None => UpdateInfo {
            available: false,
            current_version,
            version: None,
            notes: None,
            published_at: None,
        },
    })
}

#[tauri::command]
async fn check_update(app: AppHandle) -> Result<UpdateInfo, String> {
    query_update(&app).await
}

#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    let (endpoint, public_key) = update_configuration()?;
    let updater = app
        .updater_builder()
        .endpoints(vec![endpoint])
        .map_err(|error| format!("无法配置更新地址：{error}"))?
        .pubkey(public_key)
        .build()
        .map_err(|error| format!("无法初始化更新器：{error}"))?;
    let update = updater
        .check()
        .await
        .map_err(|error| format!("UPDATE_CHECK_FAILED：{error}"))?
        .ok_or("当前已经是最新版本")?;
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|error| format!("UPDATE_INSTALL_FAILED：{error}"))?;
    app.restart();
}

fn start_recording(app: &AppHandle) -> Result<(), String> {
    // Capture the user's foreground application before any runtime checks can
    // launch helper processes or touch the window manager.
    let target = InputTarget::capture();
    let state = app.state::<AppState>();
    state.unload_epoch.fetch_add(1, Ordering::Relaxed);
    {
        let runtime = state.runtime.lock().map_err(|_| "运行状态损坏")?;
        if !can_start_recording(runtime.phase) {
            let english = app_uses_english(app);
            ephemeral(
                app,
                "transcribing",
                text(english, "正在识别，请稍候", "Recognition in progress"),
                text(
                    english,
                    "当前任务完成后即可继续",
                    "You can record again when the current task finishes",
                ),
            );
            return Err("BUSY".into());
        }
        if runtime.pending_text.is_some() {
            show_main(app);
            return Err("请先复制上次未能写入剪贴板的识别结果".into());
        }
    }
    let config = state.config.lock().map_err(|_| "配置状态损坏")?.clone();
    let runtime_status = configure_worker_runtime(&state, &config)?;
    if !runtime_status.ready {
        show_main(app);
        let _ = app.emit("navigate", "models");
        ephemeral(
            app,
            "failed",
            text(
                config_uses_english(&config),
                "尚未安装推理组件",
                "Inference component not installed",
            ),
            text(
                config_uses_english(&config),
                "请先在模型管理页下载推荐组件",
                "Download the recommended component in Model management",
            ),
        );
        return Err("RUNTIME_NOT_INSTALLED：请先安装推荐的本地推理组件".into());
    }
    if config.model_path.trim().is_empty() || !std::path::Path::new(&config.model_path).is_dir() {
        let english = config_uses_english(&config);
        show_main(app);
        let _ = app.emit("navigate", "model");
        ephemeral(
            app,
            "failed",
            text(english, "尚未安装语音模型", "No voice model installed"),
            text(
                english,
                "请在模型管理页下载或导入模型",
                "Download or import a model in Model management",
            ),
        );
        return Err("MODEL_NOT_INSTALLED".into());
    }
    if let Err(error) = model_repository(&state)?.validate_loadable(
        if config.selected_model_id.is_empty() {
            "sensevoice-small"
        } else {
            &config.selected_model_id
        },
        std::path::Path::new(&config.model_path),
    ) {
        state.diagnostics.record(
            "MODEL_INTEGRITY_FAILED",
            &config.selected_model_id,
            None,
            None,
        );
        show_main(app);
        let _ = app.emit("navigate", "models");
        ephemeral(
            app,
            "failed",
            text(
                config_uses_english(&config),
                "模型损坏或未完成",
                "Model is damaged or incomplete",
            ),
            &error,
        );
        return Err(error);
    }
    let adapter_type = current_adapter(&state, &config)?;

    let request_id = uuid::Uuid::new_v4().to_string();
    let recording = match audio::Recording::start(config.input_device.as_deref()) {
        Ok(recording) => recording,
        Err(error) => {
            let code =
                if error.to_ascii_lowercase().contains("permission") || error.contains("拒绝") {
                    "AUDIO_PERMISSION_DENIED"
                } else {
                    "AUDIO_DEVICE_DISCONNECTED"
                };
            state
                .diagnostics
                .record(code, &config.selected_model_id, None, None);
            if config.error_sound {
                platform_windows::play_sound("error");
            }
            ephemeral(
                app,
                "failed",
                text(
                    config_uses_english(&config),
                    "无法开始录音",
                    "Could not start recording",
                ),
                &error,
            );
            return Err(format!("{code}：{error}"));
        }
    };
    let system_audio_ducker = if config.duck_system_audio {
        match platform_windows::SystemAudioDucker::activate() {
            Ok(ducker) => Some(ducker),
            Err(error) => {
                state.diagnostics.record(
                    "SYSTEM_AUDIO_DUCK_FAILED",
                    &config.selected_model_id,
                    None,
                    None,
                );
                ephemeral(
                    app,
                    "failed",
                    text(
                        config_uses_english(&config),
                        "无法降低电脑声音",
                        "Could not lower PC audio",
                    ),
                    &error,
                );
                return Err(error);
            }
        }
    } else {
        None
    };
    {
        let mut runtime = state.runtime.lock().map_err(|_| "运行状态损坏")?;
        runtime.phase = Phase::Recording;
        runtime.request_id = Some(request_id.clone());
        runtime.target = target;
        runtime.recording = Some(recording);
        runtime.system_audio_ducker = system_audio_ducker;
        runtime.model_load_error = None;
        runtime.preview_text.clear();
    }

    let _ = state.escape_shortcut.send(true);
    if config.start_sound {
        platform_windows::play_sound("start");
    }
    show_overlay(
        app,
        target,
        "recording",
        text(config_uses_english(&config), "正在录音", "Recording"),
        "00:00",
        0.0,
    );
    spawn_recording_clock(
        app.clone(),
        request_id.clone(),
        config.max_recording_seconds,
    );
    spawn_stream_preview(app.clone(), request_id.clone());
    let worker = state.worker.clone();
    let app_handle = app.clone();
    let preload_request = uuid::Uuid::new_v4().to_string();
    thread::spawn(move || {
        let result = worker
            .lock()
            .map_err(|_| "Worker 状态损坏".to_string())
            .and_then(|mut worker| {
                worker.load_model(
                    &config.python_path,
                    &preload_request,
                    &config.model_path,
                    &adapter_type,
                    worker_device(&config, &adapter_type),
                )
            });
        if let Err(error) = result {
            let state = app_handle.state::<AppState>();
            if let Ok(mut runtime) = state.runtime.lock() {
                if runtime.request_id.as_deref() == Some(&request_id) {
                    runtime.model_load_error = Some(error);
                }
            };
        }
    });
    Ok(())
}

fn stop_recording(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let (request_id, target, recording, system_audio_ducker, config, adapter_type) = {
        let mut runtime = state.runtime.lock().map_err(|_| "运行状态损坏")?;
        if runtime.phase != Phase::Recording {
            return Ok(());
        }
        runtime.phase = Phase::WaitingForModel;
        let request_id = runtime.request_id.clone().ok_or("录音任务丢失")?;
        let target = runtime.target;
        let recording = runtime.recording.take().ok_or("录音缓冲丢失")?;
        let system_audio_ducker = runtime.system_audio_ducker.take();
        let config = state.config.lock().map_err(|_| "配置状态损坏")?.clone();
        let adapter_type = current_adapter(&state, &config)?;
        (
            request_id,
            target,
            recording,
            system_audio_ducker,
            config,
            adapter_type,
        )
    };
    drop(system_audio_ducker);

    let pcm = match recording.finish() {
        Ok(pcm) => pcm,
        Err(error) => {
            fail_task(app, &request_id, &error);
            return Err(error);
        }
    };
    if config.stop_sound {
        platform_windows::play_sound("stop");
    }
    show_overlay(
        app,
        target,
        "loading",
        text(
            config_uses_english(&config),
            "正在加载模型",
            "Loading model",
        ),
        text(
            config_uses_english(&config),
            "录音已安全保存在内存中",
            "The recording is held safely in memory",
        ),
        0.0,
    );
    let worker = state.worker.clone();
    let app_handle = app.clone();
    thread::spawn(move || {
        let mut result = worker
            .lock()
            .map_err(|_| "Worker 状态损坏".to_string())
            .and_then(|mut worker| {
                worker.load_model(
                    &config.python_path,
                    &request_id,
                    &config.model_path,
                    &adapter_type,
                    worker_device(&config, &adapter_type),
                )?;
                {
                    let state = app_handle.state::<AppState>();
                    let mut runtime = state.runtime.lock().map_err(|_| "运行状态损坏")?;
                    if runtime.phase != Phase::WaitingForModel
                        || runtime.request_id.as_deref() != Some(&request_id)
                    {
                        return Err("CANCELLED".into());
                    }
                    runtime.phase = Phase::Transcribing;
                }
                show_overlay(
                    &app_handle,
                    target,
                    "transcribing",
                    text(config_uses_english(&config), "正在识别", "Recognizing"),
                    text(
                        config_uses_english(&config),
                        "模型正在本机处理音频",
                        "The model is processing audio locally",
                    ),
                    0.0,
                );
                worker.transcribe_loaded(&request_id, pcm)
            });
        if matches!(&result, Err(error) if error == "CANCELLED") {
            return;
        }
        if config.text_polish_enabled {
            let request_is_current = app_handle
                .state::<AppState>()
                .runtime
                .lock()
                .map(|runtime| accepts_transcription(&runtime, &request_id))
                .unwrap_or(false);
            if request_is_current {
                if let Ok(transcription) = result.as_mut() {
                    show_overlay(
                        &app_handle,
                        target,
                        "transcribing",
                        text(
                            config_uses_english(&config),
                            "正在整理文字",
                            "Polishing text",
                        ),
                        text(
                            config_uses_english(&config),
                            "本地小模型正在校对标点和分段",
                            "The local text model is checking punctuation and paragraphs",
                        ),
                        0.0,
                    );
                    match polish_text(&app_handle, &config, &transcription.text) {
                        Ok(polished) => {
                            transcription.text = polished;
                            app_handle.state::<AppState>().diagnostics.record(
                                "TEXT_POLISH_COMPLETED",
                                TEXT_MODEL_ID,
                                None,
                                None,
                            );
                        }
                        Err(error) => {
                            let code = error.split('：').next().unwrap_or("TEXT_POLISH_FALLBACK");
                            app_handle.state::<AppState>().diagnostics.record(
                                code,
                                TEXT_MODEL_ID,
                                None,
                                None,
                            );
                        }
                    }
                }
            }
        }
        finish_transcription(&app_handle, &request_id, target, result);
    });
    Ok(())
}

fn polish_text(app: &AppHandle, config: &Config, raw: &str) -> Result<String, String> {
    let state = app.state::<AppState>();
    let repository = model_repository(&state)?;
    let definition = repository.definition(TEXT_MODEL_ID)?.clone();
    let model_root = repository.installed_path(TEXT_MODEL_ID)?;
    repository.validate_loadable(TEXT_MODEL_ID, &model_root)?;
    let model_file = definition
        .files
        .first()
        .map(|file| model_root.join(&file.path))
        .filter(|path| path.is_file())
        .ok_or("TEXT_POLISH_MODEL_MISSING：文本整理模型文件不存在")?;
    let dependency = resolve_runtime_dependency(&definition, "cpu")?;
    let runtime = text_runtime_repository(&state)?.status_for_component(
        "cpu",
        "",
        Some(&dependency.component_id),
        false,
    );
    let executable = runtime
        .active_executable
        .map(PathBuf::from)
        .ok_or("TEXT_POLISH_RUNTIME_MISSING：文本整理推理框架尚未安装")?;
    let mut polisher = state
        .text_polisher
        .lock()
        .map_err(|_| "TEXT_POLISH_STATE_FAILED：文本整理状态损坏".to_owned())?;
    polisher.polish(
        &executable,
        &model_file,
        raw,
        text_polish::TextPolishOptions {
            remove_fillers: config.text_polish_remove_fillers,
            paragraphs: config.text_polish_paragraphs,
            protected_terms: &config.text_polish_protected_terms,
            idle_timeout_seconds: config.text_polish_idle_timeout_seconds,
        },
    )
}

fn finish_transcription(
    app: &AppHandle,
    request_id: &str,
    target: Option<InputTarget>,
    result: Result<worker::Transcription, String>,
) {
    let state = app.state::<AppState>();
    {
        let Ok(mut runtime) = state.runtime.lock() else {
            return;
        };
        if !accepts_transcription(&runtime, request_id) {
            return;
        }
        runtime.phase = Phase::Injecting;
    }

    let transcription = match result {
        Ok(result) => result,
        Err(error) => {
            fail_task(app, request_id, &error);
            return;
        }
    };
    let model_id = state
        .config
        .lock()
        .map(|config| config.selected_model_id.clone())
        .unwrap_or_default();
    state.diagnostics.record(
        "TRANSCRIPTION_COMPLETED",
        &model_id,
        Some(transcription.duration_ms),
        Some(transcription.inference_ms),
    );
    let config = state
        .config
        .lock()
        .map(|value| value.clone())
        .unwrap_or_default();
    show_overlay(
        app,
        target,
        "injecting",
        text(config_uses_english(&config), "正在输入", "Entering text"),
        text(
            config_uses_english(&config),
            "重新确认原输入位置",
            "Revalidating the original input target",
        ),
        0.0,
    );

    let recognized_text = transcription.text;
    let injected = match target {
        Some(target) if target.is_still_active() => {
            Some(if config.injection_method == "clipboard" {
                platform_windows::paste_text(&recognized_text, config.restore_clipboard)
            } else {
                platform_windows::type_text(&recognized_text)
            })
        }
        _ => {
            state
                .diagnostics
                .record("INPUT_TARGET_CHANGED", &model_id, None, None);
            None
        }
    };

    match injected {
        Some(Ok(())) => complete_task(
            app,
            request_id,
            "completed",
            text(config_uses_english(&config), "已输入", "Entered"),
            &recognized_text,
        ),
        failed => {
            let failure_code = failed.and_then(Result::err).map(|error| {
                let code = if error.contains("CLIPBOARD_RESTORE_FAILED") {
                    "CLIPBOARD_RESTORE_FAILED"
                } else {
                    "INJECTION_FAILED"
                };
                state.diagnostics.record(code, &model_id, None, None);
                code
            });
            match platform_windows::copy_text(&recognized_text) {
                Ok(()) => {
                    let title = if failure_code == Some("CLIPBOARD_RESTORE_FAILED") {
                        text(
                            config_uses_english(&config),
                            "剪贴板恢复失败，文字已保留",
                            "Clipboard restore failed; text was preserved",
                        )
                    } else {
                        text(config_uses_english(&config), "已复制", "Copied")
                    };
                    complete_task(app, request_id, "completed", title, &recognized_text)
                }
                Err(error) => {
                    if let Ok(mut runtime) = state.runtime.lock() {
                        runtime.pending_text = Some(recognized_text.clone());
                    }
                    let _ = app.emit("pending-text", Some(recognized_text));
                    show_main(app);
                    fail_task(
                        app,
                        request_id,
                        &format!("CLIPBOARD_RESTORE_FAILED：{error}"),
                    );
                }
            }
        }
    }
}

fn cancel(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let request_id = {
        let mut runtime = state.runtime.lock().map_err(|_| "运行状态损坏")?;
        if !can_cancel(runtime.phase) {
            return Ok(());
        }
        let request_id = runtime.request_id.take().unwrap_or_default();
        runtime.recording = None;
        runtime.system_audio_ducker = None;
        runtime.target = None;
        runtime.model_load_error = None;
        runtime.phase = Phase::Idle;
        request_id
    };
    let _ = state.escape_shortcut.send(false);
    let english = app_uses_english(app);
    terminal_overlay(
        app,
        &request_id,
        "cancelled",
        text(english, "已取消", "Cancelled"),
        text(
            english,
            "没有修改输入框或剪贴板",
            "The input target and clipboard were not changed",
        ),
    );
    schedule_model_unload(app);
    Ok(())
}

fn complete_task(app: &AppHandle, request_id: &str, state_name: &str, title: &str, text: &str) {
    reset_task(app, request_id);
    terminal_overlay(app, request_id, state_name, title, &truncate(text, 52));
    schedule_model_unload(app);
}

fn fail_task(app: &AppHandle, request_id: &str, error: &str) {
    reset_task(app, request_id);
    let state = app.state::<AppState>();
    let config = state
        .config
        .lock()
        .map(|config| config.clone())
        .unwrap_or_default();
    state
        .diagnostics
        .record(error, &config.selected_model_id, None, None);
    if config.device_preference == "cuda"
        && (error.contains("DEVICE_OUT_OF_MEMORY") || error.to_ascii_lowercase().contains("cuda"))
    {
        show_main(app);
        let _ = app.emit("gpu-fallback-required", error.to_owned());
    }
    if config.error_sound {
        platform_windows::play_sound("error");
    }
    terminal_overlay(
        app,
        request_id,
        "failed",
        text(config_uses_english(&config), "处理失败", "Failed"),
        &truncate(error, 70),
    );
    schedule_model_unload(app);
}

fn reset_task(app: &AppHandle, request_id: &str) {
    let state = app.state::<AppState>();
    if let Ok(mut runtime) = state.runtime.lock() {
        if runtime.request_id.as_deref() == Some(request_id) {
            runtime.request_id = None;
            runtime.recording = None;
            runtime.system_audio_ducker = None;
            runtime.target = None;
            runtime.model_load_error = None;
            runtime.preview_text.clear();
            runtime.phase = Phase::Idle;
        }
    }
    let _ = state.escape_shortcut.send(false);
}

fn spawn_stream_preview(app: AppHandle, request_id: String) {
    thread::spawn(move || {
        let state = app.state::<AppState>();
        let repository = match model_repository(&state) {
            Ok(repository) => repository,
            Err(_) => return,
        };
        let model = match repository.definition(PREVIEW_MODEL_ID).cloned() {
            Ok(model) => model,
            Err(_) => return,
        };
        let model_path = match repository.installed_path(PREVIEW_MODEL_ID) {
            Ok(path)
                if repository
                    .validate_loadable(PREVIEW_MODEL_ID, &path)
                    .is_ok() =>
            {
                path
            }
            _ => return,
        };
        let dependency = match resolve_runtime_dependency(&model, "cpu") {
            Ok(dependency) => dependency,
            Err(_) => return,
        };
        let executable = match runtime_repository(&state)
            .map(|repository| {
                repository.status_for_component("cpu", "", Some(&dependency.component_id), false)
            })
            .ok()
            .and_then(|status| status.active_executable)
        {
            Some(executable) => PathBuf::from(executable),
            None => return,
        };
        let mut worker = match state.preview_worker.lock() {
            Ok(worker) => worker,
            Err(_) => return,
        };
        worker.set_bundled_worker(executable);
        if worker
            .load_model(
                "",
                &request_id,
                &model_path.to_string_lossy(),
                &model.adapter_type,
                "cpu",
            )
            .and_then(|_| worker.start_preview(&request_id))
            .is_err()
        {
            state
                .diagnostics
                .record("STREAM_PREVIEW_FAILED", PREVIEW_MODEL_ID, None, None);
            return;
        }

        let mut cursor = 0;
        loop {
            thread::sleep(Duration::from_millis(120));
            let chunk = {
                let Ok(runtime) = state.runtime.lock() else {
                    break;
                };
                if runtime.phase != Phase::Recording
                    || runtime.request_id.as_deref() != Some(&request_id)
                {
                    break;
                }
                runtime
                    .recording
                    .as_ref()
                    .map(|recording| recording.preview_chunk(&mut cursor))
            };
            let Some((sample_rate, pcm)) = chunk else {
                break;
            };
            if pcm.is_empty() {
                continue;
            }
            match worker.preview_audio(&request_id, sample_rate, pcm) {
                Ok(text) => {
                    if let Ok(mut runtime) = state.runtime.lock() {
                        if runtime.phase == Phase::Recording
                            && runtime.request_id.as_deref() == Some(&request_id)
                        {
                            runtime.preview_text = text;
                        }
                    }
                }
                Err(_) => {
                    state
                        .diagnostics
                        .record("STREAM_PREVIEW_FAILED", PREVIEW_MODEL_ID, None, None);
                    break;
                }
            }
        }
        let _ = worker.finish_preview(&request_id);
    });
}

fn recording_detail(elapsed: u64, maximum: u64, preview: &str, english: bool) -> String {
    let mut detail = if maximum.saturating_sub(elapsed) <= 10 {
        if english {
            format!(
                "{:02}:{:02} · {} seconds left",
                elapsed / 60,
                elapsed % 60,
                maximum - elapsed
            )
        } else {
            format!(
                "{:02}:{:02} · 剩余 {} 秒",
                elapsed / 60,
                elapsed % 60,
                maximum - elapsed
            )
        }
    } else {
        format!("{:02}:{:02}", elapsed / 60, elapsed % 60)
    };
    if !preview.is_empty() {
        detail.push_str(" · ");
        detail.push_str(&tail_text(preview, 24));
    }
    detail
}

fn spawn_recording_clock(app: AppHandle, request_id: String, maximum: u64) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(50));
        let state = app.state::<AppState>();
        let (elapsed, level, audio_error, preview) = {
            let Ok(runtime) = state.runtime.lock() else {
                return;
            };
            if runtime.phase != Phase::Recording
                || runtime.request_id.as_deref() != Some(&request_id)
            {
                return;
            }
            runtime
                .recording
                .as_ref()
                .map_or((0, 0.0, None, String::new()), |recording| {
                    (
                        recording.elapsed().as_secs(),
                        recording.level(),
                        recording.take_error(),
                        runtime.preview_text.clone(),
                    )
                })
        };
        if let Some(error) = audio_error {
            fail_task(
                &app,
                &request_id,
                &format!("AUDIO_DEVICE_DISCONNECTED：{error}"),
            );
            return;
        }
        if elapsed >= maximum {
            let _ = stop_recording(&app);
            return;
        }
        let english = app_uses_english(&app);
        let detail = recording_detail(elapsed, maximum, &preview, english);
        emit_overlay(
            &app,
            "recording",
            text(english, "正在录音", "Recording"),
            &detail,
            level,
        );
    });
}

fn show_overlay(
    app: &AppHandle,
    target: Option<InputTarget>,
    state_name: &str,
    title: &str,
    detail: &str,
    level: f32,
) {
    let config = app
        .state::<AppState>()
        .config
        .lock()
        .map(|config| config.clone())
        .unwrap_or_default();
    if !config.show_overlay
        || target
            .filter(|target| target.is_fullscreen())
            .is_some_and(|_| !config.show_overlay_fullscreen)
    {
        app.state::<AppState>()
            .overlay_visible
            .store(false, Ordering::Relaxed);
        if let Some(window) = app.get_webview_window("overlay") {
            if let Ok(hwnd) = window.hwnd() {
                platform_windows::hide_window(hwnd.0 as isize);
            }
        }
        if let Some(window) = app.get_webview_window("overlay-cancel") {
            if let Ok(hwnd) = window.hwnd() {
                platform_windows::hide_window(hwnd.0 as isize);
            }
        }
        return;
    }
    app.state::<AppState>()
        .overlay_visible
        .store(true, Ordering::Relaxed);
    if let Some(window) = app.get_webview_window("overlay") {
        if let Some((left, top, width, height)) = target.and_then(InputTarget::work_area) {
            let scale = window.scale_factor().unwrap_or(1.0);
            let (overlay_width, overlay_height) = window
                .outer_size()
                .map(|size| (size.width as i32, size.height as i32))
                .unwrap_or(((440.0 * scale) as i32, (72.0 * scale) as i32));
            let x = left + (width - overlay_width) / 2;
            let y = top + height - overlay_height - (20.0 * scale) as i32;
            let _ = window.set_position(PhysicalPosition::new(x, y));
            if let Some(cancel_window) = app.get_webview_window("overlay-cancel") {
                let (cancel_width, cancel_height) = cancel_window
                    .outer_size()
                    .map(|size| (size.width as i32, size.height as i32))
                    .unwrap_or(((77.0 * scale) as i32, (38.0 * scale) as i32));
                let cancel_x = x + (overlay_width - cancel_width - (13.0 * scale) as i32).max(0);
                let cancel_y = y + ((overlay_height - cancel_height) / 2).max(0);
                let _ = cancel_window.set_position(PhysicalPosition::new(cancel_x, cancel_y));
            }
        }
        let _ = window.set_ignore_cursor_events(true);
        if let Ok(hwnd) = window.hwnd() {
            let _ = platform_windows::show_without_activation(hwnd.0 as isize);
        }
    }
    if let Some(window) = app.get_webview_window("overlay-cancel") {
        if matches!(state_name, "recording" | "loading" | "transcribing") {
            if let Ok(hwnd) = window.hwnd() {
                let _ = platform_windows::show_without_activation(hwnd.0 as isize);
            }
        } else {
            if let Ok(hwnd) = window.hwnd() {
                platform_windows::hide_window(hwnd.0 as isize);
            }
        }
    }
    emit_overlay(app, state_name, title, detail, level);
}

fn emit_overlay(app: &AppHandle, state_name: &str, title: &str, detail: &str, level: f32) -> u64 {
    let state = app.state::<AppState>();
    let epoch = state.overlay_epoch.fetch_add(1, Ordering::Relaxed) + 1;
    let opacity = state
        .config
        .lock()
        .map(|config| config.overlay_opacity)
        .unwrap_or(0.68);
    let payload = OverlayStatus {
        state: state_name,
        title,
        detail: detail.to_owned(),
        level,
        opacity,
    };
    let _ = app.emit_to("overlay", "overlay-status", payload.clone());
    let _ = app.emit_to("overlay-cancel", "overlay-status", payload);
    let _ = app.emit_to(
        "main",
        "audio-level",
        serde_json::json!({ "state": state_name, "level": level }),
    );
    epoch
}

fn terminal_overlay(app: &AppHandle, _request_id: &str, state: &str, title: &str, detail: &str) {
    let config = app
        .state::<AppState>()
        .config
        .lock()
        .map(|config| config.clone())
        .unwrap_or_default();
    if !config.show_overlay {
        return;
    }
    let app_state = app.state::<AppState>();
    if !app_state.overlay_visible.load(Ordering::Relaxed) {
        return;
    }
    if let Some(window) = app.get_webview_window("overlay-cancel") {
        if let Ok(hwnd) = window.hwnd() {
            platform_windows::hide_window(hwnd.0 as isize);
        }
    }
    let epoch = emit_overlay(app, state, title, detail, 0.0);
    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(1700));
        let state = app.state::<AppState>();
        if state.overlay_epoch.load(Ordering::Relaxed) == epoch {
            if let Some(window) = app.get_webview_window("overlay") {
                if let Ok(hwnd) = window.hwnd() {
                    platform_windows::hide_window(hwnd.0 as isize);
                }
            }
        }
    });
}

fn ephemeral(app: &AppHandle, state: &str, title: &str, detail: &str) {
    show_overlay(app, InputTarget::capture(), state, title, detail, 0.0);
    terminal_overlay(app, "", state, title, detail);
}

fn model_repository(state: &AppState) -> Result<ModelRepository, String> {
    let configured = state
        .config
        .lock()
        .map_err(|_| "配置状态损坏")?
        .model_storage_dir
        .clone();
    let root = configured
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| state.default_model_root.clone());
    ModelRepository::new(root)
}

fn current_adapter(state: &AppState, config: &Config) -> Result<String, String> {
    let model_id = if config.selected_model_id.is_empty() {
        "sensevoice-small"
    } else {
        &config.selected_model_id
    };
    Ok(model_repository(state)?
        .definition(model_id)?
        .adapter_type
        .clone())
}

fn prefers_native_sensevoice(config: &Config, adapter_type: &str) -> bool {
    if adapter_type != "sensevoice" {
        return false;
    }
    if config.model_path.trim().is_empty() {
        return true;
    }
    let root = std::path::Path::new(&config.model_path);
    root.join("model.onnx").is_file() && root.join("tokens.txt").is_file()
}

fn worker_device<'a>(config: &'a Config, adapter_type: &str) -> &'a str {
    if prefers_native_sensevoice(config, adapter_type) {
        "cpu"
    } else {
        &config.device_preference
    }
}

fn schedule_model_unload(app: &AppHandle) {
    let state = app.state::<AppState>();
    let Ok(config) = state.config.lock().map(|value| value.clone()) else {
        return;
    };
    if config.model_load_mode == "resident" || config.unload_policy == "session" {
        return;
    }
    let epoch = state.unload_epoch.fetch_add(1, Ordering::Relaxed) + 1;
    let delay = if config.unload_policy == "immediate" {
        0
    } else {
        config.idle_timeout_seconds
    };
    let app = app.clone();
    thread::spawn(move || {
        if delay > 0 {
            thread::sleep(Duration::from_secs(delay));
        }
        let state = app.state::<AppState>();
        let idle = state
            .runtime
            .lock()
            .map(|runtime| runtime.phase == Phase::Idle)
            .unwrap_or(false);
        if unload_timer_is_current(idle, epoch, state.unload_epoch.load(Ordering::Relaxed)) {
            if let Ok(mut worker) = state.worker.lock() {
                let _ = worker.unload();
            }
            if let Ok(mut worker) = state.preview_worker.lock() {
                let _ = worker.unload();
            }
        }
    });
}

fn preload_current_model(app: &AppHandle) {
    let state = app.state::<AppState>();
    let Ok(config) = state.config.lock().map(|value| value.clone()) else {
        return;
    };
    if !matches!(configure_worker_runtime(&state, &config), Ok(status) if status.ready) {
        return;
    }
    if config.model_load_mode != "resident"
        || config.model_path.is_empty()
        || !PathBuf::from(&config.model_path).is_dir()
    {
        return;
    }
    let Ok(adapter_type) = current_adapter(&state, &config) else {
        return;
    };
    let worker = state.worker.clone();
    thread::spawn(move || {
        if let Ok(mut worker) = worker.lock() {
            let _ = worker.load_model(
                &config.python_path,
                &uuid::Uuid::new_v4().to_string(),
                &config.model_path,
                &adapter_type,
                worker_device(&config, &adapter_type),
            );
        }
    });
}

fn reload_selected_model(app: &AppHandle) {
    let worker = app.state::<AppState>().worker.clone();
    let app = app.clone();
    thread::spawn(move || {
        if let Ok(mut worker) = worker.lock() {
            let _ = worker.unload();
        }
        preload_current_model(&app);
    });
}

fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn truncate(text: &str, maximum: usize) -> String {
    let mut value = text.chars().take(maximum).collect::<String>();
    if text.chars().count() > maximum {
        value.push('…');
    }
    value
}

fn tail_text(text: &str, maximum: usize) -> String {
    let chars = text.chars().collect::<Vec<_>>();
    if chars.len() <= maximum {
        return text.to_owned();
    }
    format!(
        "…{}",
        chars[chars.len() - maximum..].iter().collect::<String>()
    )
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let state = app.state::<AppState>();
    let config = state
        .config
        .lock()
        .map(|value| value.clone())
        .unwrap_or_default();
    let cards = model_repository(&state)
        .map(|repository| managed_speech_model_cards(&repository))
        .unwrap_or_default();
    let english = config_uses_english(&config);
    let mut model_menu = SubmenuBuilder::new(app, text(english, "当前模型", "Current model"));
    for card in cards.iter().filter(|card| card.definition.purpose == "asr") {
        let installed = matches!(card.state.as_str(), "installed" | "custom");
        if installed {
            let selected = card.definition.id == config.selected_model_id;
            model_menu = model_menu.text(
                format!("model:{}", card.definition.id),
                format!(
                    "{}{}",
                    if selected { "✓ " } else { "" },
                    card.definition.display_name
                ),
            );
        } else {
            model_menu = model_menu.text(
                format!("missing-model:{}", card.definition.id),
                format!(
                    "○ {} · {}",
                    card.definition.display_name,
                    text(english, "未安装", "Not installed")
                ),
            );
        }
    }
    let model_menu = model_menu.build()?;
    let paused = state.shortcut_paused.load(Ordering::Relaxed);
    let menu = MenuBuilder::new(app)
        .text("open", text(english, "打开主窗口", "Open Rain"))
        .item(&model_menu)
        .text("models", text(english, "模型管理", "Model management"))
        .text(
            "toggle-shortcut",
            if paused {
                text(english, "恢复快捷键", "Resume hotkey")
            } else {
                text(english, "暂停快捷键", "Pause hotkey")
            },
        )
        .text("settings", text(english, "设置", "Settings"))
        .text(
            "check-update",
            text(english, "检查更新", "Check for updates"),
        )
        .separator()
        .text("quit", text(english, "退出", "Quit"))
        .build()?;
    if let Some(tray) = app.tray_by_id("rain-tray") {
        tray.set_menu(Some(menu))?;
        return Ok(());
    }
    let mut builder = TrayIconBuilder::with_id("rain-tray")
        .tooltip(text(english, "雨音输入法", "Rain Vibetype"))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show_main(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "models" => {
                show_main(app);
                let _ = app.emit("navigate", "model");
            }
            "settings" => {
                show_main(app);
                let _ = app.emit("navigate", "general");
            }
            "toggle-shortcut" => {
                let state = app.state::<AppState>();
                state.shortcut_paused.fetch_xor(true, Ordering::Relaxed);
                let _ = setup_tray(app);
            }
            "check-update" => {
                show_main(app);
                let _ = app.emit("check-update", ());
            }
            "quit" => {
                let busy = app
                    .state::<AppState>()
                    .runtime
                    .lock()
                    .map(|runtime| runtime.phase != Phase::Idle)
                    .unwrap_or(false);
                if !busy || platform_windows::confirm_exit(app_uses_english(app)) {
                    let _ = cancel(app);
                    app.exit(0);
                }
            }
            id if id.starts_with("model:") => {
                let model_id = id.trim_start_matches("model:");
                let state = app.state::<AppState>();
                if let Err(error) = select_model_internal(app, &state, model_id) {
                    ephemeral(app, "failed", "无法切换模型", &error);
                }
            }
            id if id.starts_with("missing-model:") => {
                show_main(app);
                let _ = app.emit("navigate", "models");
            }
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if shortcut.matches(Modifiers::empty(), Code::Escape)
                        && event.state == ShortcutState::Pressed
                    {
                        let _ = cancel(app);
                        return;
                    }
                    let state = app.state::<AppState>();
                    let configured = state
                        .config
                        .lock()
                        .map(|config| config.hotkey.clone())
                        .unwrap_or_default();
                    if !shortcut_matches_configured(shortcut, &configured) {
                        return;
                    }
                    let recording = state
                        .runtime
                        .lock()
                        .map(|runtime| runtime.phase == Phase::Recording)
                        .unwrap_or(false);
                    if state.shortcut_paused.load(Ordering::Relaxed)
                        && !(recording && event.state == ShortcutState::Released)
                    {
                        return;
                    }
                    let mode = app
                        .state::<AppState>()
                        .config
                        .lock()
                        .map(|config| config.recording_mode.clone())
                        .unwrap_or_else(|_| "push_to_talk".into());
                    match (mode.as_str(), event.state) {
                        ("push_to_talk", ShortcutState::Pressed) => {
                            if recording {
                                return;
                            }
                            let _ = start_recording(app);
                        }
                        ("push_to_talk", ShortcutState::Released) => {
                            let _ = stop_recording(app);
                        }
                        ("toggle", ShortcutState::Pressed) => {
                            let _ = if recording {
                                stop_recording(app)
                            } else {
                                start_recording(app)
                            };
                        }
                        _ => {}
                    }
                })
                .build(),
        )
        .setup(|app| {
            let config_path = app.path().app_config_dir()?.join("config.json");
            let default_model_root = app.path().app_data_dir()?.join("models");
            let runtime_root = app.path().app_data_dir()?.join("runtimes");
            let text_runtime_root = app.path().app_data_dir()?.join("text-runtimes");
            let worker_script = app
                .path()
                .app_cache_dir()?
                .join("worker")
                .join("rain_worker.py");
            let bundled_worker = runtime_root.join("runtime-not-installed.exe");
            let log_dir = app.path().app_log_dir()?;
            diagnostics::install_panic_marker(log_dir.clone());
            let (escape_shortcut, escape_shortcut_commands) = mpsc::channel();
            let app_handle = app.handle().clone();
            thread::spawn(move || {
                for enabled in escape_shortcut_commands {
                    let _ = if enabled {
                        app_handle.global_shortcut().register(ESCAPE)
                    } else {
                        app_handle.global_shortcut().unregister(ESCAPE)
                    };
                }
            });
            app.manage(AppState {
                config: Mutex::new(Config::default()),
                config_path,
                default_model_root,
                runtime_root,
                text_runtime_root,
                runtime: Mutex::new(Runtime::default()),
                worker: Arc::new(Mutex::new(WorkerClient::new(
                    worker_script.clone(),
                    bundled_worker.clone(),
                ))),
                preview_worker: Arc::new(Mutex::new(WorkerClient::new(
                    worker_script.clone(),
                    bundled_worker,
                ))),
                text_polisher: Arc::new(Mutex::new(text_polish::TextPolisher::default())),
                diagnostics: diagnostics::Diagnostics::new(log_dir),
                system_status: Mutex::new(SystemStatus::default()),
                active_download: Mutex::new(None),
                shortcut_paused: AtomicBool::new(false),
                overlay_visible: AtomicBool::new(false),
                overlay_epoch: AtomicU64::new(0),
                unload_epoch: AtomicU64::new(0),
                escape_shortcut,
            });
            let state = app.state::<AppState>();
            let mut loaded = config::load(&state.config_path);
            if loaded.selected_model_id.is_empty() && !loaded.model_path.is_empty() {
                loaded.selected_model_id = "sensevoice-small".into();
            }
            *state.config.lock().map_err(|_| "配置状态损坏")? = loaded.clone();
            worker::install_script(&worker_script)?;
            configure_worker_runtime(&state, &loaded)?;
            let shortcut_result = app.global_shortcut().register(loaded.hotkey.as_str());
            let autostart = app.autolaunch();
            let autostart_result = if loaded.autostart {
                autostart.enable()
            } else {
                autostart.disable()
            };
            *state.system_status.lock().map_err(|_| "系统状态损坏")? = SystemStatus {
                shortcut_ready: shortcut_result.is_ok(),
                shortcut_error: shortcut_result
                    .err()
                    .map(|error| format!("默认快捷键注册失败：{error}")),
                autostart_ready: autostart_result.is_ok(),
                autostart_error: autostart_result.err().map(|error| error.to_string()),
            };
            setup_tray(app.handle())?;
            preload_current_model(app.handle());
            if loaded.auto_check_updates && update_configuration().is_ok() {
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    if let Ok(update) = query_update(&app_handle).await {
                        if update.available {
                            let _ = app_handle.emit("update-available", update);
                        }
                    }
                });
            }
            if std::env::args().any(|argument| argument == "--autostart") {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            get_system_status,
            save_config,
            list_input_devices,
            test_input_level,
            check_worker,
            get_runtime_status,
            refresh_runtime_status,
            get_text_polish_status,
            delete_text_model,
            delete_text_runtime,
            list_models,
            check_model_updates,
            download_model,
            pause_model_download,
            verify_model,
            import_model,
            select_model,
            delete_model,
            delete_old_model_versions,
            cancel_current,
            get_pending_text,
            copy_pending_text,
            diagnostic_files,
            export_diagnostics,
            open_log_directory,
            pending_crash_report,
            submit_crash_report,
            dismiss_crash_report,
            check_update,
            install_update
        ])
        .run(tauri::generate_context!())
        .expect("Rain failed to start");
}

fn shortcut_matches_configured(shortcut: &Shortcut, configured: &str) -> bool {
    configured
        .parse::<Shortcut>()
        .map(|configured| configured.id() == shortcut.id())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_hotkey_matches_canonical_callback_identity() {
        let callback = "Shift+Control+Space".parse::<Shortcut>().unwrap();
        assert!(shortcut_matches_configured(&callback, "Ctrl+Shift+Space"));
    }

    #[test]
    fn native_sensevoice_is_default_but_requires_native_files_for_existing_models() {
        let mut config = Config::default();
        assert!(prefers_native_sensevoice(&config, "sensevoice"));
        assert!(!prefers_native_sensevoice(&config, "paraformer_zh"));

        let directory = std::env::temp_dir().join(format!("rain-native-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        config.model_path = directory.to_string_lossy().into_owned();
        assert!(!prefers_native_sensevoice(&config, "sensevoice"));
        std::fs::write(directory.join("model.onnx"), []).unwrap();
        std::fs::write(directory.join("tokens.txt"), []).unwrap();
        assert!(prefers_native_sensevoice(&config, "sensevoice"));
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn official_download_defaults_use_the_project_release() {
        for endpoint in [
            OFFICIAL_UPDATE_ENDPOINT,
            OFFICIAL_MODEL_MANIFEST_ENDPOINT,
            OFFICIAL_RUNTIME_MANIFEST_ENDPOINT,
        ] {
            assert!(endpoint.starts_with(
                "https://github.com/qixiaoyu27/Rain-VibeType/releases/latest/download/"
            ));
        }
    }

    #[test]
    fn state_machine_blocks_overlap_and_allows_cancel_in_required_phases() {
        assert!(can_start_recording(Phase::Idle));
        for phase in [
            Phase::Recording,
            Phase::WaitingForModel,
            Phase::Transcribing,
            Phase::Injecting,
        ] {
            assert!(!can_start_recording(phase));
        }
        assert!(can_cancel(Phase::Recording));
        assert!(can_cancel(Phase::WaitingForModel));
        assert!(can_cancel(Phase::Transcribing));
        assert!(!can_cancel(Phase::Injecting));
    }

    #[test]
    fn late_or_cancelled_transcription_is_rejected() {
        let current = Runtime {
            phase: Phase::Transcribing,
            request_id: Some("current".into()),
            ..Runtime::default()
        };
        assert!(accepts_transcription(&current, "current"));
        assert!(!accepts_transcription(&current, "late"));
        let cancelled = Runtime::default();
        assert!(!accepts_transcription(&cancelled, "current"));
    }

    #[test]
    fn stale_unload_timer_cannot_unload_a_new_session() {
        assert!(unload_timer_is_current(true, 9, 9));
        assert!(!unload_timer_is_current(true, 8, 9));
        assert!(!unload_timer_is_current(false, 9, 9));
    }

    #[test]
    fn recording_detail_keeps_only_the_latest_preview_text() {
        let preview = "一二三四五六七八九十一二三四五六七八九十一二三四五六七八九十";
        let detail = recording_detail(3, 60, preview, false);
        assert!(detail.starts_with("00:03 · …"));
        assert!(detail.ends_with("七八九十"));
        assert_eq!(tail_text("实时预览", 24), "实时预览");
    }
}
