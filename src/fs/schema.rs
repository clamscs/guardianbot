use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

pub fn open_accounts_db(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 5000;
         CREATE TABLE IF NOT EXISTS accounts (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             phone_number TEXT UNIQUE NOT NULL,
             session_bytes BLOB NOT NULL,
             active_group_count INTEGER DEFAULT 0,
             is_active INTEGER DEFAULT 1,
             is_full INTEGER DEFAULT 0,
             created_at DATETIME DEFAULT CURRENT_TIMESTAMP
         );",
    )?;
    Ok(conn)
}

pub fn open_groups_db(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 5000;
         CREATE TABLE IF NOT EXISTS cheat_groups (
             chat_id TEXT PRIMARY KEY,
             title TEXT NOT NULL,
             username TEXT,
             invite_link TEXT UNIQUE,
             status TEXT CHECK(status IN ('active', 'pending_approval', 'pending_join')) NOT NULL,
             assigned_account_phone TEXT,
             added_by TEXT NOT NULL,
             created_at DATETIME DEFAULT CURRENT_TIMESTAMP
         );",
    )?;
    Ok(conn)
}
