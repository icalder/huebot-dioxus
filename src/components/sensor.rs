use crate::hue::client::CompositeSensor;
use dioxus::prelude::*;

#[component]
pub fn Sensor(sensor: CompositeSensor) -> Element {
    let name_lower = sensor.name.to_lowercase();
    let (border_class, bg_class, icon) = if sensor.is_outdoor {
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

    let motion_class = if let Some(m) = &sensor.motion {
        if m.presence {
            "text-red-600 dark:text-red-400 font-bold"
        } else {
            "text-green-600 dark:text-green-400"
        }
    } else {
        "text-gray-400"
    };

    rsx! {
        div {
            class: "p-4 rounded-lg shadow-md {border_class} {bg_class} transition-colors duration-200",
            div {
                class: "flex items-center justify-between mb-4",
                h3 {
                    class: "text-lg font-semibold",
                    "{sensor.name}"
                }
                span {
                    class: "text-2xl",
                    "{icon}"
                }
            }
            
            div {
                class: "space-y-1",
                if let Some(m) = &sensor.motion {
                    {
                        let time = m.last_updated.with_timezone(&chrono::Local).format("%H:%M:%S");
                        let status = if m.presence { "Detected" } else { "Clear" };
                        rsx! {
                            div {
                                class: "text-lg grid grid-cols-[4.5rem_1fr] items-baseline",
                                span { class: "text-gray-500", "Motion:" }
                                div {
                                    span { class: "{motion_class}", "{status}" }
                                    span { class: "text-xs text-gray-400 ml-2", "@{time}" }
                                }
                            }
                        }
                    }
                }
                
                if let Some(t) = &sensor.temperature {
                    {
                        let time = t.last_updated.with_timezone(&chrono::Local).format("%H:%M:%S");
                        rsx! {
                            div {
                                class: "text-lg grid grid-cols-[4.5rem_1fr] items-baseline",
                                span { class: "text-gray-500", "Temp:" }
                                div {
                                    span { class: "font-semibold", "{t.temperature:.1}°C" }
                                    span { class: "text-xs text-gray-400 ml-2", "@{time}" }
                                }
                            }
                        }
                    }
                }

                if let Some(l) = &sensor.light {
                    {
                        let time = l.last_updated.with_timezone(&chrono::Local).format("%H:%M:%S");
                        rsx! {
                            div {
                                class: "text-lg grid grid-cols-[4.5rem_1fr] items-baseline",
                                span { class: "text-gray-500", "Light:" }
                                div {
                                    span { class: "font-semibold", "{l.light_level} lx" }
                                    span { class: "text-xs text-gray-400 ml-2", "@{time}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
