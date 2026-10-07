use anyhow::Result;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub async fn handle_start(bot: Bot, msg: Message) -> Result<()> {
    if msg.chat.is_private() {
        let name = msg
            .from
            .as_ref()
            .map(|u| u.first_name.clone())
            .unwrap_or_default();
        let me = bot.get_me().await?;
        let username = me.username().to_string();
        let text = format!(
            "Hey there {}!\n\nI am an automated protection bot designed to scan and detect users associated with blacklisted cheat groups.\n\nAdd me to your group to keep it safe, or use the buttons below to explore my features!",
            name
        );
        let add_url = format!("https://t.me/{}?startgroup=true", username);
        let keyboard = InlineKeyboardMarkup::new(vec![
            vec![InlineKeyboardButton::url("Add me to your group", add_url.parse()?)],
            vec![InlineKeyboardButton::callback("Help & Commands", "help_menu")],
        ]);
        bot.send_message(msg.chat.id, text)
            .reply_markup(keyboard)
            .await?;
    }
    Ok(())
}
