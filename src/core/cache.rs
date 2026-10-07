use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::include::constants::CACHE_PREFIX;
use crate::include::types::CheckCachePayload;

pub struct CheckCache {
    inner: DashMap<String, (CheckCachePayload, Instant)>,
    ttl: Duration,
}

impl CheckCache {
    pub fn new(ttl_secs: u64) -> Arc<Self> {
        Arc::new(Self {
            inner: DashMap::new(),
            ttl: Duration::from_secs(ttl_secs),
        })
    }

    pub fn insert(&self, payload: CheckCachePayload) -> String {
        let short = Uuid::new_v4().simple().to_string()[..16].to_string();
        let key = format!("{}{}", CACHE_PREFIX, short);
        self.inner.insert(key.clone(), (payload, Instant::now()));
        key
    }

    pub fn get(&self, key: &str) -> Option<CheckCachePayload> {
        let entry = self.inner.get(key)?;
        if entry.1.elapsed() > self.ttl {
            drop(entry);
            self.inner.remove(key);
            return None;
        }
        Some(entry.0.clone())
    }

    pub fn evict_expired(&self) {
        let now = Instant::now();
        self.inner.retain(|_, (_, inserted)| now.duration_since(*inserted) <= self.ttl);
    }
}
