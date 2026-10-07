use anyhow::Result;
use grammers_client::InvocationError;
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

use crate::include::constants::FLOOD_EXTRA_DELAY_SECS;

pub async fn with_retry<F, Fut, T>(max_attempts: usize, mut f: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, InvocationError>>,
{
    let mut attempt = 0;
    loop {
        attempt += 1;
        match f().await {
            Ok(v) => return Ok(v),
            Err(InvocationError::Rpc(err)) if err.code == 420 => {
                let wait = err.value.unwrap_or(30) as u64 + FLOOD_EXTRA_DELAY_SECS;
                tracing::warn!(seconds = wait, "flood wait");
                if attempt >= max_attempts {
                    return Err(anyhow::anyhow!("flood wait exceeded retries"));
                }
                sleep(Duration::from_secs(wait)).await;
            }
            Err(e) => {
                if attempt >= max_attempts {
                    return Err(anyhow::anyhow!("rpc error: {:?}", e));
                }
                let backoff = 2u64.pow(attempt as u32).min(60);
                sleep(Duration::from_secs(backoff)).await;
            }
        }
    }
}
