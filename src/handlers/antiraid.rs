use anyhow::Result;
use std::collections::HashSet;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{ChatMemberKind, ChatMemberUpdated, InlineKeyboardButton, InlineKeyboardMarkup, ParseMode};

use crate::core::pool::PoolHandle;
use crate::fs::groups::active_chat_ids;
use crate::net::rpc::{get_common_chats, resolve_user};

pub async fn handle_chat_member(
    bot: Bot,
    update: ChatMemberUpdated,
    pool: PoolHandle,
    groups_conn: Arc<std::sync::Mutex<rusqlite::Connection>>,
) -> Result<()> {
    let is_new_member = matches!(update.new_chat_member.kind, ChatMemberKind::Member);
    let was_left_or_banned = matches!(
        update.old_chat_member.kind,
        ChatMemberKind::Left | ChatMemberKind::Banned(_)
    );
    if !is_new_member || !was_left_or_banned {
        return Ok(());
    }
    let chat_id = update.chat.id;
    let user = update.new_chat_member.user.clone();
    let me = bot.get_me().await?;
    let bot_member = bot.get_chat_member(chat_id, me.id).await?;
    let can_restrict = match bot_member.kind {
        ChatMemberKind::Administrator(admin) => admin.can_restrict_members,
        _ => false,
    };
    if !can_restrict {
        return Ok(());
    }
    let clients = pool.all_clients().await;
    if clients.is_empty() {
        return Ok(());
    }
    let username = user.username.clone().unwrap_or_default();
    if username.is_empty() {
        return Ok(());
    }
    let Ok((user_id, access_hash)) = resolve_user(&clients[0].1, &username).await else {
        return Ok(());
    };
    let active_ids = {
        let conn = groups_conn.lock().map_err(|e| anyhow::anyhow!("lock: {}", e))?;
        active_chat_ids(&conn)?
    };
    let active_set: HashSet<i64> = active_ids.iter().filter_map(|s| s.parse().ok()).collect();
    let mut matched = false;
    for (_, c) in &clients {
        if let Ok(ids) = get_common_chats(c, user_id, access_hash).await {
            if ids.iter().any(|id| active_set.contains(id)) {
                matched = true;
                break;
            }
        }
    }
    if !matched {
        return Ok(());
    }
    let text = format!(
        "⚠️ *Suspicious user detected*\n\n*User*: [{}](tg://user?id={})\n*ID*: `{}`\n\nThis user is associated with blacklisted cheat groups.",
        user.first_name, user.id.0, user.id.0
    );
    let keyboard = InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::callback(
        "Ban User",
        format!("ban_user:{}:{}", chat_id.0, user.id.0),
    )]]);
    bot.send_message(chat_id, text)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(keyboard)
        .await?;
    Ok(())
}

pub async fn handle_ban_callback(bot: Bot, q: CallbackQuery) -> Result<()> {
    let data = q.data.clone().unwrap_or_default();
    let parts: Vec<&str> = data.split(':').collect();
    if parts.len() != 3 {
        return Ok(());
    }
    let chat_id: i64 = parts[1].parse().unwrap_or(0);
    let user_id: u64 = parts[2].parse().unwrap_or(0);
    let clicker = q.from.id;
    let member = bot.get_chat_member(ChatId(chat_id), clicker).await?;
    let is_admin = matches!(
        member.kind,
        ChatMemberKind::Administrator(_) | ChatMemberKind::Owner(_)
    );
    if !is_admin {
        bot.answer_callback_query(q.id.clone())
            .text("Only group admins can use this.")
            .show_alert(true)
            .await?;
        return Ok(());
    }
    bot.ban_chat_member(ChatId(chat_id), UserId(user_id)).await?;
    bot.answer_callback_query(q.id.clone())
        .text("User banned.")
        .await?;
    Ok(())
}
