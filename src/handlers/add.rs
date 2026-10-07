use anyhow::Result;
use teloxide::prelude::*;

use crate::core::pool::PoolHandle;
use crate::include::types::{GroupStatus, JoinTask};
use crate::net::rpc::execute_join;

pub async fn handle_add(
    bot: Bot,
    msg: Message,
    args: String,
    pool: PoolHandle,
    groups_conn: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
    accounts_conn: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
    queue_tx: tokio::sync::mpsc::Sender<JoinTask>,
) -> Result<()> {
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.is_empty() {
        bot.send_message(msg.chat.id, "Usage: /add <link|ID> [index]").await?;
        return Ok(());
    }
    let target = parts[0].to_string();
    let phone = if parts.len() > 1 {
        parts[1].to_string()
    } else {
        let conn = accounts_conn.lock().map_err(|e| anyhow::anyhow!("lock: {}", e))?;
        crate::fs::accounts::lowest_load_account(&conn)?.unwrap_or_default()
    };
    if phone.is_empty() {
        bot.send_message(msg.chat.id, "No available account.").await?;
        return Ok(());
    }
    let client = pool.get(&phone).await;
    let Some(client) = client else {
        bot.send_message(msg.chat.id, "Account not in pool.").await?;
        return Ok(());
    };
    match execute_join(&client, &target).await {
        Ok(chat_ref) => {
            let added_by = msg
                .from
                .as_ref()
                .map(|u| u.id.0.to_string())
                .unwrap_or_default();
            {
                let conn = groups_conn.lock().map_err(|e| anyhow::anyhow!("lock: {}", e))?;
                crate::fs::groups::upsert_group(
                    &conn,
                    &chat_ref,
                    "Added",
                    None,
                    Some(&target),
                    GroupStatus::Active,
                    Some(&phone),
                    &added_by,
                )?;
            }
            {
                let conn = accounts_conn.lock().map_err(|e| anyhow::anyhow!("lock: {}", e))?;
                crate::fs::accounts::increment_group_count(&conn, &phone, 480)?;
            }
            bot.send_message(msg.chat.id, format!("Added {} via {}", chat_ref, phone))
                .await?;
        }
        Err(e) => {
            let _ = queue_tx
                .send(JoinTask {
                    invite_link: target.clone(),
                    chat_id_hint: None,
                    requested_by: msg.from.as_ref().map(|u| u.id),
                    requester_username: None,
                    requester_chat_id: Some(msg.chat.id.0),
                    submission_message_id: None,
                })
                .await;
            bot.send_message(
                msg.chat.id,
                format!("Direct join failed: {}. Queued for retry.", e),
            )
            .await?;
        }
    }
    Ok(())
}
