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
}

impl Deref for ClientEx {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(feature = "server")]
impl ClientEx {
    pub fn new(client: Client) -> Self {
        Self { inner: client }
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
}
