#[cfg(feature = "server")]
use chrono::Utc;
#[cfg(feature = "server")]
use sqlx::PgPool;
use std::sync::LazyLock;
#[cfg(feature = "server")]
use tokio::sync::OnceCell;

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
static SENSORS_CACHE: tokio::sync::RwLock<
    Option<(Vec<client::CompositeSensor>, chrono::DateTime<Utc>)>,
> = tokio::sync::RwLock::const_new(None);

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
    let sensors: Vec<client::CompositeSensor> = get_hue_client()
        .get_sensors()
        .await
        .map_err(|e| ServerFnError::new(e))?;
    let mut cache = SENSORS_CACHE.write().await;
    *cache = Some((sensors.clone(), Utc::now()));
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
static EVENT_LOOP_STARTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(feature = "server")]
fn start_event_listener() {
    use futures::StreamExt;

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
                                // Ignore SendError (happens if no subscribers)
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
pub async fn hue_events() -> Result<dioxus::fullstack::TextStream, ServerFnError> {
    start_event_listener();

    let tx = &EVENT_CHANNEL;
    let rx = tx.subscribe();

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

    Ok(dioxus::fullstack::TextStream::new(stream))
}

pub fn use_hue_event_handler(
    on_event: impl FnMut(String) + 'static,
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
            if !visible {
                return;
            }

            loop {
                match hue_events().await {
                    Ok(mut stream) => {
                        while let Some(Ok(event_str)) = stream.next().await {
                            if let Some(ref mut handler) = *on_event.borrow_mut() {
                                handler(event_str);
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
    });
}
