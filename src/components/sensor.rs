use crate::components::{HistoryPoint, Sparkline};
use crate::hue::client::CompositeSensor;
use crate::Route;
use chrono::Utc;
use dioxus::prelude::*;
use std::cmp::Ordering;

#[component]
pub fn Sensor(sensor: CompositeSensor) -> Element {
    let mut is_glowing = use_signal(|| false);
    
    // Calculate the latest update time from the sensor data for initial render stability
    let initial_time = {
        let mut t = Utc::now() - chrono::Duration::days(1); // Default to old if no data
        if let Some(m) = &sensor.motion { if m.last_updated > t { t = m.last_updated; } }
        if let Some(temp) = &sensor.temperature { if temp.last_updated > t { t = temp.last_updated; } }
        if let Some(l) = &sensor.light { if l.last_updated > t { t = l.last_updated; } }
        t
    };
    
    let mut graph_time = use_signal(|| initial_time);

    // Store previous values to calculate trends
    let mut last_temp = use_signal(|| None::<f64>);
    let mut last_light = use_signal(|| None::<i32>);
    let mut temp_trend = use_signal(|| Ordering::Equal);
    let mut light_trend = use_signal(|| Ordering::Equal);

    // Trigger effects on data change for trends and glowing only
    // Note: this effect now depends on the 'sensor' prop value
    let s_effect = sensor.clone();
    use_effect(move || {
        let s = s_effect.clone();

        // Temperature trend
        if let Some(t) = &s.temperature {
            if let Some(prev) = *last_temp.peek() {
                if (t.temperature - prev).abs() > 0.01 {
                    temp_trend.set(t.temperature.partial_cmp(&prev).unwrap_or(Ordering::Equal));
                }
            }
            last_temp.set(Some(t.temperature));
        }

        // Light trend
        if let Some(l) = &s.light {
            if let Some(prev) = *last_light.peek() {
                if l.light_level != prev {
                    light_trend.set(l.light_level.cmp(&prev));
                }
            }
            last_light.set(Some(l.light_level));
        }

        is_glowing.set(true);
        spawn(async move {
            #[cfg(feature = "server")]
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            #[cfg(not(feature = "server"))]
            gloo_timers::future::sleep(std::time::Duration::from_millis(500)).await;
            is_glowing.set(false);
        });
    });

    // Periodic refresh to keep sparklines scrolling
    use_future(move || async move {
        loop {
            // Update the graph time to "now" to create the live scrolling effect
            // We do this *before* sleep on the first run (implicitly) or quickly after hydration
            // Actually, for hydration matching, we want the first render to use initial_time.
            // So we sleep first.
            
            #[cfg(feature = "server")]
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            #[cfg(not(feature = "server"))]
            gloo_timers::future::sleep(std::time::Duration::from_secs(1)).await;

            graph_time.set(Utc::now());
        }
    });

    let sensor_ref = sensor;
    let current_time = graph_time();

    // Memos replaced with direct calculation
    let motion_history = {
        let h = sensor_ref
            .motion
            .as_ref()
            .map(|m| &m.history)
            .cloned()
            .unwrap_or_default();
        let mut points: Vec<HistoryPoint> = h
            .into_iter()
            .map(|(t, v)| HistoryPoint {
                time: t,
                value: if v { 1.0 } else { 0.0 },
            })
            .collect();
        if let Some(last) = points.last() {
            points.push(HistoryPoint {
                time: current_time,
                value: last.value,
            });
        }
        points
    };
    let temp_history = {
        let h = sensor_ref
            .temperature
            .as_ref()
            .map(|t| &t.history)
            .cloned()
            .unwrap_or_default();
        let mut points: Vec<HistoryPoint> = h
            .into_iter()
            .map(|(t, v)| HistoryPoint { time: t, value: v })
            .collect();
        if let Some(last) = points.last() {
            points.push(HistoryPoint {
                time: current_time,
                value: last.value,
            });
        }
        points
    };
    let light_history = {
        let h = sensor_ref
            .light
            .as_ref()
            .map(|l| &l.history)
            .cloned()
            .unwrap_or_default();
        let mut points: Vec<HistoryPoint> = h
            .into_iter()
            .map(|(t, v)| HistoryPoint {
                time: t,
                value: v as f64,
            })
            .collect();
        if let Some(last) = points.last() {
            points.push(HistoryPoint {
                time: current_time,
                value: last.value,
            });
        }
        points
    };

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

    let disabled_class = if !sensor_ref.enabled {
        "grayscale opacity-60 contrast-75"
    } else {
        ""
    };

    let glow_class = if is_glowing() && sensor_ref.enabled {
        "brightness-125 scale-105 shadow-2xl ring-4 ring-blue-500 z-10"
    } else {
        "brightness-100 scale-100 shadow-md ring-0 ring-transparent z-0"
    };

    let transition_class = if is_glowing() {
        "transition-all duration-100 ease-out" // Fast "pop" in
    } else {
        "transition-all duration-700 ease-in-out" // Smooth fade out
    };

    let motion_class = if let Some(m) = &sensor_ref.motion {
        if !m.enabled {
            "text-gray-500 dark:text-gray-500 italic"
        } else if m.presence {
            "text-red-600 dark:text-red-400 font-bold"
        } else {
            "text-green-600 dark:text-green-400"
        }
    } else {
        "text-gray-400"
    };

    rsx! {
        Link {
            to: Route::Graphs { sensor_id: sensor_ref.device_id.clone() },
            class: "contents",
            div {
                class: "p-4 rounded-lg {border_class} {bg_class} {glow_class} {transition_class} {disabled_class} relative overflow-hidden",
                if !sensor_ref.enabled {
                    div {
                        class: "absolute top-0 right-0 bg-gray-500 text-white text-[10px] font-bold px-2 py-0.5 rounded-bl-md z-20 uppercase tracking-tighter",
                        "Disabled"
                    }
                }
                div {
                    class: "flex items-center justify-between mb-6",
                    div {
                        class: "bg-gray-50 dark:bg-black/40 px-3 py-1.5 rounded border border-gray-300/50 dark:border-gray-800 shadow-inner flex-grow mr-4 overflow-hidden",
                        h3 {
                            class: "text-base font-bold tracking-wide text-gray-600 dark:text-gray-300 truncate",
                            "{sensor_ref.name}"
                        }
                    }
                    span {
                        class: "text-2xl drop-shadow-sm",
                        "{icon}"
                    }
                }

                div {
                    class: "space-y-1",
                    if let Some(m) = &sensor_ref.motion {
                        {
                            let time = m.last_updated.with_timezone(&chrono::Local).format("%H:%M:%S");
                            let status = if !m.enabled { "Disabled" } else if m.presence { "Detected" } else { "Clear" };
                            rsx! {
                                div {
                                    class: "text-lg grid grid-cols-[4.5rem_1fr] items-baseline",
                                    span { class: "text-gray-600 dark:text-gray-400", "Motion:" }
                                    div {
                                        class: "flex items-center justify-between",
                                        div {
                                            span { class: "{motion_class}", "{status}" }
                                            span { class: "text-xs text-gray-500 dark:text-gray-500 ml-2", "@{time}" }
                                        }
                                        Sparkline { history: motion_history, is_discrete: true, color: "#f87171", reference_time: current_time }
                                    }
                                }
                            }
                        }
                    }

                    if let Some(t) = &sensor_ref.temperature {
                        {
                            let time = t.last_updated.with_timezone(&chrono::Local).format("%H:%M:%S");
                            let value_text = if !t.enabled { "Disabled".to_string() } else { format!("{:.1}°C", t.temperature) };
                            let value_class = if !t.enabled { "text-gray-500 dark:text-gray-500 italic" } else { "font-semibold" };
                            rsx! {
                                div {
                                    class: "text-lg grid grid-cols-[4.5rem_1fr] items-baseline",
                                    span { class: "text-gray-600 dark:text-gray-400", "Temp:" }
                                    div {
                                        class: "flex items-center justify-between",
                                        div {
                                            span {
                                                class: "{value_class}",
                                                "{value_text}"
                                                if t.enabled {
                                                    match temp_trend() {
                                                        Ordering::Greater => rsx! { span { class: "text-red-500 ml-1 text-sm animate-pulse", "↑" } },
                                                        Ordering::Less => rsx! { span { class: "text-blue-500 ml-1 text-sm animate-pulse", "↓" } },
                                                        Ordering::Equal => rsx! { "" }
                                                    }
                                                }
                                            }
                                            span { class: "text-xs text-gray-500 dark:text-gray-500 ml-2", "@{time}" }
                                        }
                                        Sparkline { history: temp_history, color: "#60a5fa", reference_time: current_time }
                                    }
                                }
                            }
                        }
                    }

                    if let Some(l) = &sensor_ref.light {
                        {
                            let time = l.last_updated.with_timezone(&chrono::Local).format("%H:%M:%S");
                            let value_text = if !l.enabled { "Disabled".to_string() } else { format!("{} lx", l.light_level) };
                            let value_class = if !l.enabled { "text-gray-500 dark:text-gray-500 italic" } else { "font-semibold" };
                            rsx! {
                                div {
                                    class: "text-lg grid grid-cols-[4.5rem_1fr] items-baseline",
                                    span { class: "text-gray-600 dark:text-gray-400", "Light:" }
                                    div {
                                        class: "flex items-center justify-between",
                                        div {
                                            span {
                                                class: "{value_class}",
                                                "{value_text}"
                                                if l.enabled {
                                                    match light_trend() {
                                                        Ordering::Greater => rsx! { span { class: "text-yellow-500 ml-1 text-sm animate-pulse", "↑" } },
                                                        Ordering::Less => rsx! { span { class: "text-gray-400 ml-1 text-sm animate-pulse", "↓" } },
                                                        Ordering::Equal => rsx! { "" }
                                                    }
                                                }
                                            }
                                            span { class: "text-xs text-gray-500 dark:text-gray-500 ml-2", "@{time}" }
                                        }
                                        Sparkline { history: light_history, color: "#fbbf24", reference_time: current_time }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
