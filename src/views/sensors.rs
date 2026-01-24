use crate::hue::client::CompositeSensor;
use crate::components::{Sensor, Clock};
use dioxus::prelude::*;

#[server]
async fn get_sensors() -> Result<Vec<CompositeSensor>, ServerFnError> {
    let client = crate::hue::get_hue_client();

    let sensors = client
        .get_sensors()
        .await
        .map_err(|e| ServerFnError::new(e))?;

    Ok(sensors)
}

/// The Sensors page component that will be rendered when the current route is `[Route::Sensors]`
#[component]
pub fn Sensors() -> Element {
    let sensors = use_loader(get_sensors)?;

    rsx! {
        div {
            class: "container mx-auto p-4",
            div {
                class: "flex justify-between items-baseline mb-6",
                h1 {
                    class: "text-2xl font-bold",
                    "Sensors"
                }
                Clock {}
            }
            div {
                class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4",
                for sensor in sensors.iter() {
                    Sensor {
                        key: "{sensor.device_id}",
                        sensor: sensor.clone()
                    }
                }
            }
        }
    }
}
