use anyhow::Result;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode};

use crate::include::types::JoinTask;
use crate::include::constants::OWNER_ID_ENV;

pub async fn handle_submit(
    bot: Bot,
    msg: Message,
    args: String,
    queue_tx: tokio::sync::mpsc::Sender<JoinTask>,
) -> Result<()> {
    let link = args.trim().to_string();
    if link.is_empty() {
        bot.send_message(msg.chat.id, "Usage: /submit <invite_link>").await?;
        return Ok(());
    }
    let owner_id: i64 = std::env::var(OWNER_ID_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    if owner_id == 0 {
        bot.send_message(msg.chat.id, "Owner not configured.").await?;
        return Ok(());
    }
    let requester = msg.from.as_ref();
    let requester_username = requester
        .and_then(|u| u.username.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let requester_id = requester.map(|u| u.id.0 as i64).unwrap_or(0);
    let text = format!(
        "❗ *New group submission*\n*Requested by*: @{} (`{}`)\n*Group*: {}\n\nClick *Accept* to accept @{} submission or click *Reject* to reject @{} submission.",
        requester_username, requester_id, link, requester_username, requester_username
    );
    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("Accept ✅", format!("submit_accept:{}", link)),
        InlineKeyboardButton::callback("Reject ❌", format!("submit_reject:{}", link)),
    ]]);
    let sent = bot.send_message(ChatId(owner_id), text)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(keyboard)
        .await?;
    let task = JoinTask {
        invite_link: link.clone(),
        chat_id_hint: None,
        requested_by: msg.from.as_ref().map(|u| u.id),
        requester_username: Some(requester_username),
        requester_chat_id: Some(msg.chat.id.0),
        submission_message_id: Some(sent.id.0 as i32),
    };
    queue_tx.send(task).await?;
    Ok(())
}

pub async fn handle_submit_callback(
    bot: Bot,
    q: CallbackQuery,
    groups_conn: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
) -> Result<()> {
    let data = q.data.clone().unwrap_or_default();
    if let Some(link) = data.strip_prefix("submit_accept:") {
        {
            let conn = groups_conn.lock().map_err(|e| anyhow::anyhow!("lock: {}", e))?;
            crate::fs::groups::upsert_group(
                &conn,
                link,
                "Pending",
                None,
                Some(link),
                crate::include::types::GroupStatus::PendingJoin,
                None,
                "owner",
            )?;
        }
        bot.answer_callback_query(q.id.clone()).text("Accepted").await?;
        if let Some(msg) = q.message.as_ref() {
            let chat_id = msg.chat().id;
            let msg_id = msg.id();
            bot.edit_message_text(chat_id, msg_id, "✅ *Group added*\nThanks for submitting this group!")
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
        }
    } else if let Some(link) = data.strip_prefix("submit_reject:") {
        bot.answer_callback_query(q.id.clone()).text("Rejected").await?;
        if let Some(msg) = q.message.as_ref() {
            let chat_id = msg.chat().id;
            let msg_id = msg.id();
            bot.edit_message_text(chat_id, msg_id, "❌ Submission rejected.")
                .await?;
        }
        let _ = link;
    }
    Ok(())
}
