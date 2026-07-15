use serde::Deserialize;
use serde_json::{json, Value};
use sherpa_onnx::{OfflineRecognizer, OfflineRecognizerConfig, OfflineSenseVoiceModelConfig};
use std::{
    env,
    io::{self, BufRead, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    time::Instant,
};

const MAX_AUDIO_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Deserialize)]
struct Request {
    command: String,
    #[serde(default)]
    request_id: String,
    #[serde(default)]
    model_path: String,
    #[serde(default)]
    adapter_type: String,
    #[serde(default)]
    device: String,
    #[serde(default = "default_sample_rate")]
    sample_rate: i32,
    #[serde(default)]
    audio_bytes: usize,
}

fn default_sample_rate() -> i32 {
    16_000
}

struct LoadedModel {
    recognizer: OfflineRecognizer,
    model_path: String,
}

impl LoadedModel {
    fn load(model_path: &str, adapter_type: &str, device: &str) -> Result<Self, String> {
        if adapter_type != "sensevoice" {
            return Err(format!(
                "原生 Worker 当前只支持 SenseVoice，收到适配器：{adapter_type}"
            ));
        }
        if !matches!(device, "" | "auto" | "cpu") {
            return Err("当前原生 Worker 是 CPU 版本；CUDA 组件尚未启用".into());
        }

        let root = Path::new(model_path);
        let model = required_file(root, "model.onnx")?;
        let tokens = required_file(root, "tokens.txt")?;
        let mut config = OfflineRecognizerConfig::default();
        config.model_config.sense_voice = OfflineSenseVoiceModelConfig {
            model: Some(path_text(&model)),
            language: Some("auto".into()),
            use_itn: true,
        };
        config.model_config.tokens = Some(path_text(&tokens));
        config.model_config.provider = Some("cpu".into());
        config.model_config.num_threads = inference_threads();
        config.decoding_method = Some("greedy_search".into());
        let recognizer =
            OfflineRecognizer::create(&config).ok_or("无法创建 sherpa-onnx SenseVoice 识别器")?;
        Ok(Self {
            recognizer,
            model_path: model_path.to_owned(),
        })
    }

    fn transcribe(&self, audio: &[u8], sample_rate: i32) -> Result<String, String> {
        if audio.is_empty() || !audio.len().is_multiple_of(2) {
            return Err("录音数据为空或不是 PCM16".into());
        }
        if sample_rate <= 0 {
            return Err("录音采样率无效".into());
        }
        let samples = pcm16_to_f32(audio);
        let stream = self.recognizer.create_stream();
        stream.accept_waveform(sample_rate, &samples);
        self.recognizer.decode(&stream);
        stream
            .get_result()
            .map(|result| result.text.trim().to_owned())
            .filter(|text| !text.is_empty())
            .ok_or_else(|| "没有识别到文字".into())
    }
}

fn required_file(root: &Path, name: &str) -> Result<PathBuf, String> {
    if !root.is_dir() {
        return Err(format!("模型目录不存在：{}", root.display()));
    }
    let path = root.join(name);
    path.is_file()
        .then_some(path)
        .ok_or_else(|| format!("模型缺少 {name}；请先导出原生 ONNX 模型"))
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn inference_threads() -> i32 {
    env::var("RAIN_ASR_THREADS")
        .ok()
        .and_then(|value| value.parse::<i32>().ok())
        .filter(|value| (1..=64).contains(value))
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|value| value.get().min(8) as i32)
                .unwrap_or(4)
        })
}

fn pcm16_to_f32(audio: &[u8]) -> Vec<f32> {
    audio
        .chunks_exact(2)
        .map(|bytes| i16::from_le_bytes([bytes[0], bytes[1]]) as f32 / 32_768.0)
        .collect()
}

fn emit(output: &mut impl Write, event: &str, request_id: &str, data: Value) -> Result<(), String> {
    let mut value = match data {
        Value::Object(map) => Value::Object(map),
        _ => json!({}),
    };
    value["event"] = Value::String(event.into());
    if !request_id.is_empty() {
        value["request_id"] = Value::String(request_id.into());
    }
    serde_json::to_writer(&mut *output, &value).map_err(|error| error.to_string())?;
    output.write_all(b"\n").map_err(|error| error.to_string())?;
    output.flush().map_err(|error| error.to_string())
}

fn run(input: impl Read, output: impl Write) -> Result<(), String> {
    let mut input = BufReader::new(input);
    let mut output = BufWriter::new(output);
    let mut loaded: Option<LoadedModel> = None;
    emit(
        &mut output,
        "worker_ready",
        "",
        json!({"runtime": "sherpa-onnx", "device": "cpu"}),
    )
    .map_err(|error| error.to_string())?;

    loop {
        let mut line = String::new();
        match input.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(error) => return Err(format!("读取控制消息失败：{error}")),
        }
        let request: Request = match serde_json::from_str(&line) {
            Ok(request) => request,
            Err(error) => {
                emit(
                    &mut output,
                    "worker_error",
                    "",
                    json!({"code": "INVALID_REQUEST", "message": error.to_string()}),
                )
                .map_err(|write_error| write_error.to_string())?;
                continue;
            }
        };
        if request.audio_bytes > MAX_AUDIO_BYTES {
            emit(
                &mut output,
                "worker_error",
                &request.request_id,
                json!({"code": "AUDIO_TOO_LARGE", "message": "录音数据超过 32 MB 限制"}),
            )
            .map_err(|error| error.to_string())?;
            break;
        }
        let mut audio = vec![0_u8; request.audio_bytes];
        input
            .read_exact(&mut audio)
            .map_err(|error| format!("读取录音数据失败：{error}"))?;

        match request.command.as_str() {
            "get_status" => emit(
                &mut output,
                "status",
                &request.request_id,
                json!({
                    "runtime_ready": true,
                    "missing_dependencies": [],
                    "model_ready": loaded.is_some(),
                    "model_path": loaded.as_ref().map(|model| model.model_path.as_str()).unwrap_or(""),
                    "device": "cpu",
                    "adapter_type": loaded.as_ref().map(|_| "sensevoice").unwrap_or("")
                }),
            ),
            "load_model" => match LoadedModel::load(
                &request.model_path,
                &request.adapter_type,
                &request.device,
            ) {
                Ok(model) => {
                    loaded = Some(model);
                    emit(
                        &mut output,
                        "model_ready",
                        &request.request_id,
                        json!({"device": "cpu"}),
                    )
                }
                Err(message) => emit(
                    &mut output,
                    "worker_error",
                    &request.request_id,
                    json!({"code": "MODEL_LOAD_FAILED", "message": message}),
                ),
            },
            "transcribe" => {
                let started = Instant::now();
                emit(
                    &mut output,
                    "transcription_started",
                    &request.request_id,
                    json!({}),
                )?;
                match loaded
                    .as_ref()
                    .ok_or_else(|| "模型尚未加载".to_string())
                    .and_then(|model| model.transcribe(&audio, request.sample_rate))
                {
                    Ok(text) => emit(
                        &mut output,
                        "transcription_completed",
                        &request.request_id,
                        json!({
                            "text": text,
                            "language": "auto",
                            "duration_ms": audio.len() as u64 * 1000 / 2 / request.sample_rate as u64,
                            "inference_ms": started.elapsed().as_millis() as u64
                        }),
                    ),
                    Err(message) => emit(
                        &mut output,
                        "transcription_failed",
                        &request.request_id,
                        json!({"code": "TRANSCRIPTION_FAILED", "message": message}),
                    ),
                }
            }
            "unload_model" => {
                loaded = None;
                emit(
                    &mut output,
                    "model_unloaded",
                    &request.request_id,
                    json!({}),
                )
            }
            "shutdown" => {
                emit(
                    &mut output,
                    "shutdown_complete",
                    &request.request_id,
                    json!({}),
                )?;
                break;
            }
            _ => emit(
                &mut output,
                "worker_error",
                &request.request_id,
                json!({"code": "UNKNOWN_COMMAND", "message": "未知 Worker 命令"}),
            ),
        }
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn main() {
    if let Err(error) = run(io::stdin().lock(), io::stdout().lock()) {
        eprintln!("rain-native-worker: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcm16_conversion_and_thread_override_are_bounded() {
        assert_eq!(pcm16_to_f32(&[0, 0, 0xff, 0x7f]).len(), 2);
        assert_eq!(pcm16_to_f32(&[0, 0, 1]).len(), 1);
        assert!((1..=64).contains(&inference_threads()));
    }
}
