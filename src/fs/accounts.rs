use anyhow::Result;
use rusqlite::{params, Connection};

use crate::include::types::AccountRow;

pub fn insert_account(
    conn: &Connection,
    phone: &str,
    session_bytes: &[u8],
) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO accounts (phone_number, session_bytes, active_group_count, is_active, is_full)
         VALUES (?1, ?2, 0, 1, 0)",
        params![phone, session_bytes],
    )?;
    Ok(())
}

pub fn load_active_accounts(conn: &Connection) -> Result<Vec<AccountRow>> {
    let mut stmt = conn.prepare(
        "SELECT phone_number, session_bytes, active_group_count, is_active, is_full
         FROM accounts WHERE is_active = 1 AND is_full = 0",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(AccountRow {
            phone_number: row.get(0)?,
            session_bytes: row.get(1)?,
            active_group_count: row.get(2)?,
            is_active: row.get::<_, i64>(3)? != 0,
            is_full: row.get::<_, i64>(4)? != 0,
        })
    })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn increment_group_count(conn: &Connection, phone: &str, max: usize) -> Result<bool> {
    let current: i64 = conn.query_row(
        "SELECT active_group_count FROM accounts WHERE phone_number = ?1",
        params![phone],
        |row| row.get(0),
    )?;
    if current as usize >= max {
        conn.execute(
            "UPDATE accounts SET is_full = 1 WHERE phone_number = ?1",
            params![phone],
        )?;
        return Ok(false);
    }
    conn.execute(
        "UPDATE accounts SET active_group_count = active_group_count + 1 WHERE phone_number = ?1",
        params![phone],
    )?;
    Ok(true)
}

pub fn set_account_active(conn: &Connection, phone: &str, active: bool) -> Result<()> {
    conn.execute(
        "UPDATE accounts SET is_active = ?1 WHERE phone_number = ?2",
        params![if active { 1 } else { 0 }, phone],
    )?;
    Ok(())
}

pub fn lowest_load_account(conn: &Connection) -> Result<Option<String>> {
    let mut stmt = conn.prepare(
        "SELECT phone_number FROM accounts
         WHERE is_active = 1 AND is_full = 0
         ORDER BY active_group_count ASC LIMIT 1",
    )?;
    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row.get(0)?))
    } else {
        Ok(None)
    }
}
