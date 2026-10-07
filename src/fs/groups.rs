use anyhow::Result;
use rusqlite::{params, Connection};

use crate::include::types::{GroupRow, GroupStatus};

pub fn upsert_group(
    conn: &Connection,
    chat_id: &str,
    title: &str,
    username: Option<&str>,
    invite_link: Option<&str>,
    status: GroupStatus,
    assigned_account: Option<&str>,
    added_by: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO cheat_groups (chat_id, title, username, invite_link, status, assigned_account_phone, added_by)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(chat_id) DO UPDATE SET
             title = excluded.title,
             username = excluded.username,
             invite_link = COALESCE(excluded.invite_link, cheat_groups.invite_link),
             status = excluded.status,
             assigned_account_phone = COALESCE(excluded.assigned_account_phone, cheat_groups.assigned_account_phone)",
        params![chat_id, title, username, invite_link, status.as_str(), assigned_account, added_by],
    )?;
    Ok(())
}

pub fn active_chat_ids(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT chat_id FROM cheat_groups WHERE status = 'active'",
    )?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn find_active_group(conn: &Connection, chat_id: &str) -> Result<Option<GroupRow>> {
    let mut stmt = conn.prepare(
        "SELECT chat_id, title, username, invite_link, status, assigned_account_phone, added_by
         FROM cheat_groups WHERE chat_id = ?1 AND status = 'active'",
    )?;
    let mut rows = stmt.query(params![chat_id])?;
    if let Some(row) = rows.next()? {
        let status_str: String = row.get(4)?;
        Ok(Some(GroupRow {
            chat_id: row.get(0)?,
            title: row.get(1)?,
            username: row.get(2)?,
            invite_link: row.get(3)?,
            status: GroupStatus::from_str(&status_str).unwrap_or(GroupStatus::Active),
            assigned_account_phone: row.get(5)?,
            added_by: row.get(6)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn update_status_by_invite(conn: &Connection, invite_link: &str, status: GroupStatus) -> Result<()> {
    conn.execute(
        "UPDATE cheat_groups SET status = ?1 WHERE invite_link = ?2",
        params![status.as_str(), invite_link],
    )?;
    Ok(())
}
