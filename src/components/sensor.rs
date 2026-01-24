use dioxus::prelude::*;

#[component]
pub fn Sensor(name: String) -> Element {
    rsx! {
        div {
            class: "p-4 border rounded-lg shadow-md bg-white dark:bg-gray-800",
            h3 {
                class: "text-lg font-semibold",
                "{name}"
            }
            // Placeholders for future sensors
            div {
                class: "text-sm text-gray-500",
                "Sensors data (motion, temp, light) coming soon..."
            }
        }
    }
}
