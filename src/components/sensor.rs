use crate::hue::client::CompositeSensor;
use dioxus::prelude::*;

#[component]
pub fn Sensor(sensor: ReadSignal<CompositeSensor>) -> Element {
    let mut is_glowing = use_signal(|| false);

    // Trigger glow effect on data change
    use_effect(move || {
        // We track the sensor prop. Read() will subscribe this effect to changes.
        sensor.read();
        
        is_glowing.set(true);
        spawn(async move {
            #[cfg(feature = "server")]
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            #[cfg(not(feature = "server"))]
            gloo_timers::future::sleep(std::time::Duration::from_millis(500)).await;
            is_glowing.set(false);
        });
    });

    let sensor_ref = sensor.read();
    let name_lower = sensor_ref.name.to_lowercase();
    let (border_class, bg_class, icon) = if sensor_ref.is_outdoor {
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

    let glow_class = if is_glowing() {
        "brightness-110 scale-[1.01] shadow-xl ring-2 ring-blue-400/50"
    } else {
        "brightness-100 scale-100 shadow-md ring-0 ring-transparent"
    };

    let motion_class = if let Some(m) = &sensor_ref.motion {
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
            class: "p-4 rounded-lg {border_class} {bg_class} {glow_class} transition-all duration-500 ease-in-out",
            div {
                class: "flex items-center justify-between mb-4",
                h3 {
                    class: "text-lg font-semibold",
                    "{sensor_ref.name}"
                }
                span {
                    class: "text-2xl",
                    "{icon}"
                }
            }
            
            div {
                class: "space-y-1",
                if let Some(m) = &sensor_ref.motion {
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
                
                if let Some(t) = &sensor_ref.temperature {
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

                if let Some(l) = &sensor_ref.light {
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
