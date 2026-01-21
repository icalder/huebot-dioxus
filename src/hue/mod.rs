use std::sync::LazyLock;

pub mod client;
#[cfg(feature = "server")]
pub mod tests;

#[cfg(feature = "server")]
static HUE_CLIENT: LazyLock<client::Client> = LazyLock::new(|| {
    let ip = "192.168.1.107";
    let key = "yAqYgN-3scCv858Ed5YIvWqONSSBo-7IMOUIuqNE";

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "hue-application-key",
        reqwest::header::HeaderValue::from_str(key).unwrap(),
    );

    let reqwest_client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .default_headers(headers)
        .build()
        .unwrap();

    client::Client::new_with_client(&format!("https://{}", ip), reqwest_client)
});

#[cfg(feature = "server")]
pub fn get_hue_client() -> &'static client::Client {
    &HUE_CLIENT
}
