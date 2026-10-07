mod core;
mod drivers;
mod fs;
mod handlers;
mod include;
mod net;

use anyhow::Result;
use std::env;
use std::sync::{Arc, Mutex};
use teloxide::dispatching::UpdateFilterExt;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

use crate::core::cache::CheckCache;
use crate::core::pool::PoolHandle;
use crate::core::queue::JoinQueueWorker;
use crate::drivers::bot::build_bot;
use crate::fs::schema::{open_accounts_db, open_groups_db};
use crate::include::constants::*;
use crate::include::types::{AuthState, JoinTask};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Bot commands")]
enum BotCommand {
    #[command(description = "Start")]
    Start,
    #[command(description = "Help")]
    Help,
    #[command(description = "Check user")]
    Check(String),
    #[command(description = "Submit group")]
    Submit(String),
    #[command(description = "Add group")]
    Add(String),
    #[command(description = "Add account")]
    AddAccount,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let bot = build_bot();
    let api_id: i32 = env::var(API_ID_ENV)?.parse()?;
    let api_hash = env::var(API_HASH_ENV)?;

    let accounts_conn = Arc::new(Mutex::new(open_accounts_db(std::path::Path::new(ACCOUNTS_DB_PATH))?));
    let groups_conn = Arc::new(Mutex::new(open_groups_db(std::path::Path::new(GROUPS_DB_PATH))?));

    let pool = PoolHandle::new();
    let cache = CheckCache::new(CACHE_TTL_SECS);
    let queue_tx = JoinQueueWorker::spawn(pool.clone(), JOIN_QUEUE_CAPACITY);
    let auth_states: Arc<dashmap::DashMap<ChatId, AuthState>> = Arc::new(dashmap::DashMap::new());

    {
        let conn = accounts_conn.lock().map_err(|e| anyhow::anyhow!("lock: {}", e))?;
        let accounts = crate::fs::accounts::load_active_accounts(&conn)?;
        drop(conn);
        for acc in accounts {
            match crate::drivers::mtproto::build_client(api_id, &api_hash).await {
                Ok(c) => pool.insert(acc.phone_number.clone(), c).await,
                Err(e) => tracing::warn!(phone = %acc.phone_number, error = %e, "pool load failed"),
            }
        }
    }

    let pool_msg = pool.clone();
    let cache_msg = cache.clone();
    let accounts_msg = accounts_conn.clone();
    let groups_msg = groups_conn.clone();
    let auth_msg = auth_states.clone();
    let queue_msg = queue_tx.clone();
    let api_hash_msg = api_hash.clone();

    let cache_cb = cache.clone();
    let groups_cb = groups_conn.clone();

    let pool_cm = pool.clone();
    let groups_cm = groups_conn.clone();

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_command::<BotCommand>()
                .endpoint(move |bot: Bot, msg: Message, cmd: BotCommand| {
                    let pool = pool_msg.clone();
                    let cache = cache_msg.clone();
                    let accounts = accounts_msg.clone();
                    let groups = groups_msg.clone();
                    let auth = auth_msg.clone();
                    let queue = queue_msg.clone();
                    let api_hash = api_hash_msg.clone();
                    async move {
                        dispatch_command(
                            bot, msg, cmd, pool, cache, accounts, groups, auth, queue, api_id, api_hash,
                        )
                        .await
                    }
                }),
        )
        .branch(
            Update::filter_callback_query().endpoint(move |bot: Bot, q: CallbackQuery| {
                let cache = cache_cb.clone();
                let groups = groups_cb.clone();
                async move {
                    let data = q.data.clone().unwrap_or_default();
                    if data == "help_menu" {
                        handlers::help::handle_help_callback(bot, q).await?;
                    } else if data.starts_with(CACHE_PREFIX) {
                        handlers::check::handle_check_callback(bot, q, cache).await?;
                    } else if data.starts_with("submit_") {
                        handlers::submit::handle_submit_callback(bot, q, groups).await?;
                    } else if data.starts_with("ban_user:") {
                        handlers::antiraid::handle_ban_callback(bot, q).await?;
                    } else {
                        bot.answer_callback_query(q.id.clone()).await?;
                    }
                    Ok::<(), anyhow::Error>(())
                }
            }),
        )
        .branch(
            Update::filter_inline_query().endpoint(move |bot: Bot, q: InlineQuery| async move {
                let _ = bot.answer_inline_query(q.id, vec![]).await;
                Ok::<(), anyhow::Error>(())
            }),
        )
        .branch(
            Update::filter_chat_member().endpoint(move |bot: Bot, update: ChatMemberUpdated| {
                let pool = pool_cm.clone();
                let groups = groups_cm.clone();
                async move { handlers::antiraid::handle_chat_member(bot, update, pool, groups).await }
            }),
        );

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn dispatch_command(
    bot: Bot,
    msg: Message,
    cmd: BotCommand,
    pool: PoolHandle,
    cache: Arc<CheckCache>,
    accounts: Arc<Mutex<rusqlite::Connection>>,
    groups: Arc<Mutex<rusqlite::Connection>>,
    auth: Arc<dashmap::DashMap<ChatId, AuthState>>,
    queue: tokio::sync::mpsc::Sender<JoinTask>,
    api_id: i32,
    api_hash: String,
) -> Result<()> {
    match cmd {
        BotCommand::Start => handlers::start::handle_start(bot, msg).await,
        BotCommand::Help => handlers::help::handle_help(bot, msg).await,
        BotCommand::Check(args) => {
            handlers::check::handle_check(bot, msg, args, pool, cache, groups).await
        }
        BotCommand::Submit(args) => handlers::submit::handle_submit(bot, msg, args, queue).await,
        BotCommand::Add(args) => {
            handlers::add::handle_add(bot, msg, args, pool, groups, accounts, queue).await
        }
        BotCommand::AddAccount => {
            let owner_id: u64 = env::var(OWNER_ID_ENV)
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let from_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);
            if from_id != owner_id {
                bot.send_message(msg.chat.id, "Unauthorized.").await?;
                return Ok(());
            }
            auth.insert(msg.chat.id, AuthState::AwaitingPhone);
            bot.send_message(msg.chat.id, "Send the international phone number starting with +.")
                .await?;
            let _ = (api_id, api_hash);
            Ok(())
        }
    }
}
