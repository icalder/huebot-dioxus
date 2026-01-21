use progenitor::generate_api;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Deref;

use crate::hue::client::types::ErrorResponse;

// Generate Hue OpenAPI bindings
// NB re-evaluated when the openapi spec file changes
generate_api!("hue-openapi.yaml");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensorViewData {
    pub id: String,
    pub name: String,
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

    /// Fetch motion sensors with their device names resolved
    pub async fn get_sensors(&self) -> Result<Vec<SensorViewData>, Error<ErrorResponse>> {
        // Fetch motion sensors and devices in parallel
        let (motion_res, devices_res) =
            tokio::join!(self.inner.get_motion_sensors(), self.inner.get_devices());

        let motion_response = motion_res?;
        let devices_response = devices_res?;

        // Map device IDs to their names
        let device_names: HashMap<String, String> = devices_response
            .data
            .iter()
            .filter_map(|d| {
                let id = d.id.as_ref()?.to_string();
                let name = d.metadata.as_ref()?.name.as_ref()?.to_string();
                Some((id, name))
            })
            .collect();

        let mut sensors = Vec::new();
        for m in &motion_response.data {
            if let Some(id) = &m.id {
                // Find the owner device's name
                let name = m
                    .owner
                    .as_ref()
                    .and_then(|owner| {
                        let rid = owner.rid.as_ref()?.to_string();
                        device_names.get(&rid)
                    })
                    .cloned()
                    .unwrap_or_else(|| format!("Motion Sensor {}", id.to_string()));

                sensors.push(SensorViewData {
                    id: id.to_string(),
                    name,
                });
            }
        }

        Ok(sensors)
    }
}
