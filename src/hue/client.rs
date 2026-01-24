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
    pub is_outdoor: bool,
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

        // Map device IDs to their names and outdoor status
        let device_info: HashMap<String, (String, bool)> = devices_response
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
                Some((id, (name, is_outdoor)))
            })
            .collect();

        let mut sensors = Vec::new();
        for m in &motion_response.data {
            if let Some(id) = &m.id {
                // Find the owner device's name and outdoor status
                let (name, is_outdoor) = m
                    .owner
                    .as_ref()
                    .and_then(|owner| {
                        let rid = owner.rid.as_ref()?.to_string();
                        device_info.get(&rid)
                    })
                    .cloned()
                    .unwrap_or_else(|| (format!("Motion Sensor {}", id.to_string()), false));

                sensors.push(SensorViewData {
                    id: id.to_string(),
                    name,
                    is_outdoor,
                });
            }
        }

        sensors.sort_by(|a, b| {
            // Sort by outdoor status (Outdoor first), then by name
            b.is_outdoor
                .cmp(&a.is_outdoor)
                .then_with(|| a.name.cmp(&b.name))
        });

        Ok(sensors)
    }
}
