use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use std::path::PathBuf;

use crate::confidence::Finding;
use crate::config::aliascope_dir;

pub struct Store {
    pub conn: Connection,
}

impl Store {
    pub fn open_default() -> Result<Self> {
        Self::open(aliascope_dir()?.join("aliascope.db"))
    }

    pub fn open(path: PathBuf) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            CREATE TABLE IF NOT EXISTS scans (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                kind TEXT NOT NULL,
                query TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS findings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                scan_id INTEGER NOT NULL,
                engine TEXT NOT NULL,
                subject TEXT NOT NULL,
                title TEXT NOT NULL,
                detail TEXT NOT NULL,
                url TEXT,
                confidence TEXT NOT NULL,
                reason TEXT NOT NULL,
                FOREIGN KEY(scan_id) REFERENCES scans(id)
            );
            CREATE TABLE IF NOT EXISTS http_cache (
                cache_key TEXT PRIMARY KEY,
                status INTEGER NOT NULL,
                body_hash TEXT,
                stored_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                created_at TEXT NOT NULL,
                action TEXT NOT NULL,
                detail TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_findings_scan ON findings(scan_id);
            "#,
        )?;
        Ok(Self { conn })
    }

    pub fn audit(&self, action: &str, detail: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO audit_log (created_at, action, detail) VALUES (?1,?2,?3)",
            params![Utc::now().to_rfc3339(), action, detail],
        )?;
        Ok(())
    }

    pub fn save_scan(&self, kind: &str, query: &str, findings: &[Finding]) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO scans (kind, query, created_at) VALUES (?1,?2,?3)",
            params![kind, query, Utc::now().to_rfc3339()],
        )?;
        let id = self.conn.last_insert_rowid();
        for f in findings {
            self.conn.execute(
                "INSERT INTO findings (scan_id, engine, subject, title, detail, url, confidence, reason)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![
                    id,
                    f.engine,
                    f.subject,
                    f.title,
                    f.detail,
                    f.url,
                    f.confidence.as_str(),
                    f.reason
                ],
            )?;
        }
        self.audit("scan_saved", &format!("{kind}:{query}:{}", findings.len()))?;
        Ok(id)
    }

    pub fn cache_get(&self, key: &str, ttl_hours: i64) -> Result<Option<u16>> {
        let mut stmt = self.conn.prepare(
            "SELECT status, stored_at FROM http_cache WHERE cache_key = ?1",
        )?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            let status: i64 = row.get(0)?;
            let stored_at: String = row.get(1)?;
            if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&stored_at) {
                let age = Utc::now().signed_duration_since(ts.with_timezone(&Utc));
                if age.num_hours() <= ttl_hours {
                    return Ok(Some(status as u16));
                }
            }
        }
        Ok(None)
    }

    pub fn cache_put(&self, key: &str, status: u16) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO http_cache (cache_key, status, body_hash, stored_at)
             VALUES (?1, ?2, NULL, ?3)",
            params![key, status as i64, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn recent_scans(&self, limit: usize) -> Result<Vec<(i64, String, String, String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.kind, s.query, s.created_at,
                    (SELECT COUNT(*) FROM findings f WHERE f.scan_id = s.id)
             FROM scans s ORDER BY s.id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }
}
