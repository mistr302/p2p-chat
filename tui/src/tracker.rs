use p2pchat_types::{Keypair, PeerSearchResponse, RegisterResponse, UsernamePayload};
use p2pchat_types::signable::sign;

#[derive(Debug, thiserror::Error)]
pub enum TrackerError {
    #[error("Request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("Username not available")]
    UsernameNotAvailable,
    #[error("Username not found")]
    UsernameNotFound,
    #[error("Server error")]
    ServerError,
    #[error("Failed to parse response: {0}")]
    ParseError(String),
}

pub async fn check_username_availability(
    client: &reqwest::Client,
    username: String,
    http_tracker_domain: String,
) -> Result<bool, TrackerError> {
    let url = format!("http://{}/find-by-name?q={}", http_tracker_domain, username);
    let response = client.get(&url).send().await?;

    if response.status().is_success() {
        // Username exists, so it's NOT available
        Ok(false)
    } else if response.status().is_server_error() {
        Err(TrackerError::ServerError)
    } else {
        // Username not found, so it's available
        Ok(true)
    }
}

pub async fn register_username(
    client: &reqwest::Client,
    keys: &Keypair,
    http_tracker_domain: String,
    username: String,
) -> Result<RegisterResponse, TrackerError> {
    let payload = UsernamePayload { username };
    let signed = sign(payload, keys);

    let url = format!("http://{}/register", http_tracker_domain);
    let response = client.post(&url).json(&signed).send().await?;

    if response.status().is_success() {
        response
            .json::<RegisterResponse>()
            .await
            .map_err(|e| TrackerError::ParseError(e.to_string()))
    } else if response.status().is_server_error() {
        Err(TrackerError::ServerError)
    } else {
        Err(TrackerError::UsernameNotAvailable)
    }
}
