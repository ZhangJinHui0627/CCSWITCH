//! DeepSeek Harness 会话日志使用追踪
//!
//! 从 ~/.dsh/sessions/<session_uuid>/session.v4.jsonl.zstd 中提取已提交的 token 使用数据。
//!
//! ## 数据流
//! ```text
//! ~/.dsh/sessions/*/session.v4.jsonl.zstd → zstd 流解压 → assistant/message → proxy_request_logs 表
//! ```

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::proxy::usage::calculator::CostCalculator;
use crate::proxy::usage::parser::TokenUsage;
use crate::services::session_usage::{
    metadata_modified_nanos, SessionSyncResult,
};
use crate::services::sql_helpers::INPUT_TOKEN_SEMANTICS_FRESH;
use crate::services::usage_stats::find_model_pricing;
use rusqlite::params;
use rust_decimal::Decimal;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// 同步 DeepSeek Harness 使用数据（从 session.v4.jsonl.zstd 会话日志）
pub fn sync_dsh_usage(db: &Database) -> Result<SessionSyncResult, AppError> {
    let dsh_dir = crate::dsh_config::get_dsh_dir();
    let files = collect_dsh_session_files(&dsh_dir);

    let mut result = SessionSyncResult {
        imported: 0,
        skipped: 0,
        files_scanned: files.len() as u32,
        suspected_duplicates: 0,
        deferred_files: 0,
        errors: vec![],
    };

    if files.is_empty() {
        return Ok(result);
    }

    let cursors = crate::services::session_usage::load_sync_cursors(db)?;

    for file_path in &files {
        let last_modified = cursors
            .get(file_path.to_string_lossy().as_ref())
            .map_or(0, |c| c.last_modified);
        match sync_single_dsh_file(db, file_path, last_modified) {
            Ok((imported, skipped)) => {
                result.imported += imported;
                result.skipped += skipped;
            }
            Err(e) => {
                let msg = format!("DSH 会话文件解析失败 {}: {e}", file_path.display());
                log::warn!("[DSH-SYNC] {msg}");
                result.errors.push(msg);
            }
        }
    }

    if result.imported > 0 {
        log::info!(
            "[DSH-SYNC] 同步完成: 导入 {} 条, 跳过 {} 条, 扫描 {} 个文件",
            result.imported,
            result.skipped,
            result.files_scanned
        );
    }

    Ok(result)
}

/// 递归扫描收集所有 DSH 会话 zstd 文件
fn collect_dsh_session_files_recursive(dir: &Path, files: &mut Vec<PathBuf>, max_depth: usize) {
    if max_depth == 0 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_dsh_session_files_recursive(&path, files, max_depth - 1);
        } else if path.is_file() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == "session.v4.jsonl.zstd" || name.ends_with(".jsonl.zstd") {
                files.push(path);
            }
        }
    }
}

/// 扫描收集所有 DSH 会话 zstd 文件
pub(crate) fn collect_dsh_session_files(dsh_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let sessions_dir = dsh_dir.join("sessions");
    if sessions_dir.is_dir() {
        collect_dsh_session_files_recursive(&sessions_dir, &mut files, 5);
    }
    files
}

/// 同步单个 DSH 会话 zstd 压缩日志
fn sync_single_dsh_file(
    db: &Database,
    file_path: &Path,
    last_modified: i64,
) -> Result<(u32, u32), AppError> {
    let file_path_str = file_path.to_string_lossy().to_string();

    let metadata = fs::metadata(file_path)
        .map_err(|e| AppError::Config(format!("无法读取文件元数据: {e}")))?;
    let file_modified = metadata_modified_nanos(&metadata);

    // 文件未变化则跳过
    if file_modified <= last_modified {
        return Ok((0, 0));
    }

    let file = File::open(file_path)
        .map_err(|e| AppError::Config(format!("无法打开文件: {e}")))?;
    let decoder = zstd::stream::read::Decoder::new(file)
        .map_err(|e| AppError::Config(format!("创建 zstd 解压器失败: {e}")))?;
    let reader = BufReader::new(decoder);

    // 默认以会话目录名作为 session_id
    let inferred_session_id = file_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    let mut imported = 0u32;
    let mut skipped = 0u32;
    let mut processed_records = 0i64;

    let mut conn = lock_conn!(db.conn);
    let tx = conn.transaction()?;

    for line_res in reader.lines() {
        let line = match line_res {
            Ok(l) => l,
            Err(e) => {
                log::warn!("[DSH-SYNC] 读取 zstd 解压行失败: {e}");
                break;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        let val: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if val.get("type").and_then(|v| v.as_str()) != Some("assistant/message") {
            continue;
        }

        let data = match val.get("data") {
            Some(d) if d.is_object() => d,
            _ => continue,
        };

        let usage_obj = match data.get("usage") {
            Some(u) if u.is_object() => u,
            _ => continue,
        };

        let input_tokens = usage_obj
            .get("inputTokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let output_tokens = usage_obj
            .get("outputTokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let cache_read = usage_obj
            .get("cacheReadTokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        if input_tokens == 0 && output_tokens == 0 && cache_read == 0 {
            continue;
        }

        processed_records += 1;

        let message_id = data
            .get("message")
            .and_then(|m| m.get("id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{inferred_session_id}:{processed_records}"));

        // 模型优先级：usage.model（本条请求实际使用的模型）> message.source.model
        // > data.source.model > message.model。DSH 在会话中途 /model 切换时，
        // 每条消息的 usage.model 才是准的，message.source 只是会话启动时的配置。
        let model = usage_obj
            .get("model")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .or_else(|| {
                data.get("message")
                    .and_then(|m| m.get("source"))
                    .and_then(|s| s.get("model"))
                    .and_then(|v| v.as_str())
            })
            .or_else(|| {
                data.get("source")
                    .and_then(|s| s.get("model"))
                    .and_then(|v| v.as_str())
            })
            .or_else(|| {
                data.get("message")
                    .and_then(|m| m.get("model"))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("unknown");
        let model = match model {
            "hy4free" | "hy4std" | "hy4" | "hunyuan-hy4" => "Hunyuan Hy4",
            "hy3" | "hunyuan-hy3" => "Hunyuan Hy3",
            other => other,
        };

        let time_ms = val.get("time").and_then(|v| v.as_i64()).unwrap_or(0);
        let created_at = if time_ms > 0 {
            time_ms / 1000
        } else {
            file_modified / 1_000_000_000
        };

        let token_usage = TokenUsage {
            input_tokens,
            output_tokens,
            cache_read_tokens: cache_read,
            cache_creation_tokens: 0,
            cache_creation_1h_tokens: 0,
            model: Some(model.to_string()),
            message_id: Some(message_id.clone()),
        };

        // 分项与合计一起写：详情面板按分项展示，只写 total 会让"输入/输出成本"显示 0。
        let (input_cost, output_cost, cache_read_cost, cache_creation_cost, cost) =
            match find_model_pricing(&tx, model) {
                Some(pricing) => {
                    let c = CostCalculator::calculate_for_app(
                        "dsh",
                        &token_usage,
                        &pricing,
                        Decimal::ONE,
                    );
                    (
                        c.input_cost.to_string(),
                        c.output_cost.to_string(),
                        c.cache_read_cost.to_string(),
                        c.cache_creation_cost.to_string(),
                        c.total_cost.to_string(),
                    )
                }
                None => (
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                ),
            };

        let request_id = format!("dsh:{message_id}");

        let changed = tx.execute(
            "INSERT OR IGNORE INTO proxy_request_logs (
                request_id, provider_id, app_type, model, request_model, pricing_model,
                input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                input_cost_usd, output_cost_usd, cache_read_cost_usd, cache_creation_cost_usd,
                total_cost_usd, latency_ms, first_token_ms, duration_ms, status_code,
                session_id, provider_type, is_streaming, cost_multiplier,
                created_at, data_source, input_token_semantics
             ) VALUES (
                ?1, '_dsh_session', 'dsh', ?2, ?2, '',
                ?3, ?4, ?5, 0,
                ?10, ?11, ?12, ?13,
                ?6, 0, 0, 0, 200,
                ?7, 'dsh_session', 1, '1',
                ?8, 'dsh_session', ?9
             )",
            params![
                request_id,
                model,
                input_tokens,
                output_tokens,
                cache_read,
                cost,
                inferred_session_id,
                created_at,
                INPUT_TOKEN_SEMANTICS_FRESH,
                input_cost,
                output_cost,
                cache_read_cost,
                cache_creation_cost,
            ],
        )?;

        if changed > 0 {
            imported += 1;
        } else {
            skipped += 1;
        }
    }

    // 更新同步游标
    crate::services::session_usage::update_sync_state_on_conn(
        &tx,
        &file_path_str,
        file_modified,
        processed_records,
    )?;

    tx.commit()?;

    Ok((imported, skipped))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dsh_usage_sync_imports_from_zstd() {
        let temp_dir = tempfile::tempdir().unwrap();
        let session_dir = temp_dir.path().join("sessions").join("test-session-123");
        fs::create_dir_all(&session_dir).unwrap();

        let session_file = session_dir.join("session.v4.jsonl.zstd");
        let line = r#"{"time":1740567196000,"type":"assistant/message","data":{"message":{"id":"msg-abc-123","model":"deepseek-v4.1-flash","role":"assistant"},"source":{"provider":"qianwen","model":"deepseek-v4.1-flash"},"usage":{"inputTokens":120,"outputTokens":45,"cacheReadTokens":30}}}"#;

        let compressed = zstd::stream::encode_all(format!("{line}\n").as_bytes(), 3).unwrap();
        fs::write(&session_file, compressed).unwrap();

        let db = Database::memory().unwrap();

        let (imported, skipped) = sync_single_dsh_file(&db, &session_file, 0).unwrap();
        assert_eq!(imported, 1);
        assert_eq!(skipped, 0);

        // 再次同步相同未改动的文件应该跳过
        let metadata = fs::metadata(&session_file).unwrap();
        let modified = metadata_modified_nanos(&metadata);
        let (imported2, skipped2) = sync_single_dsh_file(&db, &session_file, modified).unwrap();
        assert_eq!(imported2, 0);
        assert_eq!(skipped2, 0);
    }

    #[test]
    fn dsh_reads_real_sessions_if_present() {
        let dsh_dir = crate::dsh_config::get_dsh_dir();
        if !dsh_dir.exists() {
            return;
        }

        let db = Database::memory().unwrap();
        let r = sync_dsh_usage(&db).unwrap();
        // 如果本机有 session 文件，导入数应大于 0
        if r.files_scanned > 0 {
            assert!(r.imported > 0, "expected at least 1 imported row, got {}", r.imported);
            let r2 = sync_dsh_usage(&db).unwrap();
            assert_eq!(r2.imported, 0);
        }
    }
}
