use anyhow::Result;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

const HELP_TEXT: &str = "Here is the list of available commands:\n\n*User Commands*:\n• /check <@username|ID>: Check whether a user belongs to blacklisted cheat groups (works in private, groups, and inline query).\n• /submit <invite_link>: Submit a suspicious cheat group for admin review.\n\n*Admin Commands*:\n• /add <link|ID> [index]: Instantly add a cheat group to the database and assign it to an MTProto account.\n• /add_account: Connect a new MTProto userbot account to the scanning pool.";

pub async fn handle_help(bot: Bot, msg: Message) -> Result<()> {
    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("Back", "start_menu"),
    ]]);
    bot.send_message(msg.chat.id, HELP_TEXT)
        .parse_mode(teloxide::types::ParseMode::MarkdownV2)
        .reply_markup(keyboard)
        .await?;
    Ok(())
}

pub async fn handle_help_callback(bot: Bot, q: CallbackQuery) -> Result<()> {
    bot.answer_callback_query(q.id.clone()).await?;
    if let Some(msg) = q.message.as_ref() {
        let chat_id = msg.chat().id;
        let msg_id = msg.id();
        let keyboard = InlineKeyboardMarkup::new(vec![vec![
            InlineKeyboardButton::callback("Back", "start_menu"),
        ]]);
        bot.edit_message_text(chat_id, msg_id, HELP_TEXT)
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .reply_markup(keyboard)
            .await?;
    }
    Ok(())
}
