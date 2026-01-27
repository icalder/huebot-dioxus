#[cfg(feature = "server")]
use chrono::Utc;
#[cfg(feature = "server")]
use futures::StreamExt;
#[cfg(feature = "server")]
use sqlx::PgPool;
use std::sync::LazyLock;
#[cfg(feature = "server")]
use tokio::sync::OnceCell;

pub mod client;
pub mod events;
pub mod models;
#[cfg(feature = "server")]
pub mod eventcache;
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
static SENSORS_CACHE: tokio::sync::RwLock<
    Option<(Vec<client::CompositeSensor>, chrono::DateTime<Utc>)>,
> = tokio::sync::RwLock::const_new(None);

#[cfg(feature = "server")]
pub async fn get_sensors_cached() -> Result<Vec<client::CompositeSensor>, ServerFnError> {
    let mut sensors = {
        let cache = SENSORS_CACHE.read().await;
        if let Some((sensors, timestamp)) = &*cache {
            if (Utc::now() - *timestamp).num_minutes() < 5 {
                Some(sensors.clone())
            } else {
                None
            }
        } else {
            None
        }
    };

    if sensors.is_none() {
        // Cache miss or expired
        let fresh_sensors: Vec<client::CompositeSensor> = get_hue_client()
            .get_sensors()
            .await
            .map_err(|e| ServerFnError::new(e))?;
        let mut cache = SENSORS_CACHE.write().await;
        *cache = Some((fresh_sensors.clone(), Utc::now()));
        sensors = Some(fresh_sensors);
    }

    let mut sensors = sensors.unwrap();

    // Backfill history from EventCache
    let events = EVENT_CACHE.get_all();
    for event_str in events {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&event_str) {
            if let Some(event) = client::HueEvent::from_json(&v) {
                let owner_rid = event.owner_rid();
                let resource_id = event.resource_id();

                for s in sensors.iter_mut() {
                    let is_owner = owner_rid == Some(s.device_id.as_str());
                    let matches_resource = resource_id.is_some()
                        && (s.motion.as_ref().map(|m| m.id.as_str()) == resource_id
                            || s.temperature.as_ref().map(|t| t.id.as_str()) == resource_id
                            || s.light.as_ref().map(|l| l.id.as_str()) == resource_id);

                    if is_owner || matches_resource {
                        s.apply_event(&event);
                    }
                }
            }
        }
    }

    Ok(sensors)
}

#[cfg(feature = "server")]
pub async fn get_db_pool() -> Result<PgPool, ServerFnError> {
    DB_POOL
        .get_or_try_init(|| async {
            let db_url = std::env::var("DATABASE_URL")
                .map_err(|_| ServerFnError::new("DATABASE_URL must be set"))?;
            PgPool::connect(&db_url)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))
        })
        .await
        .cloned()
}

#[cfg(feature = "server")]
static EVENT_CHANNEL: LazyLock<tokio::sync::broadcast::Sender<String>> = LazyLock::new(|| {
    let (tx, _) = tokio::sync::broadcast::channel(100);
    tx
});

#[cfg(feature = "server")]
static EVENT_CACHE: LazyLock<eventcache::EventCache> =
    LazyLock::new(|| eventcache::EventCache::new(30));

#[cfg(feature = "server")]
static EVENT_LOOP_STARTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(feature = "server")]
fn start_event_listener() {
    if !EVENT_LOOP_STARTED.load(std::sync::atomic::Ordering::Relaxed) {
        if !EVENT_LOOP_STARTED.swap(true, std::sync::atomic::Ordering::SeqCst) {
            tokio::spawn(async move {
                let client = get_hue_client();
                let tx = &EVENT_CHANNEL;

                loop {
                    println!("Connecting to Hue Bridge event stream...");
                    match client.event_stream().await {
                        Ok(stream) => {
                            println!("Connected to Hue Bridge event stream.");
                            futures::pin_mut!(stream);
                            while let Some(msg) = stream.next().await {
                                EVENT_CACHE.add(msg.clone());
                                
                                // Update sensor cache
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&msg) {
                                    if let Some(event) = client::HueEvent::from_json(&v) {
                                        let mut cache = SENSORS_CACHE.write().await;
                                        if let Some((ref mut sensors, _)) = *cache {
                                            let owner_rid = event.owner_rid();
                                            let resource_id = event.resource_id();

                                            for s in sensors.iter_mut() {
                                                let is_owner = owner_rid == Some(s.device_id.as_str());
                                                let matches_resource = resource_id.is_some()
                                                    && (s.motion.as_ref().map(|m| m.id.as_str()) == resource_id
                                                        || s.temperature.as_ref().map(|t| t.id.as_str()) == resource_id
                                                        || s.light.as_ref().map(|l| l.id.as_str()) == resource_id);

                                                if is_owner || matches_resource {
                                                    s.apply_event(&event);
                                                }
                                            }
                                        }
                                    }
                                }

                                // Broadcast raw message
                                let _ = tx.send(msg);
                            }
                            println!("Hue Bridge event stream ended.");
                        }
                        Err(e) => {
                            println!("Error connecting to Hue Bridge event stream: {}. Retrying in 1s...", e);
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            });
        }
    }
}

use dioxus::prelude::*;

#[server(output = StreamingText)]
pub async fn hue_events(cached: bool) -> Result<dioxus::fullstack::TextStream, ServerFnError> {
    start_event_listener();

    let tx = &EVENT_CHANNEL;
    let rx = tx.subscribe();

    let cached_stream = if cached {
        let cached = EVENT_CACHE.get_all();
        futures::stream::iter(cached)
    } else {
        futures::stream::iter(Vec::new())
    };

    let stream = futures::stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(msg) => return Some((msg, rx)),
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
            }
        }
    });

    Ok(dioxus::fullstack::TextStream::new(
        cached_stream.chain(stream),
    ))
}

pub fn use_hue_event_handler(
    cached: bool,
    on_event: impl FnMut(client::HueEvent) + 'static,
    on_error: impl FnMut(String) + 'static,
) {
    use std::cell::RefCell;
    use std::rc::Rc;

    let on_event = Rc::new(RefCell::new(Some(on_event)));
    let on_error = Rc::new(RefCell::new(Some(on_error)));

    let mut is_visible = use_signal(|| {
        #[cfg(feature = "web")]
        {
            web_sys::window()
                .and_then(|w| w.document())
                .map(|d| !d.hidden())
                .unwrap_or(true)
        }
        #[cfg(not(feature = "web"))]
        {
            true
        }
    });

    let _listener = use_hook(|| {
        #[cfg(feature = "web")]
        {
            let document = web_sys::window().unwrap().document().unwrap();
            Rc::new(gloo_events::EventListener::new(
                &document,
                "visibilitychange",
                move |_| {
                    let hidden = web_sys::window()
                        .and_then(|w| w.document())
                        .map(|d| d.hidden())
                        .unwrap_or(false);
                    is_visible.set(!hidden);
                },
            ))
        }
        #[cfg(not(feature = "web"))]
        {
            Rc::new(())
        }
    });

    use_resource(move || {
        let on_event = on_event.clone();
        let on_error = on_error.clone();
        let visible = is_visible();

        async move {
            #[cfg(feature = "web")]
            {
                if !visible {
                    return;
                }

                loop {
                    match hue_events(cached).await {
                        Ok(mut stream) => {
                            while let Some(Ok(event_str)) = stream.next().await {
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&event_str) {
                                    if let Some(event) = client::HueEvent::from_json(&v) {
                                        if let Some(ref mut handler) = *on_event.borrow_mut() {
                                            handler(event);
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let msg = e.to_string();
                            if let Some(ref mut handler) = *on_error.borrow_mut() {
                                handler(msg);
                            }
                        }
                    }
                    gloo_timers::future::TimeoutFuture::new(1000).await;

                    // Re-check visibility before looping
                    if !web_sys::window()
                        .and_then(|w| w.document())
                        .map(|d| !d.hidden())
                        .unwrap_or(true)
                    {
                        break;
                    }
                }
            }
        }
    });
}
