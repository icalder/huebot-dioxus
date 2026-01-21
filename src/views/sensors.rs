use crate::hue::client::SensorViewData;
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
