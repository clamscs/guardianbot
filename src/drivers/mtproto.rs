use anyhow::{anyhow, Result};
use grammers_client::{Client, Config, InitParams, SignInError};
use grammers_session::MemorySession;
use std::sync::Arc;

pub async fn build_client(api_id: i32, api_hash: &str) -> Result<Client> {
    let session = Arc::new(MemorySession::default());
    let client = Client::connect(Config {
        session,
        api_id,
        api_hash: api_hash.to_string(),
        params: InitParams::default(),
    })
    .await?;
    Ok(client)
}

pub async fn login_flow(
    api_id: i32,
    api_hash: &str,
    phone: &str,
) -> Result<(Client, grammers_client::types::LoginToken)> {
    let client = build_client(api_id, api_hash).await?;
    let token = client
        .request_login_code(phone)
        .await
        .map_err(|e| anyhow!("request code failed: {:?}", e))?;
    Ok((client, token))
}

pub async fn submit_code(
    client: &Client,
    token: &grammers_client::types::LoginToken,
    code: &str,
) -> Result<Option<grammers_client::types::PasswordToken>> {
    match client.sign_in(token, code).await {
        Ok(_user) => Ok(None),
        Err(SignInError::PasswordRequired(pw)) => Ok(Some(pw)),
        Err(e) => Err(anyhow!("sign in failed: {:?}", e)),
    }
}

pub async fn submit_password(
    client: &Client,
    password_token: grammers_client::types::PasswordToken,
    password: &str,
) -> Result<()> {
    client
        .check_password(password_token, password)
        .await
        .map_err(|e| anyhow!("password check failed: {:?}", e))?;
    Ok(())
}

pub async fn session_bytes(client: &Client) -> Result<Vec<u8>> {
    let session = client.session();
    let data = session.save();
    Ok(data)
}
