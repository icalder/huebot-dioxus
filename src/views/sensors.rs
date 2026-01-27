use crate::components::{ActivityIndicator, Clock, Sensor};
use crate::hue::client::CompositeSensor;
use chrono::Utc;
use dioxus::prelude::*;

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

    crate::hue::use_hue_event_handler(
        false,
        move |event| {
            let owner_rid = event.owner_rid();
            let resource_id = event.resource_id();

            let mut updated = false;
            sensors.with_mut(|list: &mut Vec<CompositeSensor>| {
                for s in list.iter_mut() {
                    let is_owner = owner_rid == Some(s.device_id.as_str());
                    let matches_resource = resource_id.is_some()
                        && (s.motion.as_ref().map(|m| m.id.as_str()) == resource_id
                            || s.temperature.as_ref().map(|t| t.id.as_str()) == resource_id
                            || s.light.as_ref().map(|l| l.id.as_str()) == resource_id);

                    if is_owner || matches_resource {
                        s.apply_event(&event);
                        updated = true;
                    }
                }
            });

            if updated {
                last_global_update.set(Utc::now());
            }
        },
        move |msg| {
            println!("Error connecting to event stream: {}", msg);
        },
    );

    rsx! {
        div { class: "container mx-auto p-4",
            div { class: "flex justify-between items-baseline mb-6",
                h1 { class: "text-2xl font-bold", "Sensors" }
                div { class: "flex items-center gap-4",
                    ActivityIndicator { last_update: last_global_update }
                    Clock {}
                }
            }
            div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4",
                for sensor in sensors.read().iter() {
                    Sensor { key: "{sensor.device_id}", sensor: sensor.clone() }
                }
            }
        }
    }
}
