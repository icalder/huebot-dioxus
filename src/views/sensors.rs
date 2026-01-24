use crate::hue::client::CompositeSensor;
use crate::components::Sensor;
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
    let mut now = use_signal(|| chrono::Local::now().format("%H:%M:%S").to_string());

    use_future(move || async move {
        loop {
            #[cfg(feature = "server")]
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            #[cfg(not(feature = "server"))]
            gloo_timers::future::sleep(std::time::Duration::from_secs(1)).await;

            now.set(chrono::Local::now().format("%H:%M:%S").to_string());
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
                span {
                    class: "text-lg text-gray-500 font-mono",
                    "Local Time: {now}"
                }
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
