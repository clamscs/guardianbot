use serde::{Deserialize, Serialize};
use teloxide::types::UserId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckCachePayload {
    pub requester_id: UserId,
    pub target_id: UserId,
    pub target_first_name: String,
    pub target_last_name: Option<String>,
    pub target_username: Option<String>,
    pub matches: Vec<MatchEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchEntry {
    pub chat_id: String,
    pub title: String,
    pub username: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Clone)]
pub struct JoinTask {
    pub invite_link: String,
    pub chat_id_hint: Option<String>,
    pub requested_by: Option<UserId>,
    pub requester_username: Option<String>,
    pub requester_chat_id: Option<i64>,
    pub submission_message_id: Option<i32>,
}

#[derive(Debug, Clone, Default)]
pub struct AccountRow {
    pub phone_number: String,
    pub session_bytes: Vec<u8>,
    pub active_group_count: i64,
    pub is_active: bool,
    pub is_full: bool,
}

#[derive(Debug, Clone)]
pub struct GroupRow {
    pub chat_id: String,
    pub title: String,
    pub username: Option<String>,
    pub invite_link: Option<String>,
    pub status: GroupStatus,
    pub assigned_account_phone: Option<String>,
    pub added_by: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupStatus {
    Active,
    PendingApproval,
    PendingJoin,
}

impl GroupStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            GroupStatus::Active => "active",
            GroupStatus::PendingApproval => "pending_approval",
            GroupStatus::PendingJoin => "pending_join",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(GroupStatus::Active),
            "pending_approval" => Some(GroupStatus::PendingApproval),
            "pending_join" => Some(GroupStatus::PendingJoin),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthState {
    AwaitingPhone,
    AwaitingCode { phone: String, token_json: String },
    AwaitingPassword { phone: String, token_json: String, password_token_json: String },
}

#[derive(Debug, Clone)]
pub enum Command {
    Start,
    Help,
    Check(String),
    Submit(String),
    Add(String, Option<String>),
    AddAccount,
}
