use reqwest::Response;

#[derive(Debug)]
pub struct ProxyError(pub String);

impl std::fmt::Display for ProxyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl std::error::Error for ProxyError {}

pub async fn create_draft(
    client: &reqwest::Client,
    base_url: &str,
    access_token_placeholder: &str,
    body: Vec<u8>,
) -> Result<Response, ProxyError> {
    client
        .post(format!("{base_url}/v1.0/me/messages"))
        .bearer_auth(access_token_placeholder)
        .header("content-type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|error| ProxyError(format!("upstream request failed: {error:#}")))
}

pub async fn send_draft(
    client: &reqwest::Client,
    base_url: &str,
    access_token_placeholder: &str,
    encoded_draft_id: &str,
) -> Result<Response, ProxyError> {
    client
        .post(format!(
            "{base_url}/v1.0/me/messages/{encoded_draft_id}/send"
        ))
        .bearer_auth(access_token_placeholder)
        .header("content-length", "0")
        .send()
        .await
        .map_err(|error| ProxyError(format!("upstream request failed: {error:#}")))
}
