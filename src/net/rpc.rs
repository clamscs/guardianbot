use anyhow::{anyhow, Result};
use grammers_client::Client;
use grammers_tl_types::enums::InputUser;
use grammers_tl_types::functions::contacts::ResolveUsername;
use grammers_tl_types::functions::messages::{GetCommonChats, ImportChatInvite};
use grammers_tl_types::types::InputUser as InputUserType;

use crate::include::constants::COMMON_CHATS_LIMIT;
use crate::net::retry::with_retry;

pub async fn execute_join(client: &Client, invite_link: &str) -> Result<String> {
    let hash = invite_link
        .rsplit('/')
        .next()
        .ok_or_else(|| anyhow!("invalid invite link"))?
        .trim_start_matches('+')
        .to_string();
    let req = ImportChatInvite { hash };
    let _updates = with_retry(3, || {
        let c = client.clone();
        let r = req.clone();
        async move { c.invoke(&r).await }
    })
    .await?;
    Ok(invite_link.to_string())
}

pub async fn get_common_chats(
    client: &Client,
    user_id: i64,
    access_hash: i64,
) -> Result<Vec<i64>> {
    let mut all = Vec::new();
    let mut max_id: i64 = 0;
    loop {
        let input_user = InputUser::User(InputUserType {
            user_id,
            access_hash,
        });
        let req = GetCommonChats {
            user_id: input_user,
            max_id,
            limit: COMMON_CHATS_LIMIT,
        };
        let result = with_retry(3, || {
            let c = client.clone();
            let r = req.clone();
            async move { c.invoke(&r).await }
        })
        .await?;
        let chats = match result {
            grammers_tl_types::enums::messages::Chats::Chats(c) => c.chats,
            grammers_tl_types::enums::messages::Chats::Slice(s) => s.chats,
        };
        if chats.is_empty() {
            break;
        }
        let mut batch_max = max_id;
        for chat in &chats {
            if let Some(id) = extract_chat_id(chat) {
                all.push(id);
                if id > batch_max {
                    batch_max = id;
                }
            }
        }
        if chats.len() < COMMON_CHATS_LIMIT as usize {
            break;
        }
        max_id = batch_max;
    }
    Ok(all)
}

fn extract_chat_id(chat: &grammers_tl_types::enums::Chat) -> Option<i64> {
    match chat {
        grammers_tl_types::enums::Chat::Chat(c) => Some(-c.id),
        grammers_tl_types::enums::Chat::Channel(c) => Some(-1000000000000 - c.id),
        grammers_tl_types::enums::Chat::ChannelForbidden(c) => Some(-1000000000000 - c.id),
        _ => None,
    }
}

pub async fn resolve_user(client: &Client, username: &str) -> Result<(i64, i64)> {
    let req = ResolveUsername {
        username: username.to_string(),
    };
    let res = client
        .invoke(&req)
        .await
        .map_err(|e| anyhow!("resolve failed: {:?}", e))?;
    let resolved = match res {
        grammers_tl_types::enums::contacts::ResolvedPeer::Peer(r) => r,
    };
    let user_id = match resolved.peer {
        grammers_tl_types::enums::Peer::User(u) => u.user_id,
        _ => return Err(anyhow!("not a user")),
    };
    let access_hash = resolved
        .users
        .iter()
        .find_map(|u| match u {
            grammers_tl_types::enums::User::User(inner) if inner.id == user_id => {
                inner.access_hash
            }
            _ => None,
        })
        .ok_or_else(|| anyhow!("user access hash missing"))?;
    Ok((user_id, access_hash))
}

pub async fn get_participant_role(
    _client: &Client,
    _chat_id: i64,
    _user_id: i64,
) -> Result<Option<String>> {
    Ok(None)
}

pub fn role_label(_role: &str) -> Option<String> {
    None
}
