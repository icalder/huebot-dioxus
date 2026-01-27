use chrono::{DateTime, Utc};
use progenitor::generate_api;
#[cfg(feature = "server")]
use std::collections::HashMap;
use std::ops::Deref;
#[cfg(feature = "server")]
use std::sync::Arc;
#[cfg(feature = "server")]
use tokio::sync::Semaphore;

#[cfg(feature = "server")]
use crate::hue::client::types::ErrorResponse;

// Generate Hue OpenAPI bindings
// NB re-evaluated when the openapi spec file changes
generate_api!("hue-openapi.yaml");

pub use crate::hue::events::HueEvent;
pub use crate::hue::models::*;

/// Extended client wrapper that adds high-level convenience methods
/// while delegating all original client methods via Deref
pub struct ClientEx {
    inner: Client,
    base_url: String,
    #[cfg(feature = "server")]
    semaphore: Arc<Semaphore>,
}

impl Deref for ClientEx {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl ClientEx {
    pub fn parse_date(s: &Option<String>) -> DateTime<Utc> {
        s.as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now)
    }

    #[cfg(feature = "server")]
    fn get_owner_rid(
        owner: &Option<crate::hue::client::types::ResourceIdentifier>,
    ) -> Option<String> {
        owner
            .as_ref()
            .and_then(|o| o.rid.as_ref())
            .map(|s| s.to_string())
    }
}

#[cfg(feature = "server")]
impl ClientEx {
    pub fn new(client: Client, base_url: String) -> Self {
        Self {
            inner: client,
            base_url,
            semaphore: Arc::new(Semaphore::new(3)),
        }
    }

    async fn retry<T, E, F, Fut>(&self, f: F) -> Result<T, E>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        let mut attempts = 0;
        loop {
            // Acquire semaphore permit before making the request
            let _permit = self
                .semaphore
                .acquire()
                .await
                .map_err(|_| {
                    // This only happens if semaphore is closed, which shouldn't happen
                    panic!("Bridge semaphore closed unexpectedly");
                })
                .unwrap();

            match f().await {
                Ok(val) => return Ok(val),
                Err(e) if attempts < 5 => {
                    attempts += 1;
                    let delay = attempts * attempts * 100; // 100, 400, 900, 1600, 2500ms
                    println!(
                        "Transient Hue error (attempt {}): {}. Retrying in {}ms",
                        attempts, e, delay
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(delay as u64)).await;
                }
                Err(e) => {
                    println!(
                        "Persistent Hue error after {} attempts: {}",
                        attempts + 1,
                        e
                    );
                    return Err(e);
                }
            }
        }
    }

    /// Fetch all sensors and group them by device into CompositeSensors
    pub async fn get_sensors(&self) -> Result<Vec<CompositeSensor>, Error<ErrorResponse>> {
        let (motion_res, temp_res, light_res, devices_res) = self
            .retry(|| async {
                tokio::try_join!(
                    self.inner.get_motion_sensors(),
                    self.inner.get_temperatures(),
                    self.inner.get_light_levels(),
                    self.inner.get_devices()
                )
            })
            .await?;

        let motion_response = motion_res;
        let temp_response = temp_res;
        let light_response = light_res;
        let devices_response = devices_res;

        let mut device_map = self.init_device_map(&devices_response.data);

        // Populate motion data
        for m in &motion_response.data {
            if let (Some(id), Some(owner_rid)) = (&m.id, Self::get_owner_rid(&m.owner)) {
                if let Some(cs) = device_map.get_mut(&owner_rid) {
                    if let Some(report) = m.motion.as_ref().and_then(|m| m.motion_report.as_ref()) {
                        let presence = report.motion.unwrap_or(false);
                        let last_updated = Self::parse_date(&report.changed);
                        cs.motion = Some(MotionData {
                            id: id.to_string(),
                            id_v1: m.id_v1.as_ref().map(|v| v.to_string()),
                            enabled: m.enabled.unwrap_or(true),
                            presence,
                            last_updated,
                            history: vec![(last_updated, presence)],
                        });
                    }
                }
            }
        }

        // Populate temperature data
        for t in &temp_response.data {
            if let (Some(id), Some(owner_rid)) = (&t.id, Self::get_owner_rid(&t.owner)) {
                if let Some(cs) = device_map.get_mut(&owner_rid) {
                    if let Some(report) = t
                        .temperature
                        .as_ref()
                        .and_then(|t| t.temperature_report.as_ref())
                    {
                        let temperature = report.temperature.unwrap_or(0.0);
                        let last_updated = report.changed.unwrap_or_else(Utc::now);
                        cs.temperature = Some(TemperatureData {
                            id: id.to_string(),
                            id_v1: t.id_v1.as_ref().map(|v| v.to_string()),
                            enabled: t.enabled.unwrap_or(true),
                            temperature,
                            last_updated,
                            history: vec![(last_updated, temperature)],
                        });
                    }
                }
            }
        }

        // Populate light data
        for l in &light_response.data {
            if let (Some(id), Some(owner_rid)) = (&l.id, Self::get_owner_rid(&l.owner)) {
                if let Some(cs) = device_map.get_mut(&owner_rid) {
                    if let Some(report) =
                        l.light.as_ref().and_then(|l| l.light_level_report.as_ref())
                    {
                        let light_level = report.light_level.map(|v| v as i32).unwrap_or(0);
                        let last_updated = report.changed.unwrap_or_else(Utc::now);
                        cs.light = Some(LightData {
                            id: id.to_string(),
                            id_v1: l.id_v1.as_ref().map(|v| v.to_string()),
                            enabled: l.enabled.unwrap_or(true),
                            light_level,
                            last_updated,
                            history: vec![(last_updated, light_level)],
                        });
                    }
                }
            }
        }

        for cs in device_map.values_mut() {
            cs.enabled = cs.motion.as_ref().map(|m| m.enabled).unwrap_or(true)
                && cs.temperature.as_ref().map(|t| t.enabled).unwrap_or(true)
                && cs.light.as_ref().map(|l| l.enabled).unwrap_or(true);
        }

        let mut sensors: Vec<CompositeSensor> = device_map
            .into_values()
            .filter(|cs| cs.motion.is_some() || cs.temperature.is_some() || cs.light.is_some())
            .collect();

        sensors.sort_by(|a, b| {
            b.is_outdoor
                .cmp(&a.is_outdoor)
                .then_with(|| a.name.cmp(&b.name))
        });

        Ok(sensors)
    }

    /// Builds a map of resource IDs to device names
    pub async fn get_name_map(&self) -> Result<HashMap<String, String>, Error<ErrorResponse>> {
        let (devices_res, rooms_res, zones_res, lights_res, bridge_homes_res) = self
            .retry(|| async {
                tokio::try_join!(
                    self.inner.get_devices(),
                    self.inner.get_rooms(),
                    self.inner.get_zones(),
                    self.inner.get_lights(),
                    self.inner.get_bridge_homes(),
                )
            })
            .await?;

        let mut name_map = HashMap::new();

        for device in &devices_res.data {
            Self::insert_resource_names(
                &mut name_map,
                device.id.as_ref().map(|id| id.to_string()),
                device
                    .metadata
                    .as_ref()
                    .and_then(|m| m.name.as_ref())
                    .map(|n| n.to_string()),
                &device.services,
            );
        }

        for room in &rooms_res.data {
            Self::insert_resource_names(
                &mut name_map,
                room.id.as_ref().map(|id| id.to_string()),
                room.metadata
                    .as_ref()
                    .and_then(|m| m.name.as_ref())
                    .map(|n| n.to_string()),
                &room.services,
            );
        }

        for zone in &zones_res.data {
            Self::insert_resource_names(
                &mut name_map,
                zone.id.as_ref().map(|id| id.to_string()),
                zone.metadata
                    .as_ref()
                    .and_then(|m| m.name.as_ref())
                    .map(|n| n.to_string()),
                &zone.services,
            );
        }

        for home in &bridge_homes_res.data {
            Self::insert_resource_names(
                &mut name_map,
                home.id.as_ref().map(|id| id.to_string()),
                Some("Bridge Home".to_string()),
                &home.services,
            );
        }

        for light in &lights_res.data {
            if let (Some(id), Some(metadata)) = (&light.id, &light.metadata) {
                if let Some(name) = &metadata.name {
                    name_map.insert(id.to_string(), name.to_string());
                }
            }
        }

        Ok(name_map)
    }

    /// Returns a stream of Hue events as JSON strings
    pub async fn event_stream(
        &self,
    ) -> Result<impl futures::Stream<Item = String>, reqwest::Error> {
        use futures::StreamExt;
        use tokio_util::codec::{FramedRead, LinesCodec};

        let url = format!("{}/eventstream/clip/v2", self.base_url);
        let response = self
            .inner
            .client()
            .get(url)
            .header("Accept", "text/event-stream")
            .send()
            .await?;

        let stream = response
            .bytes_stream()
            .map(|result| result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)));

        let reader = tokio_util::io::StreamReader::new(stream);
        let lines = FramedRead::new(reader, LinesCodec::new());

        let event_stream = lines.filter_map(|line_result| async move {
            let line: String = line_result.ok()?;
            if let Some(data) = line.strip_prefix("data: ") {
                if let Ok(envelopes) = serde_json::from_str::<Vec<serde_json::Value>>(data) {
                    let mut all_updates = Vec::new();
                    for mut env in envelopes {
                        if let Some(updates) = env.get_mut("data").and_then(|d| d.as_array_mut()) {
                            all_updates.extend(updates.drain(..).map(|u| u.to_string()));
                        }
                    }
                    return Some(futures::stream::iter(all_updates));
                }
            }
            None
        });

        Ok(event_stream.flatten())
    }

    // --- Private Helpers ---

    fn init_device_map(
        &self,
        devices: &[crate::hue::client::types::GetDevicesResponseDataItem],
    ) -> HashMap<String, CompositeSensor> {
        devices
            .iter()
            .filter_map(|d| {
                let id = d.id.as_ref()?.to_string();
                let name = d.metadata.as_ref()?.name.as_ref()?.to_string();
                let is_outdoor = d
                    .product_data
                    .as_ref()
                    .and_then(|pd| pd.product_name.as_ref())
                    .map(|pn| pn.to_lowercase().contains("outdoor"))
                    .unwrap_or(false);
                Some((
                    id.clone(),
                    CompositeSensor {
                        device_id: id,
                        name,
                        is_outdoor,
                        enabled: true,
                        motion: None,
                        temperature: None,
                        light: None,
                    },
                ))
            })
            .collect()
    }

    fn insert_resource_names(
        map: &mut HashMap<String, String>,
        id: Option<String>,
        name: Option<String>,
        services: &[crate::hue::client::types::ResourceIdentifier],
    ) {
        if let (Some(id), Some(name)) = (id, name) {
            map.insert(id, name.clone());
            for service in services {
                if let Some(rid) = &service.rid {
                    map.insert(rid.to_string(), name.clone());
                }
            }
        }
    }
}
