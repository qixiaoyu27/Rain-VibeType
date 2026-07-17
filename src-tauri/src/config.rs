use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub schema_version: u32,
    pub onboarding_completed: bool,
    pub recording_mode: String,
    pub hotkey: String,
    pub max_recording_seconds: u64,
    pub input_device: Option<String>,
    pub duck_system_audio: bool,
    pub selected_model_id: String,
    pub model_path: String,
    pub model_storage_dir: Option<String>,
    pub python_path: String,
    pub device_preference: String,
    pub model_load_mode: String,
    pub unload_policy: String,
    pub idle_timeout_seconds: u64,
    pub injection_method: String,
    pub restore_clipboard: bool,
    pub remove_terminal_period: bool,
    pub text_polish_enabled: bool,
    pub text_polish_rewrite: bool,
    pub text_polish_remove_fillers: bool,
    pub text_polish_paragraphs: bool,
    pub text_polish_protected_terms: Vec<String>,
    pub text_polish_idle_timeout_seconds: u64,
    pub show_overlay: bool,
    pub show_overlay_fullscreen: bool,
    pub overlay_opacity: f64,
    pub start_sound: bool,
    pub stop_sound: bool,
    pub error_sound: bool,
    pub feedback_disabled_confirmed: bool,
    pub autostart: bool,
    pub auto_check_updates: bool,
    pub ui_language: String,
    pub anonymous_crash_reports: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: 1,
            onboarding_completed: false,
            recording_mode: "push_to_talk".into(),
            hotkey: "Ctrl+Shift+Space".into(),
            max_recording_seconds: 60,
            input_device: None,
            duck_system_audio: false,
            selected_model_id: String::new(),
            model_path: String::new(),
            model_storage_dir: None,
            python_path: "python".into(),
            device_preference: "auto".into(),
            model_load_mode: "on_demand".into(),
            unload_policy: "idle_timeout".into(),
            idle_timeout_seconds: 600,
            injection_method: "clipboard".into(),
            restore_clipboard: true,
            remove_terminal_period: false,
            text_polish_enabled: false,
            text_polish_rewrite: false,
            text_polish_remove_fillers: false,
            text_polish_paragraphs: true,
            text_polish_protected_terms: Vec::new(),
            text_polish_idle_timeout_seconds: 600,
            show_overlay: true,
            show_overlay_fullscreen: true,
            overlay_opacity: 0.10,
            start_sound: true,
            stop_sound: true,
            error_sound: true,
            feedback_disabled_confirmed: false,
            autostart: true,
            auto_check_updates: true,
            ui_language: "zh-CN".into(),
            anonymous_crash_reports: false,
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("不支持的配置版本".into());
        }
        if !matches!(self.recording_mode.as_str(), "push_to_talk" | "toggle") {
            return Err("录音方式无效".into());
        }
        if self.hotkey.trim().is_empty() || self.hotkey.eq_ignore_ascii_case("escape") {
            return Err("全局快捷键无效".into());
        }
        if !(10..=3600).contains(&self.max_recording_seconds) {
            return Err("录音上限必须在 10 秒到 60 分钟之间".into());
        }
        if !matches!(self.device_preference.as_str(), "auto" | "cuda" | "cpu") {
            return Err("推理设备无效".into());
        }
        if !matches!(self.model_load_mode.as_str(), "on_demand" | "resident") {
            return Err("模型加载模式无效".into());
        }
        if !matches!(
            self.unload_policy.as_str(),
            "immediate" | "idle_timeout" | "session"
        ) {
            return Err("模型卸载策略无效".into());
        }
        if !(10..=86_400).contains(&self.idle_timeout_seconds) {
            return Err("空闲卸载时间必须在 10 秒到 24 小时之间".into());
        }
        if !matches!(self.injection_method.as_str(), "clipboard" | "typing") {
            return Err("文字注入方式无效".into());
        }
        if !(60..=86_400).contains(&self.text_polish_idle_timeout_seconds) {
            return Err("文本整理空闲时间必须在 60 秒到 24 小时之间".into());
        }
        if self.text_polish_protected_terms.len() > 100
            || self
                .text_polish_protected_terms
                .iter()
                .any(|term| term.chars().count() > 100)
        {
            return Err("受保护词最多 100 个，每个不超过 100 个字符".into());
        }
        if !matches!(self.ui_language.as_str(), "system" | "zh-CN" | "en") {
            return Err("界面语言无效".into());
        }
        if !self.show_overlay
            && !self.start_sound
            && !self.stop_sound
            && !self.error_sound
            && !self.feedback_disabled_confirmed
        {
            return Err("关闭全部视觉与声音反馈前需要明确确认".into());
        }
        if !(0.0..=1.0).contains(&self.overlay_opacity) {
            return Err("浮窗不透明度必须在 0% 到 100% 之间".into());
        }
        if self.python_path.trim().is_empty() {
            return Err("Python 路径不能为空".into());
        }
        if self
            .model_storage_dir
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty() && !Path::new(value).is_absolute())
        {
            return Err("模型存储目录必须是绝对路径".into());
        }
        Ok(())
    }
}

pub fn load(path: &Path) -> Config {
    let Ok(bytes) = fs::read(path) else {
        return Config::default();
    };
    match serde_json::from_slice::<Config>(&bytes) {
        Ok(config) if config.validate().is_ok() => config,
        _ => {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|value| value.as_secs())
                .unwrap_or(0);
            let backup = path.with_file_name(format!("config.corrupt-{stamp}.json"));
            let _ = fs::rename(path, backup);
            Config::default()
        }
    }
}

pub fn save(path: &Path, config: &Config) -> Result<(), String> {
    config.validate()?;
    let parent = path.parent().ok_or("配置目录无效")?;
    fs::create_dir_all(parent).map_err(|error| format!("无法创建配置目录：{error}"))?;
    let temp = temporary_path(path);
    let bytes = serde_json::to_vec_pretty(config).map_err(|error| error.to_string())?;
    fs::write(&temp, bytes).map_err(|error| format!("无法写入配置：{error}"))?;
    replace_file(&temp, path).map_err(|error| {
        let _ = fs::remove_file(&temp);
        format!("无法保存配置：{error}")
    })
}

fn temporary_path(path: &Path) -> PathBuf {
    path.with_extension(format!("tmp-{}", std::process::id()))
}

#[cfg(windows)]
pub(crate) fn replace_file(source: &Path, target: &Path) -> std::io::Result<()> {
    if !target.exists() {
        return fs::rename(source, target);
    }
    use std::{ffi::OsStr, iter, os::windows::ffi::OsStrExt, ptr};
    use windows_sys::Win32::Storage::FileSystem::{ReplaceFileW, REPLACEFILE_WRITE_THROUGH};

    let wide = |path: &Path| {
        OsStr::new(path)
            .encode_wide()
            .chain(iter::once(0))
            .collect::<Vec<u16>>()
    };
    let source = wide(source);
    let target = wide(target);
    let ok = unsafe {
        ReplaceFileW(
            target.as_ptr(),
            source.as_ptr(),
            ptr::null(),
            REPLACEFILE_WRITE_THROUGH,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    if ok == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
pub(crate) fn replace_file(source: &Path, target: &Path) -> std::io::Result<()> {
    fs::rename(source, target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_recording_limits() {
        let config = Config {
            max_recording_seconds: 0,
            ..Config::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn old_schema_one_config_receives_new_defaults() {
        let old = r#"{
          "schema_version": 1,
          "recording_mode": "toggle",
          "max_recording_seconds": 60,
          "input_device": null,
          "model_path": "",
          "python_path": "python",
          "device_preference": "cpu"
        }"#;
        let config: Config = serde_json::from_str(old).unwrap();
        assert_eq!(config.hotkey, "Ctrl+Shift+Space");
        assert_eq!(config.injection_method, "clipboard");
        assert_eq!(config.ui_language, "zh-CN");
        assert!(!config.duck_system_audio);
        assert!(!config.remove_terminal_period);
        assert!(!config.text_polish_enabled);
        assert!(!config.text_polish_rewrite);
        assert!(config.text_polish_paragraphs);
        assert!(config.autostart);
        assert_eq!(config.overlay_opacity, 0.10);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn rejects_invalid_overlay_opacity() {
        let config = Config {
            overlay_opacity: -0.01,
            ..Config::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn corrupt_config_is_backed_up_before_defaults_are_restored() {
        let directory = std::env::temp_dir().join(format!("rain-config-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("config.json");
        fs::write(&path, b"{not-json").unwrap();
        let loaded = load(&path);
        assert_eq!(loaded.hotkey, "Ctrl+Shift+Space");
        assert!(!path.exists());
        assert!(fs::read_dir(&directory)
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with("config.corrupt-")));
        let _ = fs::remove_dir_all(directory);
    }
}
