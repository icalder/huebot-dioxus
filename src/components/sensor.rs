use dioxus::prelude::*;

#[component]
pub fn Sensor(name: String, is_outdoor: bool) -> Element {
    let name_lower = name.to_lowercase();
    let (border_class, bg_class, icon) = if is_outdoor {
        (
            "border border-gray-300 dark:border-gray-500",
            "bg-blue-50 dark:bg-blue-900/10",
            "🌲",
        )
    } else {
        let icon = if name_lower.contains("garage") {
            "🚗"
        } else if name_lower.contains("shed") {
            "🛠️"
        } else {
            "🏠"
        };
        (
            "border-4 border-gray-300 dark:border-gray-600",
            "bg-white dark:bg-gray-800",
            icon,
        )
    };

    rsx! {
        div {
            class: "p-4 rounded-lg shadow-md {border_class} {bg_class} transition-colors duration-200",
            div {
                class: "flex items-center justify-between mb-2",
                h3 {
                    class: "text-lg font-semibold",
                    "{name}"
                }
                span {
                    class: "text-2xl",
                    "{icon}"
                }
            }
            // Placeholders for future sensors
            div {
                class: "text-sm text-gray-500",
                "Sensors data (motion, temp, light) coming soon..."
            }
        }
    }
}
