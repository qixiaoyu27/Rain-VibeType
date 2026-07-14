use crate::config::Config;
use serde::Serialize;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
use zip::{write::SimpleFileOptions, ZipWriter};

#[derive(Clone, Serialize)]
struct DiagnosticEvent {
    timestamp: u64,
    event: String,
    model_id: String,
    duration_ms: Option<u64>,
    inference_ms: Option<u64>,
}

pub struct Diagnostics {
    log_dir: PathBuf,
    events: Mutex<Vec<DiagnosticEvent>>,
}

impl Diagnostics {
    pub fn new(log_dir: PathBuf) -> Self {
        Self {
            log_dir,
            events: Mutex::new(Vec::new()),
        }
    }

    pub fn log_dir(&self) -> &Path {
        &self.log_dir
    }

    pub fn record(
        &self,
        event: &str,
        model_id: &str,
        duration_ms: Option<u64>,
        inference_ms: Option<u64>,
    ) {
        let entry = DiagnosticEvent {
            timestamp: now(),
            event: sanitize_event(event),
            model_id: model_id.to_owned(),
            duration_ms,
            inference_ms,
        };
        if let Ok(mut events) = self.events.lock() {
            events.push(entry.clone());
            if events.len() > 200 {
                events.remove(0);
            }
        }
        let _ = fs::create_dir_all(&self.log_dir);
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.log_dir.join("rain.log"))
        {
            if let Ok(line) = serde_json::to_string(&entry) {
                let _ = writeln!(file, "{line}");
            }
        }
    }

    pub fn export(&self, destination: &Path, config: &Config) -> Result<(), String> {
        let file = File::create(destination).map_err(|error| format!("无法创建诊断包：{error}"))?;
        let mut archive = ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        let summary = serde_json::json!({
            "app_version": env!("CARGO_PKG_VERSION"),
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "generated_at": now(),
            "contents": Self::file_list(),
        });
        write_json(&mut archive, "summary.json", &summary, options)?;

        let mut redacted = config.clone();
        redacted.model_path = "<redacted>".into();
        redacted.python_path = "<redacted>".into();
        redacted.model_storage_dir = redacted.model_storage_dir.map(|_| "<redacted>".into());
        write_json(&mut archive, "config.redacted.json", &redacted, options)?;
        let events = self
            .events
            .lock()
            .map(|events| events.clone())
            .unwrap_or_default();
        write_json(&mut archive, "recent-events.json", &events, options)?;
        archive.finish().map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn file_list() -> Vec<&'static str> {
        vec!["summary.json", "config.redacted.json", "recent-events.json"]
    }

    pub fn pending_crash_report(&self, config: &Config) -> Option<serde_json::Value> {
        if !config.anonymous_crash_reports {
            return None;
        }
        let marker = fs::read(self.log_dir.join("last-crash.json")).ok()?;
        let marker: serde_json::Value = serde_json::from_slice(&marker).ok()?;
        Some(serde_json::json!({
            "event": marker.get("event").and_then(|value| value.as_str()).unwrap_or("APP_PANIC"),
            "timestamp": marker.get("timestamp").and_then(|value| value.as_u64()).unwrap_or(0),
            "app_version": env!("CARGO_PKG_VERSION"),
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "model_id": config.selected_model_id,
            "backend": "local-python-worker"
        }))
    }

    pub fn clear_crash_report(&self) -> Result<(), String> {
        let path = self.log_dir.join("last-crash.json");
        if path.exists() {
            fs::remove_file(path).map_err(|error| format!("无法清理崩溃报告：{error}"))?;
        }
        Ok(())
    }
}

pub fn install_panic_marker(log_dir: PathBuf) {
    std::panic::set_hook(Box::new(move |_| {
        let _ = fs::create_dir_all(&log_dir);
        let marker = serde_json::json!({"timestamp": now(), "event": "APP_PANIC"});
        let _ = fs::write(log_dir.join("last-crash.json"), marker.to_string());
    }));
}

fn write_json<T: Serialize>(
    archive: &mut ZipWriter<File>,
    name: &str,
    value: &T,
    options: SimpleFileOptions,
) -> Result<(), String> {
    archive
        .start_file(name, options)
        .map_err(|error| error.to_string())?;
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    archive.write_all(&bytes).map_err(|error| error.to_string())
}

fn sanitize_event(value: &str) -> String {
    value
        .split(['：', ':'])
        .next()
        .unwrap_or("UNKNOWN")
        .chars()
        .filter(|character| character.is_ascii_uppercase() || *character == '_')
        .take(64)
        .collect::<String>()
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0)
}
