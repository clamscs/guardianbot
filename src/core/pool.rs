use anyhow::{anyhow, Result};
use grammers_client::Client;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::include::constants::{MAX_GROUPS_PREMIUM, MAX_GROUPS_REGULAR};

pub type ClientPool = Arc<RwLock<HashMap<String, Client>>>;

#[derive(Clone)]
pub struct PoolHandle {
    pool: ClientPool,
    counter: Arc<AtomicUsize>,
}

impl PoolHandle {
    pub fn new() -> Self {
        Self {
            pool: Arc::new(RwLock::new(HashMap::new())),
            counter: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub async fn insert(&self, phone: String, client: Client) {
        let mut guard = self.pool.write().await;
        guard.insert(phone, client);
        self.counter.fetch_add(1, Ordering::Relaxed);
    }

    pub async fn remove(&self, phone: &str) -> Option<Client> {
        let mut guard = self.pool.write().await;
        let removed = guard.remove(phone);
        if removed.is_some() {
            self.counter.fetch_sub(1, Ordering::Relaxed);
        }
        removed
    }

    pub async fn get(&self, phone: &str) -> Option<Client> {
        let guard = self.pool.read().await;
        guard.get(phone).cloned()
    }

    pub async fn all_clients(&self) -> Vec<(String, Client)> {
        let guard = self.pool.read().await;
        guard.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    pub async fn len(&self) -> usize {
        self.pool.read().await.len()
    }

    pub fn next_index(&self) -> usize {
        self.counter.load(Ordering::Relaxed)
    }

    pub async fn select_lowest_load(&self, counts: &HashMap<String, i64>, is_premium: bool) -> Result<String> {
        let guard = self.pool.read().await;
        let threshold = if is_premium { MAX_GROUPS_PREMIUM } else { MAX_GROUPS_REGULAR } as i64;
        let mut best: Option<(String, i64)> = None;
        for phone in guard.keys() {
            let count = counts.get(phone).copied().unwrap_or(0);
            if count >= threshold {
                continue;
            }
            match &best {
                Some((_, c)) if *c <= count => {}
                _ => best = Some((phone.clone(), count)),
            }
        }
        best.map(|(p, _)| p).ok_or_else(|| anyhow!("no available account"))
    }

    pub async fn all_phones(&self) -> Vec<String> {
        self.pool.read().await.keys().cloned().collect()
    }
}

impl Default for PoolHandle {
    fn default() -> Self {
        Self::new()
    }
}
