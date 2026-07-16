use reqwest::blocking::Client;
use serde_json::json;
use std::{
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const MAX_INPUT_CHARS: usize = 1_500;

pub struct TextPolishOptions<'a> {
    pub remove_fillers: bool,
    pub paragraphs: bool,
    pub protected_terms: &'a [String],
    pub idle_timeout_seconds: u64,
}

struct ServerProcess {
    child: Child,
    executable: PathBuf,
    model: PathBuf,
    idle_timeout_seconds: u64,
    port: u16,
    api_key: String,
    #[cfg(windows)]
    _job: crate::platform_windows::KillOnDropJob,
}

pub struct TextPolisher {
    client: Client,
    server: Option<ServerProcess>,
}

impl Default for TextPolisher {
    fn default() -> Self {
        Self {
            client: Client::builder()
                .connect_timeout(Duration::from_secs(1))
                .build()
                .unwrap_or_else(|_| Client::new()),
            server: None,
        }
    }
}

impl TextPolisher {
    pub fn polish(
        &mut self,
        executable: &Path,
        model: &Path,
        raw: &str,
        options: TextPolishOptions<'_>,
    ) -> Result<String, String> {
        let raw = raw.trim();
        if raw.is_empty() {
            return Err("TEXT_POLISH_EMPTY：原始文字为空".into());
        }
        if raw.chars().count() > MAX_INPUT_CHARS {
            return Err("TEXT_POLISH_TOO_LONG：文字过长，已保留原始识别结果".into());
        }
        self.ensure_server(executable, model, options.idle_timeout_seconds)?;
        let (port, api_key) = self
            .server
            .as_ref()
            .map(|server| (server.port, server.api_key.clone()))
            .ok_or("TEXT_POLISH_RUNTIME_FAILED：服务未启动")?;

        let system = system_prompt(options.remove_fillers, options.paragraphs);
        let protected = if options.protected_terms.is_empty() {
            "无".to_owned()
        } else {
            options.protected_terms.join("、")
        };
        let maximum_tokens = (raw.chars().count().saturating_mul(2)).clamp(32, 1_024);
        let response = self
            .client
            .post(format!("http://127.0.0.1:{port}/v1/chat/completions"))
            .bearer_auth(api_key)
            .timeout(Duration::from_secs(8))
            .json(&json!({
                "model": "rain-text",
                "messages": [
                    {"role": "system", "content": system},
                    {"role": "user", "content": format!("受保护词：{protected}\n\n原始转写：\n{raw}")}
                ],
                "temperature": 0,
                "seed": 1,
                "stream": false,
                "max_tokens": maximum_tokens,
                "chat_template_kwargs": {"enable_thinking": false}
            }))
            .send()
            .and_then(|response| response.error_for_status())
            .map_err(|error| format!("TEXT_POLISH_REQUEST_FAILED：{error}"))
            .and_then(|response| {
                response
                    .json::<serde_json::Value>()
                    .map_err(|error| format!("TEXT_POLISH_RESPONSE_INVALID：{error}"))
            })
            .and_then(|value| {
                value["choices"][0]["message"]["content"]
                    .as_str()
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
                    .map(str::to_owned)
                    .ok_or_else(|| "TEXT_POLISH_RESPONSE_EMPTY：模型没有返回正文".into())
            });

        let candidate = match response {
            Ok(candidate) => candidate,
            Err(error) => {
                self.stop();
                return Err(error);
            }
        };
        validate_candidate(raw, &candidate, &options)?;
        Ok(candidate)
    }

    pub fn stop(&mut self) {
        if let Some(mut server) = self.server.take() {
            let _ = server.child.kill();
            let _ = server.child.wait();
        }
    }

    fn ensure_server(
        &mut self,
        executable: &Path,
        model: &Path,
        idle_timeout_seconds: u64,
    ) -> Result<(), String> {
        let reusable = self.server.as_mut().is_some_and(|server| {
            server.executable == executable
                && server.model == model
                && server.idle_timeout_seconds == idle_timeout_seconds
                && server.child.try_wait().ok().flatten().is_none()
        });
        if reusable {
            return Ok(());
        }
        self.stop();

        let port = available_port()?;
        let api_key = uuid::Uuid::new_v4().to_string();
        let model_argument = model.to_string_lossy().into_owned();
        let port_argument = port.to_string();
        let idle_argument = idle_timeout_seconds.to_string();
        let mut command = Command::new(executable);
        command
            .args([
                "--model",
                &model_argument,
                "--alias",
                "rain-text",
                "--host",
                "127.0.0.1",
                "--port",
                &port_argument,
                "--ctx-size",
                "2048",
                "--parallel",
                "1",
                "--threads-http",
                "1",
                "--no-webui",
                "--jinja",
                "--api-key",
                &api_key,
                "--sleep-idle-seconds",
                &idle_argument,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        let mut child = command
            .spawn()
            .map_err(|error| format!("TEXT_POLISH_RUNTIME_FAILED：无法启动 llama.cpp：{error}"))?;
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
        let started = Instant::now();
        loop {
            if let Ok(Some(status)) = child.try_wait() {
                return Err(format!(
                    "TEXT_POLISH_RUNTIME_FAILED：llama.cpp 提前退出（{status}）"
                ));
            }
            let ready = self
                .client
                .get(format!("http://127.0.0.1:{port}/health"))
                .bearer_auth(&api_key)
                .timeout(Duration::from_millis(700))
                .send()
                .is_ok_and(|response| response.status().is_success());
            if ready {
                break;
            }
            if started.elapsed() >= Duration::from_secs(30) {
                let _ = child.kill();
                let _ = child.wait();
                return Err("TEXT_POLISH_RUNTIME_TIMEOUT：文本模型加载超过 30 秒".into());
            }
            thread::sleep(Duration::from_millis(150));
        }
        self.server = Some(ServerProcess {
            child,
            executable: executable.to_owned(),
            model: model.to_owned(),
            idle_timeout_seconds,
            port,
            api_key,
            #[cfg(windows)]
            _job: job,
        });
        Ok(())
    }
}

impl Drop for TextPolisher {
    fn drop(&mut self) {
        self.stop();
    }
}

fn available_port() -> Result<u16, String> {
    std::net::TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .map_err(|error| format!("TEXT_POLISH_RUNTIME_FAILED：无法分配本地端口：{error}"))
}

fn system_prompt(remove_fillers: bool, paragraphs: bool) -> String {
    format!(
        "你是本地语音转写整理器。只输出整理后的正文，不要解释、标题、引号或 Markdown。\
         只允许修正标点、空格和分段；不得改写、总结、补充或删除事实，不得改变数字、英文、专名和语序。\
         {}{}",
        if remove_fillers {
            "可以删除明确的嗯、呃、额等口头停顿词。"
        } else {
            "不得删除任何口头词。"
        },
        if paragraphs {
            "较长内容可按语义分段。"
        } else {
            "保持单段输出。"
        }
    )
}

fn validate_candidate(
    raw: &str,
    candidate: &str,
    options: &TextPolishOptions<'_>,
) -> Result<(), String> {
    let raw_len = raw.chars().count();
    let candidate_len = candidate.chars().count();
    if candidate_len == 0 || candidate_len > raw_len.saturating_mul(2).saturating_add(32) {
        return Err("TEXT_POLISH_REJECTED：整理结果长度异常".into());
    }
    let ascii_tokens = protected_ascii_tokens(raw);
    for term in options
        .protected_terms
        .iter()
        .map(String::as_str)
        .chain(ascii_tokens.iter().map(String::as_str))
    {
        if !term.is_empty() && occurrence_count(raw, term) != occurrence_count(candidate, term) {
            return Err(format!("TEXT_POLISH_REJECTED：受保护内容发生变化：{term}"));
        }
    }
    let candidate_signature = content_signature(candidate);
    let raw_signature = if options.remove_fillers {
        content_signature(&remove_fillers(raw))
    } else {
        content_signature(raw)
    };
    if candidate_signature != raw_signature {
        return Err("TEXT_POLISH_REJECTED：正文内容发生变化".into());
    }
    Ok(())
}

fn occurrence_count(text: &str, term: &str) -> usize {
    text.match_indices(term).count()
}

fn content_signature(text: &str) -> String {
    text.chars()
        .filter(|character| character.is_alphanumeric())
        .collect()
}

fn remove_fillers(text: &str) -> String {
    ["嗯", "呃", "额"]
        .into_iter()
        .fold(text.to_owned(), |value, filler| value.replace(filler, ""))
}

fn protected_ascii_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for character in text.chars() {
        let allowed = character.is_ascii_alphanumeric()
            || matches!(
                character,
                '_' | '-' | '.' | '@' | ':' | '+' | '%' | '#' | '/' | '\\'
            );
        if allowed {
            current.push(character);
        } else if !current.is_empty() {
            push_ascii_token(&mut tokens, &mut current);
        }
    }
    if !current.is_empty() {
        push_ascii_token(&mut tokens, &mut current);
    }
    tokens.sort();
    tokens.dedup();
    tokens
}

fn push_ascii_token(tokens: &mut Vec<String>, current: &mut String) {
    let token = current
        .trim_matches(|character| matches!(character, '-' | '.' | ':' | '/' | '\\'))
        .to_owned();
    if token
        .chars()
        .any(|character| character.is_ascii_alphanumeric())
    {
        tokens.push(token);
    }
    current.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(remove_fillers: bool) -> TextPolishOptions<'static> {
        TextPolishOptions {
            remove_fillers,
            paragraphs: true,
            protected_terms: &[],
            idle_timeout_seconds: 600,
        }
    }

    #[test]
    fn accepts_punctuation_and_paragraph_changes() {
        assert!(validate_candidate(
            "今天开会讨论 Rain 2.0 明天发布",
            "今天开会，讨论 Rain 2.0。\n\n明天发布。",
            &options(false)
        )
        .is_ok());
    }

    #[test]
    fn rejects_changed_numbers_and_names() {
        assert!(validate_candidate(
            "RTX 5060 Ti 需要 16GB",
            "RTX 5090 Ti 需要 16GB。",
            &options(false)
        )
        .is_err());
        let protected = vec!["小雨".to_owned()];
        let options = TextPolishOptions {
            protected_terms: &protected,
            ..options(false)
        };
        assert!(validate_candidate("小雨来开会", "小宇来开会。", &options).is_err());
    }

    #[test]
    fn filler_removal_requires_explicit_option() {
        assert!(validate_candidate("嗯今天开会", "今天开会。", &options(false)).is_err());
        assert!(validate_candidate("嗯今天开会", "今天开会。", &options(true)).is_ok());
    }
}
