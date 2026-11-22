//! Import ZCode's committed token-usage projection without modifying its database.
use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::services::session_usage::SessionSyncResult;
use crate::services::sql_helpers::INPUT_TOKEN_SEMANTICS_FRESH;
use rusqlite::{params, OpenFlags};

pub fn sync_zcode_usage(db: &Database) -> Result<SessionSyncResult, AppError> {
    let db_path = crate::zcode_config::get_zcode_dir()
        .join("cli")
        .join("db")
        .join("db.sqlite");
    if !db_path.exists() {
        return Ok(SessionSyncResult::default());
    }

    let source = rusqlite::Connection::open_with_flags(
        &db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| AppError::Database(format!("Failed to open ZCode database: {e}")))?;

    let key = format!("zcode:{}", db_path.display());
    sync_from_database(db, &source, &key)
}

fn sync_from_database(
    db: &Database,
    source: &rusqlite::Connection,
    key: &str,
) -> Result<SessionSyncResult, AppError> {
    let mut result = SessionSyncResult::default();
    let mut conn = lock_conn!(db.conn);
    let tx = conn.transaction()?;

    let mut cursor = tx
        .query_row(
            "SELECT COALESCE(MAX(last_line_offset), 0) FROM session_log_sync WHERE file_path = ?1",
            [key],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0);

    let mut query = source.prepare(
        "SELECT rowid, id, session_id, model_id, status, started_at, completed_at, duration_ms, \
                time_to_first_token_ms, input_tokens, output_tokens, cache_read_input_tokens, \
                cache_creation_input_tokens \
         FROM model_usage \
         WHERE rowid > ?1 AND status != 'error' AND (COALESCE(input_tokens, 0) + COALESCE(output_tokens, 0)) > 0 \
         ORDER BY rowid ASC",
    )?;

    result.files_scanned = 1;
    let mut rows = query.query([cursor])?;

    while let Some(row) = rows.next()? {
        let rowid: i64 = row.get(0)?;
        let id: String = row.get(1)?;
        let session_id: Option<String> = row.get(2)?;
        let model_id: Option<String> = row.get(3)?;
        let status: Option<String> = row.get(4)?;
        let started_at: Option<i64> = row.get(5)?;
        let completed_at: Option<i64> = row.get(6)?;
        let duration_ms: Option<i64> = row.get(7)?;
        let time_to_first_token_ms: Option<i64> = row.get(8)?;
        let input_tokens: i64 = row.get(9).unwrap_or(0);
        let output_tokens: i64 = row.get(10).unwrap_or(0);
        let cache_read: i64 = row.get(11).unwrap_or(0);
        let cache_creation: i64 = row.get(12).unwrap_or(0);

        let model = model_id.as_deref().unwrap_or("unknown");
        let ts = completed_at.or(started_at).unwrap_or(0) / 1000;
        let status_code = if status.as_deref() == Some("completed") {
            200
        } else {
            499
        };

        let request_id = format!("zcode:{id}");
        let session = session_id.as_deref().unwrap_or("");

        let changed = tx.execute(
            "INSERT OR IGNORE INTO proxy_request_logs (
                request_id, provider_id, app_type, model, request_model, pricing_model,
                input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                input_cost_usd, output_cost_usd, cache_read_cost_usd, cache_creation_cost_usd,
                total_cost_usd, latency_ms, first_token_ms, duration_ms, status_code,
                session_id, provider_type, is_streaming, cost_multiplier,
                created_at, data_source, input_token_semantics
             ) VALUES (
                ?1, '_zcode_session', 'zcode', ?2, ?2, '',
                ?3, ?4, ?5, ?6,
                '0', '0', '0', '0',
                '0', 0, ?7, ?8, ?9,
                ?10, 'zcode_session', 1, '1',
                ?11, 'zcode_session', ?12
             )",
            params![
                request_id,
                model,
                input_tokens,
                output_tokens,
                cache_read,
                cache_creation,
                time_to_first_token_ms,
                duration_ms,
                status_code,
                session,
                ts,
                INPUT_TOKEN_SEMANTICS_FRESH,
            ],
        )?;

        result.imported += changed as u32;
        result.skipped += u32::from(changed == 0);
        cursor = rowid;
    }

    // Commit watermark
    tx.execute(
        "INSERT INTO session_log_sync (file_path, last_modified, last_line_offset, last_synced_at) \
         VALUES (?1, 0, ?2, unixepoch()) \
         ON CONFLICT(file_path) DO UPDATE SET \
         last_line_offset = excluded.last_line_offset, last_synced_at = excluded.last_synced_at",
        params![key, cursor],
    )?;

    tx.commit()?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zcode_usage_sync_imports_records_and_advances_cursor() {
        let source = rusqlite::Connection::open_in_memory().unwrap();
        source
            .execute_batch(
                "CREATE TABLE model_usage (
                    id TEXT PRIMARY KEY,
                    session_id TEXT,
                    model_id TEXT,
                    status TEXT,
                    started_at INTEGER,
                    completed_at INTEGER,
                    duration_ms INTEGER,
                    time_to_first_token_ms INTEGER,
                    input_tokens INTEGER,
                    output_tokens INTEGER,
                    cache_read_input_tokens INTEGER,
                    cache_creation_input_tokens INTEGER
                );
                INSERT INTO model_usage VALUES
                    ('req-1', 's1', 'GLM-5.3', 'completed', 1000000, 1005000, 5000, 500, 1000, 200, 800, 0),
                    ('req-2', 's1', 'GLM-5.3', 'error', 1010000, 1011000, 1000, 0, 0, 0, 0, 0);",
            )
            .unwrap();

        let db = Database::memory().unwrap();

        // 1. First sync imports valid completed record and skips error
        let r1 = sync_from_database(&db, &source, "test-zcode").unwrap();
        assert_eq!(r1.imported, 1);
        assert_eq!(r1.skipped, 0);

        // Verify inserted data in proxy_request_logs
        {
            let conn = db.conn.lock().unwrap();
            let row = conn
                .query_row(
                    "SELECT request_id, model, input_tokens, output_tokens, cache_read_tokens, status_code, app_type
                     FROM proxy_request_logs WHERE app_type = 'zcode'",
                    [],
                    |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, i64>(2)?,
                            r.get::<_, i64>(3)?,
                            r.get::<_, i64>(4)?,
                            r.get::<_, i64>(5)?,
                            r.get::<_, String>(6)?,
                        ))
                    },
                )
                .unwrap();
            assert_eq!(
                row,
                (
                    "zcode:req-1".into(),
                    "GLM-5.3".into(),
                    1000,
                    200,
                    800,
                    200,
                    "zcode".into()
                )
            );
        }

        // 2. Second sync imports nothing (cursor points at row 1)
        let r2 = sync_from_database(&db, &source, "test-zcode").unwrap();
        assert_eq!(r2.imported, 0);

        // 3. Insert new row and verify incremental pickup
        source
            .execute_batch(
                "INSERT INTO model_usage VALUES
                 ('req-3', 's2', 'GLM-5.3-Flash', 'completed', 1020000, 1022000, 2000, 300, 500, 150, 400, 0);",
            )
            .unwrap();

        let r3 = sync_from_database(&db, &source, "test-zcode").unwrap();
        assert_eq!(r3.imported, 1);

        // Verify total in proxy_request_logs
        {
            let conn = db.conn.lock().unwrap();
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM proxy_request_logs WHERE app_type = 'zcode'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, 2);
        }
    }

    #[test]
    fn zcode_reads_real_database_if_present() {
        let db_path = crate::zcode_config::get_zcode_dir()
            .join("cli")
            .join("db")
            .join("db.sqlite");
        if !db_path.exists() {
            return;
        }

        let db = Database::memory().unwrap();
        let r = sync_zcode_usage(&db).unwrap();
        assert!(
            r.imported >= 400,
            "expected at least 400 imported rows, got {}",
            r.imported
        );

        // Incremental sync should import 0
        let r2 = sync_zcode_usage(&db).unwrap();
        assert_eq!(r2.imported, 0);
    }
}
