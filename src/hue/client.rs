use chrono::{DateTime, Utc};
use progenitor::{generate_api, Error as ProgenitorError};
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
    pub id_v1: Option<String>,
    pub enabled: bool,
    pub presence: bool,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemperatureData {
    pub id: String,
    pub id_v1: Option<String>,
    pub enabled: bool,
    pub temperature: f64,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LightData {
    pub id: String,
    pub id_v1: Option<String>,
    pub enabled: bool,
    pub light_level: i32,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeSensor {
    pub device_id: String,
    pub name: String,
    pub is_outdoor: bool,
    pub enabled: bool,
    pub motion: Option<MotionData>,
    pub temperature: Option<TemperatureData>,
    pub light: Option<LightData>,
}

impl CompositeSensor {
    pub fn update_from_json(&mut self, v: &serde_json::Value) {
        let event_type = v.get("type").and_then(|t| t.as_str());
        let id = v.get("id").and_then(|id| id.as_str()).unwrap_or_default().to_string();
        let id_v1 = v.get("id_v1").and_then(|id| id.as_str()).map(|s| s.to_string());
        let enabled = v.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true);

        match event_type {
            Some("motion") => {
                if let Some(report) = v.get("motion").and_then(|m| m.get("motion_report")) {
                    self.motion = Some(MotionData {
                        id,
                        id_v1,
                        enabled,
                        presence: report.get("motion").and_then(|m| m.as_bool()).unwrap_or(false),
                        last_updated: ClientEx::parse_date(&report.get("changed").and_then(|c| c.as_str()).map(|s| s.to_string())),
                    });
                } else if v.get("enabled").is_some() {
                    if let Some(m) = &mut self.motion {
                        m.enabled = enabled;
                    }
                }
            }
            Some("temperature") => {
                if let Some(report) = v.get("temperature").and_then(|t| t.get("temperature_report")) {
                    self.temperature = Some(TemperatureData {
                        id,
                        id_v1,
                        enabled,
                        temperature: report.get("temperature").and_then(|t| t.as_f64()).unwrap_or(0.0),
                        last_updated: ClientEx::parse_date(&report.get("changed").and_then(|c| c.as_str()).map(|s| s.to_string())),
                    });
                } else if v.get("enabled").is_some() {
                    if let Some(t) = &mut self.temperature {
                        t.enabled = enabled;
                    }
                }
            }
            Some("light_level") => {
                if let Some(report) = v.get("light").and_then(|l| l.get("light_level_report")) {
                    self.light = Some(LightData {
                        id,
                        id_v1,
                        enabled,
                        light_level: report.get("light_level").and_then(|l| l.as_i64()).map(|v| v as i32).unwrap_or(0),
                        last_updated: ClientEx::parse_date(&report.get("changed").and_then(|c| c.as_str()).map(|s| s.to_string())),
                    });
                } else if v.get("enabled").is_some() {
                    if let Some(l) = &mut self.light {
                        l.enabled = enabled;
                    }
                }
            }
            _ => {}
        }

        // Update overall enabled state: if any sensor service is enabled, the composite is considered enabled.
        // Usually they are all enabled/disabled together if it's the main "sensor" switch.
        self.enabled = self.motion.as_ref().map(|m| m.enabled).unwrap_or(true) 
            && self.temperature.as_ref().map(|t| t.enabled).unwrap_or(true)
            && self.light.as_ref().map(|l| l.enabled).unwrap_or(true);
    }
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

impl ClientEx {
    pub fn parse_date(s: &Option<String>) -> DateTime<Utc> {
        s.as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now)
    }

    fn get_owner_rid(
        owner: &Option<crate::hue::client::types::ResourceIdentifier>,
    ) -> Option<String> {
        owner.as_ref().and_then(|o| o.rid.as_ref()).map(|s| s.to_string())
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

        let mut device_map = self.init_device_map(&devices_response.data);

        // Populate motion data
        for m in &motion_response.data {
            if let (Some(id), Some(owner_rid)) = (&m.id, Self::get_owner_rid(&m.owner)) {
                if let Some(cs) = device_map.get_mut(&owner_rid) {
                    if let Some(report) = m.motion.as_ref().and_then(|m| m.motion_report.as_ref()) {
                        cs.motion = Some(MotionData {
                            id: id.to_string(),
                            id_v1: m.id_v1.as_ref().map(|v| v.to_string()),
                            enabled: m.enabled.unwrap_or(true),
                            presence: report.motion.unwrap_or(false),
                            last_updated: Self::parse_date(&report.changed),
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
                        cs.temperature = Some(TemperatureData {
                            id: id.to_string(),
                            id_v1: t.id_v1.as_ref().map(|v| v.to_string()),
                            enabled: t.enabled.unwrap_or(true),
                            temperature: report.temperature.unwrap_or(0.0),
                            last_updated: report.changed.unwrap_or_else(Utc::now),
                        });
                    }
                }
            }
        }

        // Populate light data
        for l in &light_response.data {
            if let (Some(id), Some(owner_rid)) = (&l.id, Self::get_owner_rid(&l.owner)) {
                if let Some(cs) = device_map.get_mut(&owner_rid) {
                    if let Some(report) = l.light.as_ref().and_then(|l| l.light_level_report.as_ref())
                    {
                        cs.light = Some(LightData {
                            id: id.to_string(),
                            id_v1: l.id_v1.as_ref().map(|v| v.to_string()),
                            enabled: l.enabled.unwrap_or(true),
                            light_level: report.light_level.map(|v| v as i32).unwrap_or(0),
                            last_updated: report.changed.unwrap_or_else(Utc::now),
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
        let (devices_res, rooms_res, zones_res, lights_res, bridge_homes_res) = tokio::join!(
            self.inner.get_devices(),
            self.inner.get_rooms(),
            self.inner.get_zones(),
            self.inner.get_lights(),
            self.inner.get_bridge_homes(),
        );

        let mut name_map = HashMap::new();

        for device in &devices_res?.data {
            Self::insert_resource_names(
                &mut name_map,
                device.id.as_ref().map(|id| id.to_string()),
                device.metadata.as_ref().and_then(|m| m.name.as_ref()).map(|n| n.to_string()),
                &device.services,
            );
        }

        for room in &rooms_res?.data {
            Self::insert_resource_names(
                &mut name_map,
                room.id.as_ref().map(|id| id.to_string()),
                room.metadata.as_ref().and_then(|m| m.name.as_ref()).map(|n| n.to_string()),
                &room.services,
            );
        }

        for zone in &zones_res?.data {
            Self::insert_resource_names(
                &mut name_map,
                zone.id.as_ref().map(|id| id.to_string()),
                zone.metadata.as_ref().and_then(|m| m.name.as_ref()).map(|n| n.to_string()),
                &zone.services,
            );
        }

        for home in &bridge_homes_res?.data {
            Self::insert_resource_names(
                &mut name_map,
                home.id.as_ref().map(|id| id.to_string()),
                Some("Bridge Home".to_string()),
                &home.services,
            );
        }

        for light in &lights_res?.data {
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
