use crate::hue::client::CompositeSensor;
use crate::components::{Sensor, Clock, ActivityIndicator};
use crate::hue::hue_events;
use dioxus::prelude::*;
use chrono::Utc;

#[server]
async fn get_sensors() -> Result<Vec<CompositeSensor>, ServerFnError> {
    crate::hue::get_sensors_cached().await
}

/// The Sensors page component that will be rendered when the current route is `[Route::Sensors]`
#[component]
pub fn Sensors() -> Element {
    let initial_sensors = use_loader(get_sensors)?;
    let mut sensors = use_signal(move || initial_sensors.read().clone());
    let mut last_global_update = use_signal(Utc::now);

    use_resource(move || async move {
        match hue_events().await {
            Ok(mut stream) => {
                use futures::StreamExt;
                while let Some(Ok(event_str_raw)) = stream.next().await {
                    let event_str: String = event_str_raw;
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&event_str) {
                        let owner_rid = v.get("owner").and_then(|o| o.get("rid")).and_then(|rid| rid.as_str());
                        let resource_id = v.get("id").and_then(|id| id.as_str());

                        let mut updated = false;
                        sensors.with_mut(|list: &mut Vec<CompositeSensor>| {
                            for s in list.iter_mut() {
                                // Update if the event belongs to this device (via owner) 
                                // or matches a known resource ID already attached to this sensor
                                let is_owner = owner_rid == Some(&s.device_id);
                                let matches_resource = resource_id.is_some() && (
                                    s.motion.as_ref().map(|m| m.id.as_str()) == resource_id ||
                                    s.temperature.as_ref().map(|t| t.id.as_str()) == resource_id ||
                                    s.light.as_ref().map(|l| l.id.as_str()) == resource_id
                                );

                                if is_owner || matches_resource {
                                    s.update_from_json(&v);
                                    updated = true;
                                }
                            }
                        });

                        if updated {
                            last_global_update.set(Utc::now());
                        }
                    }
                }
            }
            Err(e) => {
                println!("Error connecting to event stream: {}", e);
            }
        }
    });

    rsx! {
        div {
            class: "container mx-auto p-4",
            div {
                class: "flex justify-between items-baseline mb-6",
                h1 {
                    class: "text-2xl font-bold",
                    "Sensors"
                }
                div {
                    class: "flex items-center gap-4",
                    ActivityIndicator { last_update: last_global_update }
                    Clock {}
                }
            }
            div {
                class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4",
                for sensor in sensors.read().iter() {
                    Sensor {
                        key: "{sensor.device_id}",
                        sensor: sensor.clone()
                    }
                }
            }
        }
    }
}
