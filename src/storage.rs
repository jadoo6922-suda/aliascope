use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use std::path::PathBuf;

use crate::username::Hit;

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open_default() -> Result<Self> {
        let dir = dirs::home_dir()
            .context("home dir")?
            .join(".aliascope");
        std::fs::create_dir_all(&dir)?;
        Self::open(dir.join("aliascope.db"))
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
            CREATE TABLE IF NOT EXISTS hits (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                scan_id INTEGER NOT NULL,
                alias TEXT NOT NULL,
                platform TEXT NOT NULL,
                url TEXT NOT NULL,
                status INTEGER NOT NULL,
                confidence TEXT NOT NULL,
                FOREIGN KEY(scan_id) REFERENCES scans(id)
            );
            CREATE INDEX IF NOT EXISTS idx_hits_scan ON hits(scan_id);
            CREATE INDEX IF NOT EXISTS idx_scans_query ON scans(query);
            "#,
        )?;
        Ok(Self { conn })
    }

    pub fn save_user_scan(&self, query: &str, hits: &[Hit]) -> Result<i64> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO scans (kind, query, created_at) VALUES (?1, ?2, ?3)",
            params!["user", query, now],
        )?;
        let scan_id = self.conn.last_insert_rowid();
        for h in hits {
            let conf = if h.likely_exists { "medium" } else { "low" };
            self.conn.execute(
                "INSERT INTO hits (scan_id, alias, platform, url, status, confidence)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![scan_id, h.alias, h.platform, h.url, h.status as i64, conf],
            )?;
        }
        Ok(scan_id)
    }

    pub fn recent_scans(&self, limit: usize) -> Result<Vec<(i64, String, String, String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.kind, s.query, s.created_at,
                    (SELECT COUNT(*) FROM hits h WHERE h.scan_id = s.id)
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
