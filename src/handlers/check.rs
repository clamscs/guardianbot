use anyhow::Result;
use futures::future::join_all;
use std::collections::HashMap;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode};

use crate::core::cache::CheckCache;
use crate::core::pool::PoolHandle;
use crate::include::constants::CACHE_PREFIX;
use crate::include::types::{CheckCachePayload, MatchEntry};
use crate::net::rpc::{get_common_chats, get_participant_role, resolve_user};
use crate::fs::groups::active_chat_ids;

pub async fn handle_check(
    bot: Bot,
    msg: Message,
    args: String,
    pool: PoolHandle,
    cache: Arc<CheckCache>,
    groups_conn: Arc<std::sync::Mutex<rusqlite::Connection>>,
) -> Result<()> {
    let arg = args.trim().trim_start_matches('@').to_string();
    if arg.is_empty() {
        bot.send_message(msg.chat.id, "Usage: /check <@username|ID>").await?;
        return Ok(());
    }
    let clients = pool.all_clients().await;
    if clients.is_empty() {
        bot.send_message(msg.chat.id, "No MTProto accounts available.").await?;
        return Ok(());
    }
    let (user_id, access_hash) = match resolve_user(&clients[0].1, &arg).await {
        Ok(v) => v,
        Err(e) => {
            bot.send_message(msg.chat.id, format!("Resolution failed: {}", e)).await?;
            return Ok(());
        }
    };
    let active_ids: Vec<String> = {
        let conn = groups_conn.lock().map_err(|e| anyhow::anyhow!("lock: {}", e))?;
        active_chat_ids(&conn)?
    };
    let active_set: std::collections::HashSet<i64> = active_ids
        .iter()
        .filter_map(|s| s.parse::<i64>().ok())
        .collect();
    let futures: Vec<_> = clients
        .iter()
        .map(|(_, c)| get_common_chats(c, user_id, access_hash))
        .collect();
    let results = join_all(futures).await;
    let mut matched: HashMap<i64, ()> = HashMap::new();
    for r in results {
        if let Ok(ids) = r {
            for id in ids {
                if active_set.contains(&id) {
                    matched.insert(id, ());
                }
            }
        }
    }
    let mut entries = Vec::new();
    for chat_id in matched.keys() {
        let role = get_participant_role(&clients[0].1, *chat_id, user_id)
            .await
            .unwrap_or(None);
        let (title, username) = {
            let conn = groups_conn.lock().map_err(|e| anyhow::anyhow!("lock: {}", e))?;
            match crate::fs::groups::find_active_group(&conn, &chat_id.to_string())? {
                Some(g) => (g.title, g.username),
                None => (format!("Group {}", chat_id), None),
            }
        };
        entries.push(MatchEntry {
            chat_id: chat_id.to_string(),
            title,
            username,
            role,
        });
    }
    let count = entries.len();
    let payload = CheckCachePayload {
        requester_id: msg.from.as_ref().map(|u| u.id).unwrap_or(UserId(0)),
        target_id: UserId(user_id as u64),
        target_first_name: arg.clone(),
        target_last_name: None,
        target_username: Some(arg.clone()),
        matches: entries,
    };
    let key = cache.insert(payload);
    let first_name = arg.clone();
    let text = if count > 0 {
        format!(
            "*User info*:\n• *ID*: `{}`\n• *First Name*: {}\n• *Last Name*: None\n• *Link*: [{}](tg://user?id={})\n\n⚠️ *Found in <u>{} {}</u>*",
            user_id, first_name, first_name, user_id, count,
            if count == 1 { "group" } else { "groups" }
        )
    } else {
        format!(
            "*User info*:\n• *ID*: `{}`\n• *First Name*: {}\n• *Last Name*: None\n• *Link*: [{}](tg://user?id={})\n\n✅ *Found in <u>0 groups</u>*",
            user_id, first_name, first_name, user_id
        )
    };
    let keyboard = if count > 0 {
        Some(InlineKeyboardMarkup::new(vec![vec![
            InlineKeyboardButton::callback("View matching groups", &key),
        ]]))
    } else {
        None
    };
    let mut req = bot.send_message(msg.chat.id, text).parse_mode(ParseMode::MarkdownV2);
    if let Some(kb) = keyboard {
        req = req.reply_markup(kb);
    }
    req.await?;
    Ok(())
}

pub async fn handle_check_callback(
    bot: Bot,
    q: CallbackQuery,
    cache: Arc<CheckCache>,
) -> Result<()> {
    let data = q.data.clone().unwrap_or_default();
    if !data.starts_with(CACHE_PREFIX) {
        return Ok(());
    }
    let payload = match cache.get(&data) {
        Some(p) => p,
        None => {
            bot.answer_callback_query(q.id.clone())
                .text("This session has expired. Please run /check again.")
                .show_alert(true)
                .await?;
            return Ok(());
        }
    };
    let clicker = q.from.id;
    if clicker != payload.requester_id {
        bot.answer_callback_query(q.id.clone())
            .text("This is not for you!")
            .show_alert(true)
            .await?;
        return Ok(());
    }
    bot.answer_callback_query(q.id.clone()).await?;
    let mut lines = Vec::new();
    for (i, m) in payload.matches.iter().enumerate() {
        let role_part = m
            .role
            .as_ref()
            .map(|r| format!(" ({})", r.to_lowercase()))
            .unwrap_or_default();
        lines.push(format!("{}. {}{}", i + 1, m.title, role_part));
        lines.push(format!("   ├── `ID`: `{}`", m.chat_id));
        match &m.username {
            Some(u) => lines.push(format!("   └── `Username`: `@{}`", u)),
            None => lines.push("   └── `Username`: `None (Private)`".to_string()),
        }
    }
    let text = lines.join("\n");
    if let Some(msg) = q.message.as_ref() {
        let chat_id = msg.chat().id;
        let msg_id = msg.id();
        bot.edit_message_text(chat_id, msg_id, text)
            .parse_mode(ParseMode::MarkdownV2)
            .await?;
    }
    Ok(())
}
