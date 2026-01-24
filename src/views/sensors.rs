use crate::hue::client::SensorViewData;
use crate::components::Sensor;
use dioxus::prelude::*;

#[server]
async fn get_sensors() -> Result<Vec<SensorViewData>, ServerFnError> {
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
            h1 {
                class: "text-2xl font-bold mb-6",
                "Sensors"
            }
            div {
                class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4",
                for sensor in sensors.iter() {
                    Sensor {
                        key: "{sensor.id}",
                        name: sensor.name.clone()
                    }
                }
            }
        }
    }
}
