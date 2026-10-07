use anyhow::Result;
use rand::Rng;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;

use crate::core::pool::PoolHandle;
use crate::include::types::JoinTask;
use crate::net::rpc::execute_join;

pub struct JoinQueueWorker {
    pool: PoolHandle,
}

impl JoinQueueWorker {
    pub fn spawn(pool: PoolHandle, capacity: usize) -> mpsc::Sender<JoinTask> {
        let (tx, mut rx) = mpsc::channel::<JoinTask>(capacity);
        let worker = Arc::new(Self { pool });
        tokio::spawn(async move {
            while let Some(task) = rx.recv().await {
                if let Err(e) = worker.process(task).await {
                    tracing::error!(error = %e, "join queue task failed");
                }
                let jitter = rand::thread_rng().gen_range(35..=80);
                sleep(Duration::from_secs(jitter)).await;
            }
        });
        tx
    }

    async fn process(&self, task: JoinTask) -> Result<()> {
        let clients = self.pool.all_clients().await;
        for (phone, client) in clients {
            match execute_join(&client, &task.invite_link).await {
                Ok(chat_id) => {
                    tracing::info!(phone = %phone, chat_id = %chat_id, "joined via queue");
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!(phone = %phone, error = %e, "join attempt failed");
                }
            }
        }
        Err(anyhow::anyhow!("all join attempts failed"))
    }
}
