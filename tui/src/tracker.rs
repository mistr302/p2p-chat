use p2pchat_types::{Keypair, RegisterResponse, UsernamePayload};

#[derive(Debug, thiserror::Error)]
pub enum TrackerError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
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
    let response = client.get(&url).send().await
        .map_err(|e| TrackerError::ConnectionFailed(e.to_string()))?;

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

    // Format the request to match server expectations
    let message = serde_json::to_string(&payload)
        .map_err(|e| TrackerError::ParseError(e.to_string()))?;

    let public_key = keys.public().encode_protobuf();
    let signature = keys.sign(message.as_bytes())
        .map_err(|e| TrackerError::ParseError(e.to_string()))?;

    let request_body = serde_json::json!({
        "public_key": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &public_key),
        "message": message,
        "signature": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &signature),
    });

    let url = format!("http://{}/register", http_tracker_domain);
    let response = client.post(&url).json(&request_body).send().await
        .map_err(|e| TrackerError::ConnectionFailed(e.to_string()))?;

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
