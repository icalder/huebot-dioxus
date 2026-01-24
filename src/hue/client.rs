use chrono::{DateTime, Utc};
use progenitor::generate_api;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Deref;

use crate::hue::client::types::ErrorResponse;

// Generate Hue OpenAPI bindings
// NB re-evaluated when the openapi spec file changes
generate_api!("hue-openapi.yaml");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MotionData {
    pub id: String,
    pub presence: bool,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemperatureData {
    pub id: String,
    pub temperature: f64,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LightData {
    pub id: String,
    pub light_level: i32,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeSensor {
    pub device_id: String,
    pub name: String,
    pub is_outdoor: bool,
    pub motion: Option<MotionData>,
    pub temperature: Option<TemperatureData>,
    pub light: Option<LightData>,
}

/// Extended client wrapper that adds high-level convenience methods
/// while delegating all original client methods via Deref
pub struct ClientEx {
    inner: Client,
    base_url: String,
}

impl Deref for ClientEx {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(feature = "server")]
impl ClientEx {
    pub fn new(client: Client, base_url: String) -> Self {
        Self {
            inner: client,
            base_url,
        }
    }

    /// Fetch all sensors and group them by device into CompositeSensors
    pub async fn get_sensors(&self) -> Result<Vec<CompositeSensor>, Error<ErrorResponse>> {
        // Fetch all sensor types and devices in parallel
        let (motion_res, temp_res, light_res, devices_res) = tokio::join!(
            self.inner.get_motion_sensors(),
            self.inner.get_temperatures(),
            self.inner.get_light_levels(),
            self.inner.get_devices()
        );

        let motion_response = motion_res?;
        let temp_response = temp_res?;
        let light_response = light_res?;
        let devices_response = devices_res?;

        // Map device IDs to their names and outdoor status
        let mut device_map: HashMap<String, CompositeSensor> = devices_response
            .data
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
                        motion: None,
                        temperature: None,
                        light: None,
                    },
                ))
            })
            .collect();

        // Helper to get owner RID from ResourceOwned
        let get_owner_rid = |owner: &Option<crate::hue::client::types::ResourceIdentifier>| {
            owner
                .as_ref()
                .and_then(|o| o.rid.as_ref())
                .map(|s| s.to_string())
        };

        // Helper to parse Hue date strings
        let parse_date = |s: &Option<String>| {
            s.as_ref()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now)
        };

        // Populate motion data
        for m in &motion_response.data {
            if let (Some(id), Some(owner_rid)) = (&m.id, get_owner_rid(&m.owner)) {
                if let Some(cs) = device_map.get_mut(&owner_rid) {
                    if let Some(report) = m.motion.as_ref().and_then(|m| m.motion_report.as_ref()) {
                        cs.motion = Some(MotionData {
                            id: id.to_string(),
                            presence: report.motion.unwrap_or(false),
                            last_updated: parse_date(&report.changed),
                        });
                    }
                }
            }
        }

        // Populate temperature data
        for t in &temp_response.data {
            if let (Some(id), Some(owner_rid)) = (&t.id, get_owner_rid(&t.owner)) {
                if let Some(cs) = device_map.get_mut(&owner_rid) {
                    if let Some(report) = t
                        .temperature
                        .as_ref()
                        .and_then(|t| t.temperature_report.as_ref())
                    {
                        cs.temperature = Some(TemperatureData {
                            id: id.to_string(),
                            temperature: report.temperature.unwrap_or(0.0),
                            last_updated: report.changed.unwrap_or_else(Utc::now),
                        });
                    }
                }
            }
        }

        // Populate light data
        for l in &light_response.data {
            if let (Some(id), Some(owner_rid)) = (&l.id, get_owner_rid(&l.owner)) {
                if let Some(cs) = device_map.get_mut(&owner_rid) {
                    if let Some(report) = l.light.as_ref().and_then(|l| l.light_level_report.as_ref())
                    {
                        cs.light = Some(LightData {
                            id: id.to_string(),
                            light_level: report.light_level.map(|v| v as i32).unwrap_or(0),
                            last_updated: report.changed.unwrap_or_else(Utc::now),
                        });
                    }
                }
            }
        }

        // Filter out devices that don't have any sensor data
        let mut sensors: Vec<CompositeSensor> = device_map
            .into_values()
            .filter(|cs| cs.motion.is_some() || cs.temperature.is_some() || cs.light.is_some())
            .collect();

        sensors.sort_by(|a, b| {
            // Sort by outdoor status (Outdoor first), then by name
            b.is_outdoor
                .cmp(&a.is_outdoor)
                .then_with(|| a.name.cmp(&b.name))
        });

        Ok(sensors)
    }

    /// Builds a map of resource IDs to device names
    pub async fn get_name_map(&self) -> Result<HashMap<String, String>, Error<ErrorResponse>> {
        let (devices_res, rooms_res, zones_res, lights_res, bridge_homes_res) = tokio::join!(
            self.inner.get_devices(),
            self.inner.get_rooms(),
            self.inner.get_zones(),
            self.inner.get_lights(),
            self.inner.get_bridge_homes(),
        );

        let devices = devices_res?;
        let rooms = rooms_res?;
        let zones = zones_res?;
        let lights = lights_res?;
        let bridge_homes = bridge_homes_res?;

        let mut name_map = HashMap::new();

        // Map devices and their services
        for device in &devices.data {
            if let (Some(id), Some(metadata)) = (&device.id, &device.metadata) {
                if let Some(name) = &metadata.name {
                    let name_str = name.to_string();
                    name_map.insert(id.to_string(), name_str.clone());

                    for service in &device.services {
                        if let Some(rid) = &service.rid {
                            name_map.insert(rid.to_string(), name_str.clone());
                        }
                    }
                }
            }
        }

        // Map rooms
        for room in &rooms.data {
            if let (Some(id), Some(metadata)) = (&room.id, &room.metadata) {
                if let Some(name) = &metadata.name {
                    let name_str = name.to_string();
                    name_map.insert(id.to_string(), name_str.clone());

                    for service in &room.services {
                        if let Some(rid) = &service.rid {
                            name_map.insert(rid.to_string(), name_str.clone());
                        }
                    }
                }
            }
        }

        // Map zones
        for zone in &zones.data {
            if let (Some(id), Some(metadata)) = (&zone.id, &zone.metadata) {
                if let Some(name) = &metadata.name {
                    let name_str = name.to_string();
                    name_map.insert(id.to_string(), name_str.clone());

                    for service in &zone.services {
                        if let Some(rid) = &service.rid {
                            name_map.insert(rid.to_string(), name_str.clone());
                        }
                    }
                }
            }
        }

        // Map bridge homes (whole house)
        for home in &bridge_homes.data {
            if let Some(id) = &home.id {
                let name = "Bridge Home".to_string();
                name_map.insert(id.to_string(), name.clone());
                for service in &home.services {
                    if let Some(rid) = &service.rid {
                        name_map.insert(rid.to_string(), name.clone());
                    }
                }
            }
        }

        // Map lights (deprecated metadata but useful fallback)
        for light in &lights.data {
            if let (Some(id), Some(metadata)) = (&light.id, &light.metadata) {
                if let Some(name) = &metadata.name {
                    name_map.insert(id.to_string(), name.to_string());
                }
            }
        }

        Ok(name_map)
    }

    /// Returns a stream of Hue events as JSON strings
    pub async fn event_stream(&self) -> Result<impl futures::Stream<Item = String>, reqwest::Error> {
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
                // Hue sends events as an array of update envelopes
                if let Ok(envelopes) = serde_json::from_str::<Vec<serde_json::Value>>(data) {
                    let mut all_updates = Vec::new();
                    for mut env in envelopes {
                        // Extract the resource updates from the "data" array in each envelope
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
}