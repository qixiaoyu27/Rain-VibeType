use reqwest::{blocking::Client, header::RANGE, StatusCode, Url};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
use zip::ZipArchive;

const MODELSCOPE_USER_AGENT: &str = concat!("RainVibetype/", env!("CARGO_PKG_VERSION"));

fn manifest_is_unpublished(status: StatusCode) -> bool {
    status == StatusCode::NOT_FOUND
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ModelManifest {
    pub schema_version: u32,
    pub manifest_version: String,
    pub models: Vec<ModelDefinition>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ModelDefinition {
    pub id: String,
    pub display_name: String,
    pub engine: String,
    pub adapter_type: String,
    #[serde(default = "default_model_purpose")]
    pub purpose: String,
    #[serde(default)]
    pub runtime: ModelRuntimeDependency,
    pub repository_id: String,
    pub revision: String,
    pub model_version: String,
    pub languages: Vec<String>,
    pub download_size: u64,
    pub installed_size: u64,
    pub license: String,
    pub official_source: String,
    pub mirror_source: Option<String>,
    pub recommended_hardware: String,
    pub speed_grade: String,
    pub adapter_compatibility: String,
    pub files: Vec<ModelFile>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ModelRuntimeDependency {
    pub repository: String,
    pub components: BTreeMap<String, String>,
}

impl ModelRuntimeDependency {
    pub fn component_for(&self, accelerator: &str) -> Option<&str> {
        self.components
            .get(accelerator)
            .or_else(|| self.components.get("cpu"))
            .map(String::as_str)
    }

    pub fn uses(&self, component_id: &str) -> bool {
        self.components.values().any(|id| id == component_id)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ModelFile {
    pub path: String,
    pub size: u64,
    pub sha256: String,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ModelCard {
    #[serde(flatten)]
    pub definition: ModelDefinition,
    pub state: String,
    pub installed_path: Option<String>,
    pub previous_versions: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DownloadProgress {
    pub model_id: String,
    pub downloaded: u64,
    pub total: u64,
    pub file: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ImportResult {
    pub model_id: String,
    pub model_path: String,
    pub verified: bool,
    pub warning: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct InstallMarker {
    schema_version: u32,
    manifest_version: String,
    model_id: String,
    model_version: String,
    verified: bool,
    #[serde(default)]
    definition: Option<ModelDefinition>,
}

pub struct ModelRepository {
    manifest: ModelManifest,
    root: PathBuf,
}

impl ModelRepository {
    pub fn new(root: PathBuf) -> Result<Self, String> {
        let embedded: ModelManifest =
            serde_json::from_str(include_str!("../resources/models.json"))
                .map_err(|error| format!("模型清单无效：{error}"))?;
        validate_manifest(&embedded)?;
        let mut manifest = fs::read(root.join(".models-manifest.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<ModelManifest>(&bytes).ok())
            .filter(|manifest| validate_manifest(manifest).is_ok())
            .filter(|manifest| manifest.manifest_version >= embedded.manifest_version)
            .unwrap_or_else(|| embedded.clone());
        for model in embedded.models {
            if !manifest.models.iter().any(|current| current.id == model.id) {
                manifest.models.push(model);
            }
        }
        Ok(Self { manifest, root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn definition(&self, model_id: &str) -> Result<&ModelDefinition, String> {
        self.manifest
            .models
            .iter()
            .find(|model| model.id == model_id)
            .ok_or_else(|| format!("未知模型：{model_id}"))
    }

    pub fn manifest_version(&self) -> &str {
        &self.manifest.manifest_version
    }

    pub fn refresh_manifest(&self, endpoint: &str) -> Result<bool, String> {
        let url = Url::parse(endpoint).map_err(|error| format!("模型清单地址无效：{error}"))?;
        if url.scheme() != "https" {
            return Err("模型清单地址必须使用 HTTPS".into());
        }
        let response = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(20))
            .timeout(std::time::Duration::from_secs(60))
            .user_agent(MODELSCOPE_USER_AGENT)
            .build()
            .map_err(|error| error.to_string())?
            .get(url)
            .send()
            .map_err(|error| format!("无法检查模型更新：{error}"))?;
        if manifest_is_unpublished(response.status()) {
            return Ok(false);
        }
        let response = response
            .error_for_status()
            .map_err(|error| format!("无法检查模型更新：{error}"))?;
        if response
            .content_length()
            .is_some_and(|size| size > 2 * 1024 * 1024)
        {
            return Err("远程模型清单过大".into());
        }
        let bytes = response
            .bytes()
            .map_err(|error| format!("无法读取模型清单：{error}"))?;
        if bytes.len() > 2 * 1024 * 1024 {
            return Err("远程模型清单过大".into());
        }
        let manifest: ModelManifest =
            serde_json::from_slice(&bytes).map_err(|error| format!("远程模型清单无效：{error}"))?;
        validate_manifest(&manifest)?;
        if manifest.manifest_version == self.manifest.manifest_version {
            return Ok(false);
        }
        self.persist_current_definitions()?;
        fs::create_dir_all(&self.root).map_err(|error| error.to_string())?;
        let target = self.root.join(".models-manifest.json");
        let temporary = self
            .root
            .join(format!(".models-manifest.tmp-{}", std::process::id()));
        fs::write(
            &temporary,
            serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("无法保存模型清单：{error}"))?;
        crate::config::replace_file(&temporary, &target).map_err(|error| {
            let _ = fs::remove_file(&temporary);
            format!("无法启用模型清单：{error}")
        })?;
        Ok(true)
    }

    pub fn list(&self) -> Vec<ModelCard> {
        self.manifest
            .models
            .iter()
            .cloned()
            .map(|definition| {
                let path = self.install_path(&definition);
                let installed = path.join(".rain-model.json").is_file();
                let custom_path = self.custom_path(&definition);
                let custom = custom_path.join(".rain-model.json").is_file();
                let previous = self.previous_installations(&definition);
                let previous_versions = previous
                    .iter()
                    .map(|(_, marker)| marker.model_version.clone())
                    .collect::<Vec<_>>();
                ModelCard {
                    definition,
                    state: if custom {
                        "custom"
                    } else if installed {
                        "installed"
                    } else if !previous.is_empty() {
                        "update_available"
                    } else {
                        "not_installed"
                    }
                    .into(),
                    installed_path: if custom {
                        Some(custom_path.to_string_lossy().into_owned())
                    } else if installed {
                        Some(path.to_string_lossy().into_owned())
                    } else if let Some((previous_path, _)) = previous.last() {
                        Some(previous_path.to_string_lossy().into_owned())
                    } else {
                        None
                    },
                    previous_versions,
                }
            })
            .collect()
    }

    pub fn models_using_runtime(&self, component_id: &str) -> Vec<String> {
        self.list()
            .into_iter()
            .filter_map(|card| {
                if !matches!(
                    card.state.as_str(),
                    "installed" | "custom" | "update_available"
                ) {
                    return None;
                }
                let installed_definition = card
                    .installed_path
                    .as_deref()
                    .and_then(|path| read_marker(Path::new(path)))
                    .and_then(|marker| marker.definition)
                    .filter(|definition| !definition.runtime.components.is_empty());
                installed_definition
                    .as_ref()
                    .unwrap_or(&card.definition)
                    .runtime
                    .uses(component_id)
                    .then_some(card.definition.id)
            })
            .collect()
    }

    pub fn installed_path(&self, model_id: &str) -> Result<PathBuf, String> {
        let model = self.definition(model_id)?;
        let path = self.install_path(model);
        if self.custom_path(model).join(".rain-model.json").is_file() {
            Ok(self.custom_path(model))
        } else if path.join(".rain-model.json").is_file() {
            Ok(path)
        } else if let Some((path, _)) = self.previous_installations(model).pop() {
            Ok(path)
        } else {
            Err("MODEL_NOT_INSTALLED：模型尚未安装".into())
        }
    }

    pub fn verify(&self, model_id: &str) -> Result<PathBuf, String> {
        let model = self.definition(model_id)?;
        let path = self.installed_path(model_id)?;
        verify_installed_path(&path, model)?;
        Ok(path)
    }

    pub fn validate_loadable(&self, model_id: &str, path: &Path) -> Result<(), String> {
        let model = self.definition(model_id)?;
        let marker = read_marker(path)
            .ok_or("MODEL_INTEGRITY_FAILED：模型缺少或包含无效的 Rain 安装标记")?;
        if marker.model_id != model_id {
            return Err("MODEL_INTEGRITY_FAILED：模型类型与安装标记不一致".into());
        }
        let root = fs::canonicalize(&self.root).map_err(|error| error.to_string())?;
        let path = fs::canonicalize(path).map_err(|error| error.to_string())?;
        if !path.starts_with(&root) {
            return Err("MODEL_INTEGRITY_FAILED：拒绝加载应用管理目录以外的模型".into());
        }
        validate_installed_sizes(&path, model, &marker)?;
        Ok(())
    }

    pub fn download(
        &self,
        model_id: &str,
        paused: &AtomicBool,
        mut progress: impl FnMut(DownloadProgress),
    ) -> Result<PathBuf, String> {
        let model = self.definition(model_id)?.clone();
        let final_dir = self.install_path(&model);
        if final_dir.join(".rain-model.json").is_file() && verify_files(&final_dir, &model).is_ok()
        {
            return Ok(final_dir);
        }

        let staging = final_dir.with_extension("incomplete");
        fs::create_dir_all(&staging).map_err(|error| format!("无法创建模型目录：{error}"))?;
        let remaining = remaining_bytes(&staging, &model);
        if let Some(free) = crate::platform_windows::free_disk_space(&staging) {
            if free < remaining.saturating_add(256 * 1024 * 1024) {
                return Err(format!(
                    "磁盘空间不足：还需要至少 {:.1} GB",
                    remaining as f64 / 1_073_741_824.0
                ));
            }
        }

        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(20))
            .user_agent(MODELSCOPE_USER_AGENT)
            .build()
            .map_err(|error| format!("无法初始化下载器：{error}"))?;
        let mut downloaded = model.download_size.saturating_sub(remaining);
        for file in &model.files {
            if paused.load(Ordering::Relaxed) {
                return Err("DOWNLOAD_PAUSED：下载已暂停".into());
            }
            let target = staging.join(path_from_manifest(&file.path)?);
            if target.is_file()
                && target.metadata().map(|value| value.len()).unwrap_or(0) == file.size
                && hash_file(&target)? == file.sha256
            {
                progress(DownloadProgress {
                    model_id: model.id.clone(),
                    downloaded,
                    total: model.download_size,
                    file: file.path.clone(),
                });
                continue;
            }
            if target.exists() {
                fs::remove_file(&target).map_err(|error| error.to_string())?;
            }
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            let part = target.with_extension(format!(
                "{}part",
                target
                    .extension()
                    .map(|value| format!("{}.", value.to_string_lossy()))
                    .unwrap_or_default()
            ));
            let before = part
                .metadata()
                .map(|value| value.len())
                .unwrap_or(0)
                .min(file.size);
            downloaded = downloaded.saturating_sub(before);
            download_file(&client, &model, file, &part, paused, |current| {
                progress(DownloadProgress {
                    model_id: model.id.clone(),
                    downloaded: downloaded + current,
                    total: model.download_size,
                    file: file.path.clone(),
                })
            })?;
            if hash_file(&part)? != file.sha256 {
                let _ = fs::remove_file(&part);
                return Err(format!("MODEL_INTEGRITY_FAILED：{} 校验失败", file.path));
            }
            fs::rename(&part, &target).map_err(|error| format!("无法完成模型文件：{error}"))?;
            downloaded += file.size;
        }
        verify_files(&staging, &model)?;
        let marker = InstallMarker {
            schema_version: 1,
            manifest_version: self.manifest.manifest_version.clone(),
            model_id: model.id.clone(),
            model_version: model.model_version.clone(),
            verified: true,
            definition: Some(model.clone()),
        };
        fs::write(
            staging.join(".rain-model.json"),
            serde_json::to_vec_pretty(&marker).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("无法写入模型状态：{error}"))?;
        if final_dir.exists() {
            safe_remove_dir(&self.root, &final_dir)?;
        }
        fs::rename(&staging, &final_dir).map_err(|error| format!("无法启用已下载模型：{error}"))?;
        Ok(final_dir)
    }

    pub fn delete(&self, model_id: &str) -> Result<(), String> {
        let model = self.definition(model_id)?;
        let path = self.install_path(model);
        if path.exists() {
            safe_remove_dir(&self.root, &path)?;
        }
        let staging = path.with_extension("incomplete");
        if staging.exists() {
            safe_remove_dir(&self.root, &staging)?;
        }
        let custom = self.custom_path(model);
        if custom.exists() {
            safe_remove_dir(&self.root, &custom)?;
        }
        let parent = self.root.join(&model.id);
        if parent.exists() {
            cleanup_import_staging(&self.root, &parent)?;
            for (path, _) in self.previous_installations(model) {
                safe_remove_dir(&self.root, &path)?;
            }
        }
        Ok(())
    }

    pub fn delete_previous_versions(&self, model_id: &str) -> Result<Vec<String>, String> {
        let model = self.definition(model_id)?;
        let previous = self.previous_installations(model);
        let versions = previous
            .iter()
            .map(|(_, marker)| marker.model_version.clone())
            .collect::<Vec<_>>();
        for (path, _) in previous {
            safe_remove_dir(&self.root, &path)?;
        }
        Ok(versions)
    }

    pub fn import(&self, model_id: &str, source: &Path) -> Result<ImportResult, String> {
        let model = self.definition(model_id)?.clone();
        if !source.exists() {
            return Err("导入路径不存在".into());
        }
        let parent = self.root.join(&model.id);
        fs::create_dir_all(&parent).map_err(|error| format!("无法创建模型目录：{error}"))?;
        cleanup_import_staging(&self.root, &parent)?;
        let staging = parent.join(format!("import-{}.incomplete", uuid::Uuid::new_v4()));
        if source.is_dir() {
            copy_directory(source, &staging)?;
        } else if source
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
        {
            extract_zip(source, &staging, model.installed_size.saturating_mul(2))?;
        } else {
            return Err("仅支持模型目录或 ZIP 压缩包".into());
        }

        let located = locate_model_root(&staging, &model)?;
        if located != staging {
            let normalized = parent.join(format!("normalized-{}.incomplete", uuid::Uuid::new_v4()));
            copy_directory(&located, &normalized)?;
            safe_remove_dir(&self.root, &staging)?;
            fs::rename(&normalized, &staging).map_err(|error| error.to_string())?;
        }
        required_structure(&staging, &model)?;
        let verified = verify_files(&staging, &model).is_ok();
        let final_dir = if verified {
            self.install_path(&model)
        } else {
            self.custom_path(&model)
        };
        let marker = InstallMarker {
            schema_version: 1,
            manifest_version: self.manifest.manifest_version.clone(),
            model_id: model.id.clone(),
            model_version: if verified {
                model.model_version.clone()
            } else {
                "custom".into()
            },
            verified,
            definition: verified.then(|| model.clone()),
        };
        fs::write(
            staging.join(".rain-model.json"),
            serde_json::to_vec_pretty(&marker).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        if final_dir.exists() {
            safe_remove_dir(&self.root, &final_dir)?;
        }
        fs::rename(&staging, &final_dir).map_err(|error| format!("无法完成模型导入：{error}"))?;
        Ok(ImportResult {
            model_id: model.id,
            model_path: final_dir.to_string_lossy().into_owned(),
            verified,
            warning: (!verified)
                .then(|| "模型版本不在官方清单中，已标记为未经验证的本地自定义模型".into()),
        })
    }

    pub fn delete_managed_path(&self, path: &Path) -> Result<(), String> {
        if !path.exists() {
            return Ok(());
        }
        if !path.join(".rain-model.json").is_file() {
            return Err("拒绝删除没有 Rain 模型标记的目录".into());
        }
        safe_remove_dir(&self.root, path)
    }

    fn install_path(&self, model: &ModelDefinition) -> PathBuf {
        self.root.join(&model.id).join(&model.model_version)
    }

    fn custom_path(&self, model: &ModelDefinition) -> PathBuf {
        self.root.join(&model.id).join("custom-current")
    }

    fn previous_installations(&self, model: &ModelDefinition) -> Vec<(PathBuf, InstallMarker)> {
        let current = self.install_path(model);
        let custom = self.custom_path(model);
        let Ok(entries) = fs::read_dir(self.root.join(&model.id)) else {
            return Vec::new();
        };
        let mut installations = entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let path = entry.path();
                let file_type = entry.file_type().ok()?;
                if !file_type.is_dir()
                    || file_type.is_symlink()
                    || path == current
                    || path == custom
                {
                    return None;
                }
                let marker = read_marker(&path)?;
                (marker.model_id == model.id && marker.verified).then_some((path, marker))
            })
            .collect::<Vec<_>>();
        installations.sort_by(|left, right| left.1.model_version.cmp(&right.1.model_version));
        installations
    }

    fn persist_current_definitions(&self) -> Result<(), String> {
        for model in &self.manifest.models {
            let path = self.install_path(model);
            let Some(mut marker) = read_marker(&path) else {
                continue;
            };
            if marker.verified && marker.definition.is_none() {
                marker.definition = Some(model.clone());
                let target = path.join(".rain-model.json");
                let temporary = path.join(format!(".rain-model.tmp-{}", std::process::id()));
                fs::write(
                    &temporary,
                    serde_json::to_vec_pretty(&marker).map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?;
                crate::config::replace_file(&temporary, &target).map_err(|error| {
                    let _ = fs::remove_file(&temporary);
                    error.to_string()
                })?;
            }
        }
        Ok(())
    }
}

fn validate_manifest(manifest: &ModelManifest) -> Result<(), String> {
    if manifest.schema_version != 2 || manifest.models.len() < 3 {
        return Err("模型清单必须包含基础语音模型且使用 schema_version 2".into());
    }
    let mut ids = std::collections::HashSet::new();
    for model in &manifest.models {
        if !ids.insert(model.id.as_str())
            || model.id.is_empty()
            || model
                .id
                .chars()
                .any(|value| !(value.is_ascii_alphanumeric() || value == '-' || value == '_'))
        {
            return Err(format!("模型清单包含无效或重复 ID：{}", model.id));
        }
        let expected = match model.id.as_str() {
            "sensevoice-small" => Some(("sensevoice", "asr")),
            "fun-asr-nano" => Some(("fun_asr_nano", "asr")),
            "paraformer-zh" => Some(("paraformer_zh", "asr")),
            "streaming-zipformer-preview" => Some(("streaming_zipformer", "asr_preview")),
            "qwen3-0-6b-text" => Some(("text_polish", "text_polish")),
            _ => None,
        };
        if model.adapter_type.trim().is_empty()
            || !matches!(
                model.purpose.as_str(),
                "asr" | "asr_preview" | "text_polish"
            )
            || model.files.is_empty()
            || expected.is_some_and(|(adapter, purpose)| {
                model.adapter_type != adapter || model.purpose != purpose
            })
        {
            return Err(format!("模型 {} 的适配器或文件清单无效", model.id));
        }
        if !matches!(model.runtime.repository.as_str(), "speech" | "text")
            || model.runtime.components.is_empty()
            || model
                .runtime
                .components
                .iter()
                .any(|(accelerator, component_id)| {
                    !matches!(accelerator.as_str(), "cpu" | "nvidia")
                        || !safe_identifier(component_id)
                })
        {
            return Err(format!("模型 {} 的推理组件映射无效", model.id));
        }
        for file in &model.files {
            path_from_manifest(&file.path)?;
            if file.url.as_deref().is_some_and(|url| {
                Url::parse(url)
                    .map(|url| url.scheme() != "https")
                    .unwrap_or(true)
            }) {
                return Err(format!("模型 {} 包含无效的文件下载地址", model.id));
            }
            if file.size == 0
                || file.sha256.len() != 64
                || !file.sha256.chars().all(|value| value.is_ascii_hexdigit())
            {
                return Err(format!("模型 {} 包含无效的文件哈希或大小", model.id));
            }
        }
    }
    for required in ["sensevoice-small", "fun-asr-nano", "paraformer-zh"] {
        if !ids.contains(required) {
            return Err(format!("模型清单缺少基础语音模型：{required}"));
        }
    }
    Ok(())
}

fn default_model_purpose() -> String {
    "asr".into()
}

fn safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

fn read_marker(path: &Path) -> Option<InstallMarker> {
    let bytes = fs::read(path.join(".rain-model.json")).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn marker_definition<'a>(
    marker: &'a InstallMarker,
    current: &'a ModelDefinition,
) -> Result<&'a ModelDefinition, String> {
    if !marker.verified {
        return Ok(current);
    }
    if let Some(definition) = marker.definition.as_ref() {
        if definition.id != current.id || definition.adapter_type != current.adapter_type {
            return Err("MODEL_INTEGRITY_FAILED：已安装模型与当前适配器不兼容".into());
        }
        return Ok(definition);
    }
    if marker.model_version == current.model_version {
        Ok(current)
    } else {
        Err("MODEL_INTEGRITY_FAILED：旧模型缺少版本清单，请重新导入或下载".into())
    }
}

fn verify_installed_path(path: &Path, current: &ModelDefinition) -> Result<(), String> {
    let marker = read_marker(path).ok_or("MODEL_INTEGRITY_FAILED：模型安装标记无效")?;
    if marker.model_id != current.id {
        return Err("MODEL_INTEGRITY_FAILED：模型类型与安装标记不一致".into());
    }
    let definition = marker_definition(&marker, current)?;
    if marker.verified {
        verify_files(path, definition)
    } else {
        required_structure(path, definition)
    }
}

fn validate_installed_sizes(
    path: &Path,
    current: &ModelDefinition,
    marker: &InstallMarker,
) -> Result<(), String> {
    let definition = marker_definition(marker, current)?;
    required_structure(path, definition)?;
    if marker.verified {
        for file in &definition.files {
            let file_path = path.join(path_from_manifest(&file.path)?);
            if file_path.metadata().map(|value| value.len()).ok() != Some(file.size) {
                return Err(format!(
                    "MODEL_INTEGRITY_FAILED：{} 大小不符，请重新校验或下载",
                    file.path
                ));
            }
        }
    }
    Ok(())
}

fn path_from_manifest(value: &str) -> Result<PathBuf, String> {
    let path = Path::new(value);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::Prefix(_)
            )
        })
    {
        return Err(format!("模型清单包含不安全路径：{value}"));
    }
    Ok(path.to_owned())
}

fn remaining_bytes(directory: &Path, model: &ModelDefinition) -> u64 {
    model
        .files
        .iter()
        .map(|file| {
            let target = directory.join(&file.path);
            if target.metadata().map(|value| value.len()).ok() == Some(file.size) {
                return 0;
            }
            let part = target.with_extension(format!(
                "{}part",
                target
                    .extension()
                    .map(|value| format!("{}.", value.to_string_lossy()))
                    .unwrap_or_default()
            ));
            file.size
                .saturating_sub(part.metadata().map(|value| value.len()).unwrap_or(0))
        })
        .sum()
}

fn download_file(
    client: &Client,
    model: &ModelDefinition,
    file: &ModelFile,
    part: &Path,
    paused: &AtomicBool,
    mut progress: impl FnMut(u64),
) -> Result<(), String> {
    let mut existing = part.metadata().map(|value| value.len()).unwrap_or(0);
    if existing > file.size {
        fs::remove_file(part).map_err(|error| error.to_string())?;
        existing = 0;
    }
    let url = if let Some(url) = &file.url {
        Url::parse(url).map_err(|error| error.to_string())?
    } else {
        let mut url = Url::parse(&format!(
            "https://modelscope.cn/api/v1/models/{}/repo",
            model.repository_id
        ))
        .map_err(|error| error.to_string())?;
        url.query_pairs_mut()
            .append_pair("Revision", &model.revision)
            .append_pair("FilePath", &file.path);
        url
    };
    let mut request = client.get(url);
    if existing > 0 {
        request = request.header(RANGE, format!("bytes={existing}-"));
    }
    let mut response = request
        .send()
        .map_err(|error| format!("模型下载失败：{error}"))?;
    if !response.status().is_success() {
        return Err(format!("模型下载失败：HTTP {}", response.status()));
    }
    let resumed = response.status() == StatusCode::PARTIAL_CONTENT;
    let mut output = if resumed {
        OpenOptions::new().create(true).append(true).open(part)
    } else {
        existing = 0;
        File::create(part)
    }
    .map_err(|error| format!("无法写入模型文件：{error}"))?;
    let mut buffer = vec![0u8; 1024 * 1024];
    let mut current = existing;
    loop {
        if paused.load(Ordering::Relaxed) {
            output.flush().map_err(|error| error.to_string())?;
            return Err("DOWNLOAD_PAUSED：下载已暂停".into());
        }
        let count = response
            .read(&mut buffer)
            .map_err(|error| format!("读取模型下载失败：{error}"))?;
        if count == 0 {
            break;
        }
        output
            .write_all(&buffer[..count])
            .map_err(|error| format!("写入模型下载失败：{error}"))?;
        current += count as u64;
        progress(current);
    }
    output.flush().map_err(|error| error.to_string())?;
    if current != file.size {
        return Err(format!(
            "模型文件大小不符：{}（期望 {}，实际 {}）",
            file.path, file.size, current
        ));
    }
    Ok(())
}

fn verify_files(directory: &Path, model: &ModelDefinition) -> Result<(), String> {
    for file in &model.files {
        let path = directory.join(path_from_manifest(&file.path)?);
        if path.metadata().map(|value| value.len()).ok() != Some(file.size)
            || hash_file(&path)? != file.sha256
        {
            return Err(format!("MODEL_INTEGRITY_FAILED：{} 缺失或损坏", file.path));
        }
    }
    Ok(())
}

fn required_structure(directory: &Path, model: &ModelDefinition) -> Result<(), String> {
    for file in &model.files {
        if !directory.join(path_from_manifest(&file.path)?).is_file() {
            return Err(format!(
                "MODEL_INTEGRITY_FAILED：缺少必需文件 {}",
                file.path
            ));
        }
    }
    Ok(())
}

fn locate_model_root(directory: &Path, model: &ModelDefinition) -> Result<PathBuf, String> {
    if required_structure(directory, model).is_ok() {
        return Ok(directory.to_owned());
    }
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.is_dir() && required_structure(&path, model).is_ok() {
            return Ok(path);
        }
    }
    Err("MODEL_INTEGRITY_FAILED：导入内容不符合所选模型的目录结构".into())
}

fn copy_directory(source: &Path, target: &Path) -> Result<(), String> {
    fs::create_dir_all(target).map_err(|error| error.to_string())?;
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_symlink() {
            return Err("模型目录不能包含符号链接".into());
        }
        let destination = target.join(entry.file_name());
        if file_type.is_dir() {
            copy_directory(&entry.path(), &destination)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), destination).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn extract_zip(source: &Path, target: &Path, maximum_size: u64) -> Result<(), String> {
    fs::create_dir_all(target).map_err(|error| error.to_string())?;
    let file = File::open(source).map_err(|error| format!("无法打开模型压缩包：{error}"))?;
    let mut archive = ZipArchive::new(file).map_err(|error| format!("模型压缩包无效：{error}"))?;
    let mut total = 0u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        total = total.saturating_add(entry.size());
        if total > maximum_size.max(4 * 1024 * 1024 * 1024) {
            return Err("模型压缩包解压后体积异常".into());
        }
        let enclosed = entry
            .enclosed_name()
            .ok_or("模型压缩包包含不安全路径")?
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
        return Err("拒绝删除应用模型目录以外的路径".into());
    }
    fs::remove_dir_all(target).map_err(|error| format!("无法删除模型：{error}"))
}

fn cleanup_import_staging(root: &Path, parent: &Path) -> Result<(), String> {
    for entry in fs::read_dir(parent).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if path.is_dir()
            && name.ends_with(".incomplete")
            && (name.starts_with("import-") || name.starts_with("normalized-"))
        {
            safe_remove_dir(root, &path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_model(hash: &str) -> ModelDefinition {
        ModelDefinition {
            id: "tiny".into(),
            display_name: "Tiny".into(),
            engine: "test".into(),
            adapter_type: "test".into(),
            purpose: "asr".into(),
            runtime: ModelRuntimeDependency {
                repository: "speech".into(),
                components: BTreeMap::from([("cpu".into(), "test-runtime".into())]),
            },
            repository_id: "test/tiny".into(),
            revision: "main".into(),
            model_version: "1".into(),
            languages: vec!["en".into()],
            download_size: 10,
            installed_size: 10,
            license: "test".into(),
            official_source: "https://example.invalid".into(),
            mirror_source: None,
            recommended_hardware: "test".into(),
            speed_grade: "test".into(),
            adapter_compatibility: "1".into(),
            files: vec![ModelFile {
                path: "weights.bin".into(),
                size: 10,
                sha256: hash.into(),
                url: None,
            }],
        }
    }

    #[test]
    fn manifest_paths_cannot_escape_model_root() {
        assert!(path_from_manifest("config.yaml").is_ok());
        assert!(path_from_manifest("Qwen/tokenizer.json").is_ok());
        assert!(path_from_manifest("../outside").is_err());
        assert!(path_from_manifest("C:\\outside").is_err());
    }

    #[test]
    fn missing_release_manifest_is_not_a_model_update_error() {
        assert!(manifest_is_unpublished(StatusCode::NOT_FOUND));
        assert!(!manifest_is_unpublished(StatusCode::INTERNAL_SERVER_ERROR));
    }

    #[test]
    fn embedded_manifest_is_versioned_and_has_optional_text_model() {
        let repository = ModelRepository::new(PathBuf::from("models")).unwrap();
        assert_eq!(repository.manifest.schema_version, 2);
        assert_eq!(repository.manifest.models.len(), 5);
        assert_eq!(
            repository.definition("qwen3-0-6b-text").unwrap().purpose,
            "text_polish"
        );
        let sensevoice = repository.definition("sensevoice-small").unwrap();
        assert_eq!(sensevoice.engine, "sherpa-onnx");
        assert_eq!(
            sensevoice
                .files
                .iter()
                .map(|file| file.path.as_str())
                .collect::<Vec<_>>(),
            ["model.onnx", "tokens.txt"]
        );
        assert_eq!(
            repository
                .definition("streaming-zipformer-preview")
                .unwrap()
                .purpose,
            "asr_preview"
        );
        assert!(repository
            .manifest
            .models
            .iter()
            .all(|model| !model.files.is_empty()
                && model.files.iter().all(|file| file.sha256.len() == 64)));
        assert_eq!(
            repository
                .definition("sensevoice-small")
                .unwrap()
                .runtime
                .component_for("nvidia"),
            Some("rain-runtime-onnx-cpu")
        );
        for model_id in ["fun-asr-nano", "paraformer-zh"] {
            assert_eq!(
                repository
                    .definition(model_id)
                    .unwrap()
                    .runtime
                    .component_for("cpu"),
                Some("rain-runtime-cpu")
            );
            assert_eq!(
                repository
                    .definition(model_id)
                    .unwrap()
                    .runtime
                    .component_for("nvidia"),
                Some("rain-runtime-nvidia")
            );
        }
    }

    #[test]
    fn shared_runtime_stays_referenced_until_the_last_model_is_deleted() {
        let root = std::env::temp_dir().join(format!("rain-runtime-refs-{}", uuid::Uuid::new_v4()));
        let repository = ModelRepository::new(root.clone()).unwrap();
        for model_id in ["fun-asr-nano", "paraformer-zh"] {
            let definition = repository.definition(model_id).unwrap().clone();
            let path = repository.install_path(&definition);
            fs::create_dir_all(&path).unwrap();
            let marker = InstallMarker {
                schema_version: 1,
                manifest_version: repository.manifest.manifest_version.clone(),
                model_id: definition.id.clone(),
                model_version: definition.model_version.clone(),
                verified: true,
                definition: Some(definition),
            };
            fs::write(
                path.join(".rain-model.json"),
                serde_json::to_vec(&marker).unwrap(),
            )
            .unwrap();
        }

        assert_eq!(
            repository.models_using_runtime("rain-runtime-cpu"),
            ["fun-asr-nano", "paraformer-zh"]
        );
        repository.delete("fun-asr-nano").unwrap();
        assert_eq!(
            repository.models_using_runtime("rain-runtime-cpu"),
            ["paraformer-zh"]
        );
        repository.delete("paraformer-zh").unwrap();
        assert!(repository
            .models_using_runtime("rain-runtime-cpu")
            .is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn partial_download_bytes_are_counted_for_resume() {
        let directory = std::env::temp_dir().join(format!("rain-model-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("weights.bin.part"), [1_u8; 4]).unwrap();
        assert_eq!(remaining_bytes(&directory, &tiny_model(&"0".repeat(64))), 6);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn hash_verification_rejects_corrupted_model_files() {
        let directory = std::env::temp_dir().join(format!("rain-hash-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("weights.bin"), b"0123456789").unwrap();
        let correct = hash_file(&directory.join("weights.bin")).unwrap();
        assert!(verify_files(&directory, &tiny_model(&correct)).is_ok());
        assert!(verify_files(&directory, &tiny_model(&"f".repeat(64))).is_err());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn catalog_update_keeps_old_version_until_explicit_deletion() {
        let root = std::env::temp_dir().join(format!("rain-update-{}", uuid::Uuid::new_v4()));
        let embedded = ModelRepository::new(root.clone()).unwrap();
        let old_definition = embedded.definition("sensevoice-small").unwrap().clone();
        let old_path = root
            .join(&old_definition.id)
            .join(&old_definition.model_version);
        fs::create_dir_all(&old_path).unwrap();
        let marker = InstallMarker {
            schema_version: 1,
            manifest_version: embedded.manifest.manifest_version.clone(),
            model_id: old_definition.id.clone(),
            model_version: old_definition.model_version.clone(),
            verified: true,
            definition: Some(old_definition.clone()),
        };
        fs::write(
            old_path.join(".rain-model.json"),
            serde_json::to_vec(&marker).unwrap(),
        )
        .unwrap();

        let mut next_manifest = embedded.manifest.clone();
        next_manifest.manifest_version = "future-catalog".into();
        let next_definition = next_manifest
            .models
            .iter_mut()
            .find(|model| model.id == old_definition.id)
            .unwrap();
        next_definition.model_version = "future-model".into();
        next_definition.runtime.components =
            BTreeMap::from([("cpu".into(), "future-runtime".into())]);
        fs::write(
            root.join(".models-manifest.json"),
            serde_json::to_vec(&next_manifest).unwrap(),
        )
        .unwrap();

        let updated = ModelRepository::new(root.clone()).unwrap();
        let card = updated
            .list()
            .into_iter()
            .find(|model| model.definition.id == old_definition.id)
            .unwrap();
        assert_eq!(card.state, "update_available");
        assert_eq!(card.previous_versions, vec![old_definition.model_version]);
        assert_eq!(
            updated.installed_path(&card.definition.id).unwrap(),
            old_path
        );
        assert_eq!(
            updated.models_using_runtime("rain-runtime-onnx-cpu"),
            ["sensevoice-small"]
        );
        assert!(updated.models_using_runtime("future-runtime").is_empty());
        updated
            .delete_previous_versions(&card.definition.id)
            .unwrap();
        assert!(!old_path.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    #[ignore = "requires live ModelScope network access"]
    fn live_modelscope_large_file_range_request_works() {
        let repository = ModelRepository::new(PathBuf::from("models")).unwrap();
        let model = repository.definition("fun-asr-nano").unwrap();
        let file = model
            .files
            .iter()
            .find(|file| file.path == "model.pt")
            .unwrap();
        let mut url = Url::parse(&format!(
            "https://modelscope.cn/api/v1/models/{}/repo",
            model.repository_id
        ))
        .unwrap();
        url.query_pairs_mut()
            .append_pair("Revision", &model.revision)
            .append_pair("FilePath", &file.path);
        let response = Client::builder()
            .user_agent(MODELSCOPE_USER_AGENT)
            .build()
            .unwrap()
            .get(url)
            .header(RANGE, "bytes=0-0")
            .send()
            .unwrap();
        let status = response.status();
        let final_url = response.url().clone();
        let headers = response.headers().clone();
        let body = response.text().unwrap_or_default();
        assert!(
            status.is_success(),
            "{status} {final_url}\n{headers:?}\n{body}"
        );
        assert_eq!(body.len(), 1);
    }
}
