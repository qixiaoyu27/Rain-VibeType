use reqwest::{blocking::Client, header::RANGE, StatusCode, Url};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    sync::OnceLock,
    time::Duration,
};
use zip::ZipArchive;

const USER_AGENT: &str = concat!("RainVibetype/", env!("CARGO_PKG_VERSION"));
const MANIFEST_FILE: &str = ".runtime-manifest.json";
const MARKER_FILE: &str = ".rain-runtime.json";
pub const NATIVE_SENSEVOICE_COMPONENT: &str = "rain-runtime-onnx-cpu";

fn manifest_is_unpublished(status: StatusCode) -> bool {
    status == StatusCode::NOT_FOUND
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RuntimeManifest {
    pub schema_version: u32,
    pub manifest_version: String,
    pub components: Vec<RuntimeComponent>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RuntimeComponent {
    pub id: String,
    pub display_name: String,
    pub version: String,
    pub accelerator: String,
    pub url: String,
    pub archive_size: u64,
    pub installed_size: u64,
    pub sha256: String,
    pub executable: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct RuntimeCard {
    #[serde(flatten)]
    pub definition: RuntimeComponent,
    pub installed: bool,
    pub recommended: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct RuntimeStatus {
    pub ready: bool,
    pub source: String,
    pub nvidia_detected: bool,
    pub nvidia_name: Option<String>,
    pub recommended_accelerator: String,
    pub recommended_component_id: Option<String>,
    pub active_component_id: Option<String>,
    pub active_executable: Option<String>,
    pub components: Vec<RuntimeCard>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RuntimeDownloadProgress {
    pub component_id: String,
    pub downloaded: u64,
    pub total: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct InstallMarker {
    schema_version: u32,
    manifest_version: String,
    component: RuntimeComponent,
}

pub struct RuntimeRepository {
    root: PathBuf,
    manifest: Option<RuntimeManifest>,
}

impl RuntimeRepository {
    pub fn new(root: PathBuf) -> Result<Self, String> {
        Self::new_with_fallback(root, None)
    }

    pub fn new_with_embedded(root: PathBuf, manifest_json: &str) -> Result<Self, String> {
        let manifest: RuntimeManifest = serde_json::from_str(manifest_json)
            .map_err(|error| format!("内置推理组件清单无效：{error}"))?;
        validate_manifest(&manifest)?;
        Self::new_with_fallback(root, Some(manifest))
    }

    fn new_with_fallback(root: PathBuf, fallback: Option<RuntimeManifest>) -> Result<Self, String> {
        fs::create_dir_all(&root).map_err(|error| format!("无法创建推理组件目录：{error}"))?;
        let manifest = fs::read(root.join(MANIFEST_FILE))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .filter(|manifest| validate_manifest(manifest).is_ok())
            .or(fallback);
        Ok(Self { root, manifest })
    }

    pub fn refresh_manifest(&mut self, endpoint: &str) -> Result<bool, String> {
        let url = Url::parse(endpoint).map_err(|error| format!("推理组件清单地址无效：{error}"))?;
        if url.scheme() != "https" {
            return Err("推理组件清单必须使用 HTTPS".into());
        }
        let client = download_client()?;
        let response = client
            .get(url)
            .timeout(Duration::from_secs(5))
            .send()
            .map_err(|error| format!("无法下载推理组件清单：{error}"))?;
        if manifest_is_unpublished(response.status()) {
            return Ok(false);
        }
        let mut response = response
            .error_for_status()
            .map_err(|error| format!("无法下载推理组件清单：{error}"))?;
        let mut bytes = Vec::new();
        response
            .by_ref()
            .take(1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("无法读取推理组件清单：{error}"))?;
        if bytes.len() > 1024 * 1024 {
            return Err("推理组件清单超过 1 MB 限制".into());
        }
        let manifest: RuntimeManifest = serde_json::from_slice(&bytes)
            .map_err(|error| format!("推理组件清单格式无效：{error}"))?;
        validate_manifest(&manifest)?;
        let changed = self
            .manifest
            .as_ref()
            .is_none_or(|current| current.manifest_version != manifest.manifest_version);
        let target = self.root.join(MANIFEST_FILE);
        let temporary = self
            .root
            .join(format!("{MANIFEST_FILE}.tmp-{}", std::process::id()));
        fs::write(
            &temporary,
            serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("无法保存推理组件清单：{error}"))?;
        crate::config::replace_file(&temporary, &target).map_err(|error| {
            let _ = fs::remove_file(&temporary);
            format!("无法启用推理组件清单：{error}")
        })?;
        self.manifest = Some(manifest);
        Ok(changed)
    }

    pub fn status_for_component(
        &self,
        preference: &str,
        python_path: &str,
        component_id: Option<&str>,
        allow_python_fallback: bool,
    ) -> RuntimeStatus {
        let (nvidia_detected, nvidia_name) = detect_nvidia();
        let accelerator = preferred_accelerator(preference, nvidia_detected).to_owned();
        let recommended_component = component_id.and_then(|id| {
            self.manifest.as_ref().and_then(|manifest| {
                manifest
                    .components
                    .iter()
                    .find(|component| component.id == id)
            })
        });
        let installed =
            component_id.and_then(|id| self.installed_matching(|component| component.id == id));
        let python_ready = allow_python_fallback && explicit_python(python_path).is_some();
        let active_executable = installed
            .as_ref()
            .map(|(_, executable)| executable.to_string_lossy().into_owned());
        let active_component_id = installed
            .as_ref()
            .map(|(marker, _)| marker.component.id.clone());
        let components = self
            .manifest
            .as_ref()
            .map(|manifest| {
                manifest
                    .components
                    .iter()
                    .cloned()
                    .map(|definition| RuntimeCard {
                        installed: self.component_executable(&definition).is_some(),
                        recommended: recommended_component
                            .is_some_and(|component| component.id == definition.id),
                        definition,
                    })
                    .collect()
            })
            .unwrap_or_default();
        RuntimeStatus {
            ready: installed.is_some() || python_ready,
            source: if installed.is_some() {
                "managed".into()
            } else if python_ready {
                "python".into()
            } else {
                "missing".into()
            },
            nvidia_detected,
            nvidia_name,
            recommended_accelerator: recommended_component
                .map(|component| component.accelerator.clone())
                .unwrap_or(accelerator),
            recommended_component_id: recommended_component.map(|component| component.id.clone()),
            active_component_id,
            active_executable,
            components,
        }
    }

    pub fn component(&self, component_id: &str) -> Option<&RuntimeComponent> {
        self.manifest
            .as_ref()?
            .components
            .iter()
            .find(|component| component.id == component_id)
    }

    pub fn is_installed(&self, component_id: &str) -> bool {
        self.installed_matching(|component| component.id == component_id)
            .is_some()
    }

    pub fn download(
        &self,
        component_id: Option<&str>,
        preference: &str,
        prefer_native_sensevoice: bool,
        mut progress: impl FnMut(RuntimeDownloadProgress),
    ) -> Result<PathBuf, String> {
        let manifest = self
            .manifest
            .as_ref()
            .ok_or("RUNTIME_NOT_CONFIGURED：此版本尚未配置推理组件下载清单")?;
        let (nvidia, _) = detect_nvidia();
        let accelerator = preferred_accelerator(preference, nvidia);
        let component = match component_id.filter(|value| !value.trim().is_empty()) {
            Some(id) => manifest
                .components
                .iter()
                .find(|component| component.id == id)
                .ok_or("RUNTIME_NOT_FOUND：推理组件不在当前清单中")?,
            None => component_for_runtime(manifest, accelerator, prefer_native_sensevoice)
                .ok_or("RUNTIME_NOT_FOUND：当前清单没有适合此设备的推理组件")?,
        };
        if component.id != NATIVE_SENSEVOICE_COMPONENT && component.accelerator != accelerator {
            return Err("RUNTIME_MISMATCH：所选推理组件与当前推理设备设置不一致".into());
        }
        if component.accelerator == "nvidia" && !nvidia {
            return Err("NVIDIA_GPU_NOT_FOUND：没有检测到可用的 NVIDIA 显卡或驱动".into());
        }
        if let Some(executable) = self.component_executable(component) {
            return Ok(executable);
        }

        let downloads = self.root.join(".downloads");
        fs::create_dir_all(&downloads).map_err(|error| format!("无法创建下载目录：{error}"))?;
        let archive = downloads.join(format!("{}-{}.zip.part", component.id, component.version));
        let existing = archive.metadata().map(|value| value.len()).unwrap_or(0);
        let remaining = component
            .archive_size
            .saturating_sub(existing.min(component.archive_size));
        if let Some(free) = crate::platform_windows::free_disk_space(&downloads) {
            let required = remaining
                .saturating_add(component.installed_size)
                .saturating_add(256 * 1024 * 1024);
            if free < required {
                return Err(format!(
                    "磁盘空间不足：推理组件还需要至少 {:.1} GB",
                    required as f64 / 1_073_741_824.0
                ));
            }
        }
        download_archive(component, &archive, &mut progress)?;
        if hash_file(&archive)? != component.sha256 {
            let _ = fs::remove_file(&archive);
            return Err("RUNTIME_INTEGRITY_FAILED：推理组件 SHA-256 校验失败".into());
        }

        let component_root = self.root.join(&component.id);
        fs::create_dir_all(&component_root).map_err(|error| error.to_string())?;
        let final_dir = component_root.join(&component.version);
        let staging = component_root.join(format!("{}.incomplete", component.version));
        if staging.exists() {
            safe_remove_dir(&self.root, &staging)?;
        }
        extract_archive(
            &archive,
            &staging,
            component.installed_size.saturating_mul(2),
        )?;
        let executable = staging.join(relative_path(&component.executable)?);
        if !executable.is_file() {
            safe_remove_dir(&self.root, &staging)?;
            return Err("RUNTIME_INTEGRITY_FAILED：推理组件缺少 Worker 可执行文件".into());
        }
        let marker = InstallMarker {
            schema_version: 1,
            manifest_version: manifest.manifest_version.clone(),
            component: component.clone(),
        };
        fs::write(
            staging.join(MARKER_FILE),
            serde_json::to_vec_pretty(&marker).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("无法写入推理组件状态：{error}"))?;
        if final_dir.exists() {
            safe_remove_dir(&self.root, &final_dir)?;
        }
        fs::rename(&staging, &final_dir).map_err(|error| format!("无法启用推理组件：{error}"))?;
        let _ = fs::remove_file(&archive);
        Ok(final_dir.join(relative_path(&component.executable)?))
    }

    pub fn remove(&self, component_id: &str) -> Result<(), String> {
        if !safe_identifier(component_id) {
            return Err("RUNTIME_NOT_FOUND：推理组件 ID 无效".into());
        }
        let component_root = self.root.join(component_id);
        if component_root.exists() {
            safe_remove_dir(&self.root, &component_root)?;
        }
        let downloads = self.root.join(".downloads");
        if downloads.is_dir() {
            for entry in fs::read_dir(downloads).map_err(|error| error.to_string())? {
                let path = entry.map_err(|error| error.to_string())?.path();
                if path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(&format!("{component_id}-")))
                    && path.is_file()
                {
                    fs::remove_file(path).map_err(|error| error.to_string())?;
                }
            }
        }
        Ok(())
    }

    fn component_executable(&self, component: &RuntimeComponent) -> Option<PathBuf> {
        let directory = self.root.join(&component.id).join(&component.version);
        let marker = read_marker(&directory)?;
        if marker.component.id != component.id || marker.component.version != component.version {
            return None;
        }
        let executable = directory.join(relative_path(&marker.component.executable).ok()?);
        executable.is_file().then_some(executable)
    }

    fn installed_matching(
        &self,
        predicate: impl Fn(&RuntimeComponent) -> bool,
    ) -> Option<(InstallMarker, PathBuf)> {
        let mut matches = Vec::new();
        for component_entry in fs::read_dir(&self.root).ok()?.flatten() {
            let component_path = component_entry.path();
            if !component_path.is_dir()
                || component_path
                    .file_name()
                    .is_some_and(|name| name == ".downloads")
            {
                continue;
            }
            for version_entry in fs::read_dir(component_path).ok()?.flatten() {
                let directory = version_entry.path();
                let Some(marker) = read_marker(&directory) else {
                    continue;
                };
                if !predicate(&marker.component) {
                    continue;
                }
                let Ok(relative) = relative_path(&marker.component.executable) else {
                    continue;
                };
                let executable = directory.join(relative);
                if executable.is_file() {
                    matches.push((marker, executable));
                }
            }
        }
        matches.sort_by(|left, right| left.0.component.version.cmp(&right.0.component.version));
        matches.pop()
    }
}

pub fn explicit_python(configured: &str) -> Option<PathBuf> {
    let path = Path::new(configured);
    (path.is_absolute() && path.is_file()).then(|| path.to_owned())
}

fn preferred_accelerator(preference: &str, nvidia_detected: bool) -> &'static str {
    match preference {
        "cpu" => "cpu",
        "cuda" => "nvidia",
        _ if nvidia_detected => "nvidia",
        _ => "cpu",
    }
}

pub fn selected_accelerator(preference: &str) -> String {
    preferred_accelerator(preference, detect_nvidia().0).to_owned()
}

fn component_for_runtime<'a>(
    manifest: &'a RuntimeManifest,
    accelerator: &str,
    prefer_native_sensevoice: bool,
) -> Option<&'a RuntimeComponent> {
    prefer_native_sensevoice
        .then(|| {
            manifest
                .components
                .iter()
                .find(|component| component.id == NATIVE_SENSEVOICE_COMPONENT)
        })
        .flatten()
        .or_else(|| {
            manifest.components.iter().find(|component| {
                component.accelerator == accelerator && component.id != NATIVE_SENSEVOICE_COMPONENT
            })
        })
}

fn detect_nvidia() -> (bool, Option<String>) {
    static NVIDIA: OnceLock<(bool, Option<String>)> = OnceLock::new();
    NVIDIA.get_or_init(probe_nvidia).clone()
}

fn probe_nvidia() -> (bool, Option<String>) {
    let mut command = Command::new("nvidia-smi");
    command
        .args(["--query-gpu=name", "--format=csv,noheader"])
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let output = command.output();
    let Ok(output) = output else {
        return (false, None);
    };
    if !output.status.success() {
        return (false, None);
    }
    let name = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_owned);
    (name.is_some(), name)
}

fn validate_manifest(manifest: &RuntimeManifest) -> Result<(), String> {
    if manifest.schema_version != 1 || manifest.manifest_version.trim().is_empty() {
        return Err("推理组件清单版本无效".into());
    }
    if manifest.components.is_empty() {
        return Err("推理组件清单为空".into());
    }
    let mut ids = std::collections::HashSet::new();
    for component in &manifest.components {
        if !safe_identifier(&component.id) || !safe_identifier(&component.version) {
            return Err("推理组件清单包含不安全的标识符".into());
        }
        if !ids.insert(component.id.as_str()) {
            return Err("推理组件清单包含重复组件".into());
        }
        if !matches!(component.accelerator.as_str(), "cpu" | "nvidia") {
            return Err("推理组件清单包含未知加速器".into());
        }
        let url = Url::parse(&component.url).map_err(|_| "推理组件下载地址无效")?;
        if url.scheme() != "https" {
            return Err("推理组件下载地址必须使用 HTTPS".into());
        }
        if component.archive_size == 0
            || component.installed_size == 0
            || component.sha256.len() != 64
            || !component
                .sha256
                .bytes()
                .all(|value| value.is_ascii_hexdigit())
        {
            return Err("推理组件大小或 SHA-256 无效".into());
        }
        relative_path(&component.executable)?;
    }
    Ok(())
}

fn safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn relative_path(value: &str) -> Result<PathBuf, String> {
    let path = Path::new(value);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("推理组件清单包含不安全路径".into());
    }
    Ok(path.to_owned())
}

fn download_client() -> Result<Client, String> {
    Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .user_agent(USER_AGENT)
        .build()
        .map_err(|error| format!("无法初始化推理组件下载器：{error}"))
}

fn download_archive(
    component: &RuntimeComponent,
    destination: &Path,
    progress: &mut impl FnMut(RuntimeDownloadProgress),
) -> Result<(), String> {
    let mut existing = destination.metadata().map(|value| value.len()).unwrap_or(0);
    if existing > component.archive_size {
        fs::remove_file(destination).map_err(|error| error.to_string())?;
        existing = 0;
    }
    if existing == component.archive_size {
        progress(RuntimeDownloadProgress {
            component_id: component.id.clone(),
            downloaded: existing,
            total: component.archive_size,
        });
        return Ok(());
    }
    let client = download_client()?;
    let mut request = client.get(&component.url);
    if existing > 0 {
        request = request.header(RANGE, format!("bytes={existing}-"));
    }
    let mut response = request
        .send()
        .map_err(|error| format!("推理组件下载失败：{error}"))?;
    if existing > 0 && response.status() == StatusCode::OK {
        existing = 0;
    } else if existing > 0 && response.status() != StatusCode::PARTIAL_CONTENT {
        return Err(format!(
            "推理组件服务器不支持续传：HTTP {}",
            response.status()
        ));
    } else if existing == 0 && !response.status().is_success() {
        return Err(format!("推理组件下载失败：HTTP {}", response.status()));
    }
    let mut output = OpenOptions::new()
        .create(true)
        .write(true)
        .append(existing > 0)
        .truncate(existing == 0)
        .open(destination)
        .map_err(|error| format!("无法写入推理组件：{error}"))?;
    let mut downloaded = existing;
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let count = response
            .read(&mut buffer)
            .map_err(|error| format!("读取推理组件下载流失败：{error}"))?;
        if count == 0 {
            break;
        }
        output
            .write_all(&buffer[..count])
            .map_err(|error| format!("写入推理组件失败：{error}"))?;
        downloaded = downloaded.saturating_add(count as u64);
        progress(RuntimeDownloadProgress {
            component_id: component.id.clone(),
            downloaded,
            total: component.archive_size,
        });
    }
    output.flush().map_err(|error| error.to_string())?;
    if downloaded != component.archive_size {
        return Err(format!(
            "推理组件下载不完整：应为 {} 字节，实际为 {} 字节",
            component.archive_size, downloaded
        ));
    }
    Ok(())
}

fn extract_archive(source: &Path, target: &Path, maximum_size: u64) -> Result<(), String> {
    fs::create_dir_all(target).map_err(|error| error.to_string())?;
    let file = File::open(source).map_err(|error| format!("无法打开推理组件：{error}"))?;
    let mut archive =
        ZipArchive::new(file).map_err(|error| format!("推理组件压缩包无效：{error}"))?;
    let mut total = 0u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err("推理组件压缩包不能包含符号链接".into());
        }
        total = total.saturating_add(entry.size());
        if total > maximum_size.max(1024 * 1024 * 1024) {
            return Err("推理组件解压后体积异常".into());
        }
        let enclosed = entry
            .enclosed_name()
            .ok_or("推理组件压缩包包含不安全路径")?
            .to_owned();
        let destination = target.join(enclosed);
        if entry.is_dir() {
            fs::create_dir_all(&destination).map_err(|error| error.to_string())?;
            continue;
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let mut output = File::create(destination).map_err(|error| error.to_string())?;
        std::io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn read_marker(directory: &Path) -> Option<InstallMarker> {
    let bytes = fs::read(directory.join(MARKER_FILE)).ok()?;
    let marker: InstallMarker = serde_json::from_slice(&bytes).ok()?;
    (marker.schema_version == 1).then_some(marker)
}

fn hash_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn safe_remove_dir(root: &Path, target: &Path) -> Result<(), String> {
    let root = fs::canonicalize(root).map_err(|error| error.to_string())?;
    let target = fs::canonicalize(target).map_err(|error| error.to_string())?;
    if target == root || !target.starts_with(&root) {
        return Err("拒绝删除 Rain 推理组件目录以外的路径".into());
    }
    fs::remove_dir_all(target).map_err(|error| format!("无法清理推理组件：{error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> RuntimeManifest {
        RuntimeManifest {
            schema_version: 1,
            manifest_version: "2026-07-14".into(),
            components: vec![
                RuntimeComponent {
                    id: NATIVE_SENSEVOICE_COMPONENT.into(),
                    display_name: "Native SenseVoice".into(),
                    version: "1.0.0".into(),
                    accelerator: "cpu".into(),
                    url: "https://example.invalid/native.zip".into(),
                    archive_size: 10,
                    installed_size: 20,
                    sha256: "c".repeat(64),
                    executable: "rain-worker/rain-worker.exe".into(),
                },
                RuntimeComponent {
                    id: "rain-runtime-cpu".into(),
                    display_name: "CPU".into(),
                    version: "1.0.0".into(),
                    accelerator: "cpu".into(),
                    url: "https://example.invalid/cpu.zip".into(),
                    archive_size: 10,
                    installed_size: 20,
                    sha256: "a".repeat(64),
                    executable: "rain-worker/rain-worker.exe".into(),
                },
                RuntimeComponent {
                    id: "rain-runtime-nvidia".into(),
                    display_name: "NVIDIA".into(),
                    version: "1.0.0".into(),
                    accelerator: "nvidia".into(),
                    url: "https://example.invalid/nvidia.zip".into(),
                    archive_size: 10,
                    installed_size: 20,
                    sha256: "b".repeat(64),
                    executable: "rain-worker/rain-worker.exe".into(),
                },
            ],
        }
    }

    #[test]
    fn automatic_selection_prefers_nvidia_only_when_detected() {
        assert_eq!(preferred_accelerator("auto", true), "nvidia");
        assert_eq!(preferred_accelerator("auto", false), "cpu");
        assert_eq!(preferred_accelerator("cpu", true), "cpu");
        assert_eq!(preferred_accelerator("cuda", false), "nvidia");
        assert_eq!(
            component_for_runtime(&manifest(), "nvidia", true)
                .unwrap()
                .id,
            NATIVE_SENSEVOICE_COMPONENT
        );
        assert_eq!(
            component_for_runtime(&manifest(), "nvidia", false)
                .unwrap()
                .id,
            "rain-runtime-nvidia"
        );
    }

    #[test]
    fn manifest_rejects_insecure_urls_and_paths() {
        assert!(validate_manifest(&manifest()).is_ok());
        let mut insecure = manifest();
        insecure.components[0].url = "http://example.invalid/cpu.zip".into();
        assert!(validate_manifest(&insecure).is_err());
        let mut escaping = manifest();
        escaping.components[0].executable = "../rain-worker.exe".into();
        assert!(validate_manifest(&escaping).is_err());
    }

    #[test]
    fn missing_release_manifest_is_not_a_runtime_error() {
        assert!(manifest_is_unpublished(StatusCode::NOT_FOUND));
        assert!(!manifest_is_unpublished(StatusCode::INTERNAL_SERVER_ERROR));
    }

    #[test]
    fn orphan_runtime_can_be_removed_without_a_catalog() {
        let root =
            std::env::temp_dir().join(format!("rain-runtime-remove-{}", uuid::Uuid::new_v4()));
        let component = root.join("orphan-runtime").join("1.0.0");
        fs::create_dir_all(&component).unwrap();
        fs::write(component.join("worker.exe"), b"test").unwrap();
        let repository = RuntimeRepository::new(root.clone()).unwrap();
        repository.remove("orphan-runtime").unwrap();
        assert!(!root.join("orphan-runtime").exists());
        let _ = fs::remove_dir_all(root);
    }
}
