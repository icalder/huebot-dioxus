use std::sync::LazyLock;
#[cfg(feature = "server")]
use tokio::sync::OnceCell;
#[cfg(feature = "server")]
use sqlx::PgPool;
#[cfg(feature = "server")]
use chrono::Utc;

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

    let base_url = format!("https://{}", ip.trim().trim_end_matches('/'));
    let client = client::Client::new_with_client(&base_url, reqwest_client);
    client::ClientEx::new(client, base_url)
});

#[cfg(feature = "server")]
pub fn get_hue_client() -> &'static client::ClientEx {
    &HUE_CLIENT
}

#[cfg(feature = "server")]
static DB_POOL: OnceCell<PgPool> = OnceCell::const_new();

#[cfg(feature = "server")]
static SENSORS_CACHE: tokio::sync::RwLock<Option<(Vec<client::CompositeSensor>, chrono::DateTime<Utc>)>> = tokio::sync::RwLock::const_new(None);

#[cfg(feature = "server")]
pub async fn get_sensors_cached() -> Result<Vec<client::CompositeSensor>, ServerFnError> {
    {
        let cache = SENSORS_CACHE.read().await;
        if let Some((sensors, timestamp)) = &*cache {
            if (Utc::now() - *timestamp).num_minutes() < 5 {
                return Ok(sensors.clone());
            }
        }
    }
    
    // Cache miss or expired
    let sensors: Vec<client::CompositeSensor> = get_hue_client().get_sensors().await.map_err(|e| ServerFnError::new(e))?;
    let mut cache = SENSORS_CACHE.write().await;
    *cache = Some((sensors.clone(), Utc::now()));
    Ok(sensors)
}

#[cfg(feature = "server")]
pub async fn get_db_pool() -> Result<PgPool, ServerFnError> {
    DB_POOL.get_or_try_init(|| async {
        let db_url = std::env::var("DATABASE_URL").map_err(|_| ServerFnError::new("DATABASE_URL must be set"))?;
        PgPool::connect(&db_url).await.map_err(|e| ServerFnError::new(e.to_string()))
    }).await.cloned()
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