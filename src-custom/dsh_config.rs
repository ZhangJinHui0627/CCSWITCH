//! DeepSeek Harness（DSH 桌面端 Agent）。
//!
//! DeepSeek Harness 的供应商连接由其自身 profiles/patch 管理；CC Switch 在
//! 数据库里登记 DSH 供应商（additive 模式）并从其会话日志 `session.v4.jsonl.zstd`
//! 中导入已提交的 token 使用量。
//! 这里提供配置目录与可执行文件定位。

use std::path::PathBuf;

use crate::config::get_home_dir;

/// DSH 的配置与数据根目录（~/.dsh）。
pub(crate) fn get_dsh_dir() -> PathBuf {
    get_home_dir().join(".dsh")
}

/// DSH 桌面端的配置文件路径（~/.dsh/profiles/desktop/cordis.patch.yml）。
#[allow(dead_code)]
pub(crate) fn dsh_settings_path() -> PathBuf {
    get_dsh_dir()
        .join("profiles")
        .join("desktop")
        .join("cordis.patch.yml")
}

/// DSH 桌面端可执行程序路径探测。
pub(crate) fn dsh_exe_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let p = PathBuf::from(local_app_data)
                .join("Programs")
                .join("DeepSeek Harness")
                .join("DeepSeek Harness.exe");
            if p.exists() {
                return Some(p);
            }
        }
        let fallback = get_home_dir()
            .join("AppData")
            .join("Local")
            .join("Programs")
            .join("DeepSeek Harness")
            .join("DeepSeek Harness.exe");
        if fallback.exists() {
            return Some(fallback);
        }
    }

    #[cfg(target_os = "macos")]
    {
        let app = PathBuf::from("/Applications/DeepSeek Harness.app");
        if app.exists() {
            return Some(app);
        }
        let user_app = get_home_dir().join("Applications/DeepSeek Harness.app");
        if user_app.exists() {
            return Some(user_app);
        }
    }

    None
}

/// 检查 DSH 是否已安装或已使用。
pub(crate) fn dsh_installed() -> bool {
    let dir = get_dsh_dir();
    dir.exists() || dsh_exe_path().is_some()
}

/// DSH 凭证文件路径（~/.dsh/.credentials.yaml）。
pub(crate) fn dsh_credentials_path() -> PathBuf {
    get_dsh_dir().join(".credentials.yaml")
}

/// 读取 DSH 的环境变量凭证映射（refs）。
pub(crate) fn read_credentials() -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    let path = dsh_credentials_path();
    if !path.exists() {
        return map;
    }
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(val) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
            if let Some(refs) = val.get("refs").and_then(|r| r.as_mapping()) {
                for (k, v) in refs {
                    if let (Some(ks), Some(vs)) = (k.as_str(), v.as_str()) {
                        map.insert(ks.to_string(), vs.to_string());
                    }
                }
            }
        }
    }
    map
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct DshLiveProvider {
    pub id: String,
    pub name: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub api: Option<String>,
    pub default_model: Option<String>,
    pub models: Vec<serde_json::Value>,
    pub settings_config: serde_json::Value,
    pub is_default: bool,
}

/// 从 DSH 真实配置（cordis.patch.yml + .credentials.yaml）解析活体供应商配置。
pub(crate) fn get_dsh_live_providers() -> Vec<DshLiveProvider> {
    let patch_path = dsh_settings_path();
    if !patch_path.exists() {
        return Vec::new();
    }
    let content = match std::fs::read_to_string(&patch_path) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to read DSH patch file: {e}");
            return Vec::new();
        }
    };
    let items: Vec<serde_yaml::Value> = match serde_yaml::from_str(&content) {
        Ok(items) => items,
        Err(e) => {
            log::warn!("Failed to parse DSH patch yaml: {e}");
            return Vec::new();
        }
    };

    let creds = read_credentials();

    // 寻找全局默认模型与供应商
    let mut default_provider_id: Option<String> = None;
    let mut default_model: Option<String> = None;
    for item in &items {
        if item.get("id").and_then(|v| v.as_str()) == Some("agent-default-model") {
            if let Some(config) = item.get("config") {
                default_provider_id = config
                    .get("provider")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                default_model = config
                    .get("model")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
            }
        }
    }

    let mut result = Vec::new();
    for item in &items {
        if item.get("id").and_then(|v| v.as_str()) == Some("llm-pi-ai") {
            if let Some(providers_val) = item.get("config").and_then(|c| c.get("providers")) {
                if let Some(mapping) = providers_val.as_mapping() {
                    for (pkey, pval) in mapping {
                        let pid = match pkey.as_str() {
                            Some(s) => s.to_string(),
                            None => continue,
                        };
                        let display_name = pval
                            .get("displayName")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&pid)
                            .to_string();
                        let base_url = pval
                            .get("baseURL")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let api = pval
                            .get("api")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        let api_key_env = pval.get("apiKeyEnv").and_then(|v| v.as_str());
                        let api_key = api_key_env.and_then(|env_name| {
                            creds
                                .get(env_name)
                                .cloned()
                                .or_else(|| std::env::var(env_name).ok())
                        });

                        let models_json: Vec<serde_json::Value> =
                            if let Some(mseq) = pval.get("models").and_then(|v| v.as_sequence()) {
                                mseq.iter()
                                    .filter_map(|m| serde_json::to_value(m).ok())
                                    .collect()
                            } else {
                                Vec::new()
                            };

                        let is_default = default_provider_id.as_deref() == Some(&pid);
                        let prov_default_model = if is_default {
                            default_model.clone()
                        } else {
                            models_json.first().and_then(|m| {
                                m.get("id").and_then(|v| v.as_str()).map(|s| s.to_string())
                            })
                        };

                        let mut settings_obj = serde_json::Map::new();
                        if let Some(ref bu) = base_url {
                            settings_obj.insert("baseURL".to_string(), bu.clone().into());
                            settings_obj.insert("base_url".to_string(), bu.clone().into());
                        }
                        if let Some(ref ak) = api_key {
                            settings_obj.insert("apiKey".to_string(), ak.clone().into());
                            settings_obj.insert("api_key".to_string(), ak.clone().into());
                        }
                        if let Some(ref a) = api {
                            settings_obj.insert("api".to_string(), a.clone().into());
                        }
                        if let Some(ref dm) = prov_default_model {
                            settings_obj.insert("default_model".to_string(), dm.clone().into());
                            settings_obj.insert("model".to_string(), dm.clone().into());
                        }
                        if !models_json.is_empty() {
                            settings_obj.insert(
                                "models".to_string(),
                                serde_json::Value::Array(models_json.clone()),
                            );
                        }

                        result.push(DshLiveProvider {
                            id: pid,
                            name: display_name,
                            base_url,
                            api_key,
                            api,
                            default_model: prov_default_model,
                            models: models_json,
                            settings_config: serde_json::Value::Object(settings_obj),
                            is_default,
                        });
                    }
                }
            }
        }
    }

    result
}
