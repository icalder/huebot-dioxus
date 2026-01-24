use std::sync::LazyLock;

pub mod client;
#[cfg(feature = "server")]
pub mod tests;

#[cfg(feature = "server")]
static HUE_CLIENT: LazyLock<client::ClientEx> = LazyLock::new(|| {
    let ip = std::env::var("HUE_IP").expect("HUE_IP environment variable must be set");
    let key = std::env::var("HUE_KEY").expect("HUE_KEY environment variable must be set");

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "hue-application-key",
        reqwest::header::HeaderValue::from_str(&key).unwrap(),
    );

    let reqwest_client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .default_headers(headers)
        .build()
        .unwrap();

    let base_url = format!("https://{}", ip);
    let client = client::Client::new_with_client(&base_url, reqwest_client);
    client::ClientEx::new(client, base_url)
});

#[cfg(feature = "server")]
pub fn get_hue_client() -> &'static client::ClientEx {
    &HUE_CLIENT
}

use dioxus::prelude::*;

#[server(output = StreamingText)]
pub async fn hue_events() -> Result<dioxus::fullstack::TextStream, ServerFnError> {
    let client = get_hue_client();
    
    let stream = client
        .event_stream()
        .await
        .map_err(|e| ServerFnError::new(e))?;
        
    Ok(dioxus::fullstack::TextStream::new(stream))
}
