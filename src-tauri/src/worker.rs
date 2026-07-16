use serde::Serialize;
use serde_json::Value;
use std::{
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    time::Duration,
};

#[derive(Debug, Serialize)]
pub struct Transcription {
    pub text: String,
    pub language: String,
    pub duration_ms: u64,
    pub inference_ms: u64,
}

pub struct WorkerClient {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    events: Option<Receiver<Result<String, String>>>,
    python_path: String,
    script_path: PathBuf,
    bundled_worker: PathBuf,
    loaded_model: Option<(String, String, String)>,
    consecutive_crashes: u8,
    restart_blocked: bool,
    read_timeout: Duration,
    #[cfg(windows)]
    job: Option<crate::platform_windows::KillOnDropJob>,
}

impl WorkerClient {
    pub fn new(script_path: PathBuf, bundled_worker: PathBuf) -> Self {
        Self {
            child: None,
            stdin: None,
            events: None,
            python_path: String::new(),
            script_path,
            bundled_worker,
            loaded_model: None,
            consecutive_crashes: 0,
            restart_blocked: false,
            read_timeout: Duration::from_secs(600),
            #[cfg(windows)]
            job: None,
        }
    }

    pub fn check(&mut self, python_path: &str) -> Result<String, String> {
        // A user-initiated health check is the explicit way to retry after the
        // automatic restart guard has stopped a crash loop.
        self.consecutive_crashes = 0;
        self.restart_blocked = false;
        self.ensure_started(python_path)?;
        let request_id = uuid::Uuid::new_v4().to_string();
        self.send(
            serde_json::json!({"command": "get_status", "request_id": request_id}),
            &[],
        )?;
        let response = self.read_until(&request_id, &["status"])?;
        if !response["runtime_ready"].as_bool().unwrap_or(false) {
            return Err(format!(
                "Worker 缺少依赖：{}",
                response["missing_dependencies"]
                    .as_array()
                    .map(|values| values
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", "))
                    .unwrap_or_else(|| "funasr / numpy / torch".into())
            ));
        }
        let ready = response["model_ready"].as_bool().unwrap_or(false);
        Ok(if ready {
            format!(
                "Worker 正常，模型已加载到 {}",
                response["device"].as_str().unwrap_or("未知设备")
            )
        } else {
            "Worker 正常，模型将在首次录音时加载".into()
        })
    }

    pub fn set_bundled_worker(&mut self, executable: PathBuf) {
        if self.bundled_worker != executable {
            self.stop();
            self.bundled_worker = executable;
        }
    }

    pub fn load_model(
        &mut self,
        python_path: &str,
        request_id: &str,
        model_path: &str,
        adapter_type: &str,
        device: &str,
    ) -> Result<(), String> {
        self.ensure_started(python_path)?;
        let desired = (
            model_path.to_owned(),
            adapter_type.to_owned(),
            device.to_owned(),
        );
        if self.loaded_model.as_ref() == Some(&desired) {
            return Ok(());
        }
        self.send(
            serde_json::json!({
                "command": "load_model",
                "request_id": request_id,
                "model_path": model_path,
                "adapter_type": adapter_type,
                "device": device,
                "options": {}
            }),
            &[],
        )?;
        self.read_until(request_id, &["model_ready"])?;
        self.loaded_model = Some(desired);
        Ok(())
    }

    pub fn transcribe_loaded(
        &mut self,
        request_id: &str,
        pcm: Vec<i16>,
    ) -> Result<Transcription, String> {
        let mut bytes = Vec::with_capacity(pcm.len() * 2);
        for sample in pcm {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        self.send(
            serde_json::json!({
                "command": "transcribe",
                "request_id": request_id,
                "sample_rate": 16000,
                "audio_bytes": bytes.len()
            }),
            &bytes,
        )?;
        drop(bytes);
        let response = self.read_until(request_id, &["transcription_completed"])?;
        let text = response["text"]
            .as_str()
            .unwrap_or_default()
            .trim()
            .to_owned();
        if text.is_empty() {
            return Err("TRANSCRIPTION_EMPTY：没有识别到文字".into());
        }
        Ok(Transcription {
            text,
            language: response["language"].as_str().unwrap_or("zh").to_owned(),
            duration_ms: response["duration_ms"].as_u64().unwrap_or(0),
            inference_ms: response["inference_ms"].as_u64().unwrap_or(0),
        })
        .inspect(|_| {
            self.consecutive_crashes = 0;
            self.restart_blocked = false;
        })
    }

    pub fn start_preview(&mut self, request_id: &str) -> Result<(), String> {
        self.send(
            serde_json::json!({"command": "preview_start", "request_id": request_id}),
            &[],
        )?;
        self.read_until(request_id, &["preview_started"])?;
        Ok(())
    }

    pub fn preview_audio(
        &mut self,
        request_id: &str,
        sample_rate: u32,
        pcm: Vec<i16>,
    ) -> Result<String, String> {
        let mut bytes = Vec::with_capacity(pcm.len() * 2);
        for sample in pcm {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        self.send(
            serde_json::json!({
                "command": "preview_audio",
                "request_id": request_id,
                "sample_rate": sample_rate,
                "audio_bytes": bytes.len()
            }),
            &bytes,
        )?;
        let response = self.read_until(request_id, &["preview_partial"])?;
        Ok(response["text"]
            .as_str()
            .unwrap_or_default()
            .trim()
            .to_owned())
    }

    pub fn finish_preview(&mut self, request_id: &str) -> Result<(), String> {
        self.send(
            serde_json::json!({"command": "preview_finish", "request_id": request_id}),
            &[],
        )?;
        self.read_until(request_id, &["preview_completed"])?;
        Ok(())
    }

    pub fn unload(&mut self) -> Result<(), String> {
        if self.child.is_none() || self.loaded_model.is_none() {
            self.loaded_model = None;
            return Ok(());
        }
        let request_id = uuid::Uuid::new_v4().to_string();
        self.send(
            serde_json::json!({"command": "unload_model", "request_id": request_id}),
            &[],
        )?;
        self.read_until(&request_id, &["model_unloaded"])?;
        self.loaded_model = None;
        Ok(())
    }

    pub fn shutdown(&mut self) {
        self.stop();
    }

    fn ensure_started(&mut self, python_path: &str) -> Result<(), String> {
        if self.restart_blocked {
            return Err(
                "WORKER_CRASHED：Worker 连续崩溃，已停止自动重启；请在诊断页执行 Worker 检查后重试"
                    .into(),
            );
        }
        let bundled = self.bundled_worker.is_file();
        let executable = if bundled {
            self.bundled_worker.clone()
        } else {
            resolve_executable(python_path)
        };
        let runtime_identity = executable.to_string_lossy().into_owned();
        let had_child = self.child.is_some();
        let running = self
            .child
            .as_mut()
            .map(|child| child.try_wait().ok().flatten().is_none())
            .unwrap_or(false);
        if running && self.python_path == runtime_identity {
            return Ok(());
        }
        if had_child && !running {
            self.note_crash();
            if self.restart_blocked {
                self.stop();
                return Err("WORKER_CRASHED：Worker 连续崩溃，已停止自动重启；请检查模型、推理设备或诊断信息".into());
            }
        }
        self.stop();
        let mut command = Command::new(&executable);
        if !bundled {
            command.arg(&self.script_path);
        }
        command
            .env("PYTHONUNBUFFERED", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                self.note_crash();
                return Err(format!(
                    "无法启动推理 Worker（{}）：{error}",
                    executable.display()
                ));
            }
        };
        #[cfg(windows)]
        let job = {
            use std::os::windows::io::AsRawHandle;
            match crate::platform_windows::KillOnDropJob::attach(child.as_raw_handle()) {
                Ok(job) => job,
                Err(error) => {
                    let _ = child.kill();
                    return Err(error);
                }
            }
        };
        let stdin = child.stdin.take().ok_or("无法连接 Worker 输入")?;
        let stdout = child.stdout.take().ok_or("无法连接 Worker 输出")?;
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let mut stdout = BufReader::new(stdout);
            loop {
                let mut line = String::new();
                match stdout.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        if sender.send(Ok(line)).is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = sender.send(Err(error.to_string()));
                        break;
                    }
                }
            }
        });
        self.child = Some(child);
        self.stdin = Some(stdin);
        self.events = Some(receiver);
        self.python_path = runtime_identity;
        self.loaded_model = None;
        #[cfg(windows)]
        {
            self.job = Some(job);
        }

        let ready = self.read_event()?;
        if ready["event"] != "worker_ready" {
            self.note_crash();
            self.stop();
            return Err(format!("Worker 启动失败：{ready}"));
        }
        Ok(())
    }

    fn send(&mut self, message: Value, audio: &[u8]) -> Result<(), String> {
        let stdin = self.stdin.as_mut().ok_or("Worker 未启动")?;
        serde_json::to_writer(&mut *stdin, &message).map_err(|error| error.to_string())?;
        stdin.write_all(b"\n").map_err(|error| error.to_string())?;
        stdin.write_all(audio).map_err(|error| error.to_string())?;
        stdin.flush().map_err(|error| error.to_string())
    }

    fn read_until(&mut self, request_id: &str, accepted: &[&str]) -> Result<Value, String> {
        loop {
            let response = self.read_event()?;
            if response["request_id"].as_str() != Some(request_id) {
                continue;
            }
            let event = response["event"].as_str().unwrap_or_default();
            if accepted.contains(&event) {
                return Ok(response);
            }
            if matches!(event, "worker_error" | "transcription_failed") {
                let code = response["code"].as_str().unwrap_or("WORKER_CRASHED");
                let message = response["message"].as_str().unwrap_or("Worker 执行失败");
                return Err(format!("{code}：{message}"));
            }
        }
    }

    fn read_event(&mut self) -> Result<Value, String> {
        loop {
            let received = self
                .events
                .as_ref()
                .ok_or("Worker 未启动")?
                .recv_timeout(self.read_timeout);
            let line = match received {
                Ok(Ok(line)) => line,
                Ok(Err(error)) => {
                    self.note_crash();
                    self.stop();
                    return Err(format!("WORKER_CRASHED：读取 Worker 失败：{error}"));
                }
                Err(RecvTimeoutError::Timeout) => {
                    self.note_crash();
                    self.stop();
                    return Err("WORKER_CRASHED：Worker 响应超时".into());
                }
                Err(RecvTimeoutError::Disconnected) => {
                    self.note_crash();
                    self.stop();
                    return Err("WORKER_CRASHED：Worker 已退出".into());
                }
            };
            if let Ok(message) = serde_json::from_str(&line) {
                return Ok(message);
            }
        }
    }

    fn note_crash(&mut self) {
        self.consecutive_crashes = self.consecutive_crashes.saturating_add(1);
        if self.consecutive_crashes >= 3 {
            self.restart_blocked = true;
        }
    }

    fn stop(&mut self) {
        let shutdown_sent = if let Some(stdin) = self.stdin.as_mut() {
            let sent = writeln!(
                stdin,
                "{{\"command\":\"shutdown\",\"request_id\":\"exit\"}}"
            )
            .and_then(|_| stdin.flush())
            .is_ok();
            sent
        } else {
            false
        };
        drop(self.stdin.take());
        if let Some(mut child) = self.child.take() {
            let deadline = std::time::Instant::now() + Duration::from_millis(500);
            while shutdown_sent
                && child.try_wait().ok().flatten().is_none()
                && std::time::Instant::now() < deadline
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
            }
            let _ = child.wait();
            #[cfg(windows)]
            drop(self.job.take());
        }
        self.events = None;
        self.loaded_model = None;
    }
}

impl Drop for WorkerClient {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn install_script(path: &Path) -> std::io::Result<()> {
    const SOURCE: &str = include_str!("../../worker/rain_worker.py");
    if matches!(std::fs::read_to_string(path), Ok(content) if content == SOURCE) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, SOURCE)
}

fn resolve_executable(configured: &str) -> PathBuf {
    let path = Path::new(configured);
    if path.is_absolute() || path.components().count() == 1 {
        path.to_owned()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn python_available() -> bool {
        Command::new("python")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    #[test]
    fn repeated_worker_crashes_stop_automatic_restart() {
        let mut worker = WorkerClient::new(
            PathBuf::from("worker.py"),
            PathBuf::from("missing-worker.exe"),
        );
        worker.note_crash();
        worker.note_crash();
        assert!(!worker.restart_blocked);
        worker.note_crash();
        assert!(worker.restart_blocked);
    }

    #[test]
    fn ipc_handshake_request_ids_and_binary_audio_round_trip() {
        if !python_available() {
            return;
        }
        let directory = std::env::temp_dir().join(format!("rain-ipc-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let script = directory.join("fake_worker.py");
        let shutdown_marker = directory.join("shutdown-complete");
        let shutdown_marker_for_python = shutdown_marker.to_string_lossy().replace('\\', "\\\\");
        std::fs::write(
            &script,
            format!(r#"import json, sys
def emit(event, request_id=None, **data):
    value = {{"event": event, **data}}
    if request_id is not None: value["request_id"] = request_id
    sys.stdout.write(json.dumps(value) + "\n"); sys.stdout.flush()
emit("worker_ready")
while True:
    line = sys.stdin.buffer.readline()
    if not line: break
    message = json.loads(line)
    count = int(message.get("audio_bytes", 0))
    audio = sys.stdin.buffer.read(count)
    request_id = message.get("request_id", "")
    command = message.get("command")
    if command == "load_model":
        emit("model_ready", request_id, device="cpu")
        emit("model_ready", request_id, device="cpu")
    elif command == "transcribe":
        emit("transcription_started", "late-request")
        emit("transcription_completed", request_id, text=str(len(audio)), language="en", duration_ms=1, inference_ms=2)
    elif command == "preview_start": emit("preview_started", request_id)
    elif command == "preview_audio": emit("preview_partial", request_id, text=str(len(audio)))
    elif command == "preview_finish": emit("preview_completed", request_id, text="")
    elif command == "unload_model": emit("model_unloaded", request_id)
    elif command == "shutdown":
        open("{shutdown_marker_for_python}", "w", encoding="utf-8").write("ok")
        emit("shutdown_complete", request_id)
        break
"#),
        )
        .unwrap();
        let mut worker = WorkerClient::new(script, directory.join("missing.exe"));
        worker
            .load_model("python", "load-1", "model", "sensevoice", "cpu")
            .unwrap();
        let result = worker
            .transcribe_loaded("request-1", vec![1_i16, 2_i16, 3_i16])
            .unwrap();
        assert_eq!(result.text, "6");
        assert_eq!(result.inference_ms, 2);
        worker.unload().unwrap();
        worker
            .load_model(
                "python",
                "load-preview",
                "preview",
                "streaming_zipformer",
                "cpu",
            )
            .unwrap();
        worker.start_preview("preview-1").unwrap();
        assert_eq!(
            worker
                .preview_audio("preview-1", 16_000, vec![1_i16, 2_i16])
                .unwrap(),
            "4"
        );
        worker.finish_preview("preview-1").unwrap();
        worker.stop();
        assert!(shutdown_marker.is_file());
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn worker_response_timeout_stops_the_child() {
        if !python_available() {
            return;
        }
        let directory = std::env::temp_dir().join(format!("rain-timeout-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let script = directory.join("slow_worker.py");
        std::fs::write(
            &script,
            r#"import json, sys
sys.stdout.write(json.dumps({"event": "worker_ready"}) + "\n"); sys.stdout.flush()
while True:
    line = sys.stdin.buffer.readline()
    if not line: break
    message = json.loads(line)
    if message.get("command") == "shutdown": break
"#,
        )
        .unwrap();
        let mut worker = WorkerClient::new(script, directory.join("missing.exe"));
        worker.read_timeout = Duration::from_millis(50);
        let error = worker
            .load_model("python", "load-timeout", "model", "sensevoice", "cpu")
            .unwrap_err();
        assert!(error.contains("响应超时"));
        assert!(worker.child.is_none());
        let _ = std::fs::remove_dir_all(directory);
    }
}
