use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensorViewData {
    pub id: String,
    pub name: String,
}

#[server]
async fn get_sensors() -> Result<Vec<SensorViewData>, ServerFnError> {
    let client = crate::hue::get_hue_client();

    // Fetch motion sensors and devices in parallel
    let (motion_res, devices_res) = tokio::join!(client.get_motion_sensors(), client.get_devices());

    let motion_response = motion_res.map_err(|e| ServerFnError::new(e.to_string()))?;
    let devices_response = devices_res.map_err(|e| ServerFnError::new(e.to_string()))?;

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

/// The Sensors page component that will be rendered when the current route is `[Route::Sensors]`
#[component]
pub fn Sensors() -> Element {
    let sensors_resource = use_resource(get_sensors);

    rsx! {
        div {
            h1 { "Sensors" }
            match sensors_resource.value()() {
                Some(Ok(list)) => rsx! {
                    ul {
                        for sensor in list {
                            li { "{sensor.name}" }
                        }
                    }
                },
                Some(Err(e)) => rsx! { "Error loading sensors: {e}" },
                None => rsx! { "Loading..." },
            }
        }
    }
}