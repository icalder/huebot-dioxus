use crate::hue::client::CompositeSensor;
use crate::components::{Sparkline, HistoryPoint};
use dioxus::prelude::*;
use std::cmp::Ordering;
use chrono::{Utc, Duration};

#[component]
pub fn Sensor(sensor: ReadSignal<CompositeSensor>) -> Element {
    let mut is_glowing = use_signal(|| false);
    
    // Store previous values to calculate trends
    let mut last_temp = use_signal(|| None::<f64>);
    let mut last_light = use_signal(|| None::<i32>);
    let mut temp_trend = use_signal(|| Ordering::Equal);
    let mut light_trend = use_signal(|| Ordering::Equal);

    // History for sparklines
    let mut temp_history = use_signal(Vec::<HistoryPoint>::new);
    let mut light_history = use_signal(Vec::<HistoryPoint>::new);
    let mut motion_history = use_signal(Vec::<HistoryPoint>::new);

    // Helper to update history and prune old data
    let update_history = move |history: &mut Signal<Vec<HistoryPoint>>, val: f64| {
        let now = Utc::now();
        let limit = now - Duration::minutes(10);
        history.with_mut(|h| {
            h.push(HistoryPoint { value: val, time: now });
            h.retain(|p| p.time >= limit);
        });
    };

    // Trigger effects on data change
    use_effect(move || {
        let s = sensor.read();
        
        // Temperature trend and history
        if let Some(t) = &s.temperature {
            if let Some(prev) = *last_temp.peek() {
                if (t.temperature - prev).abs() > 0.01 {
                    temp_trend.set(t.temperature.partial_cmp(&prev).unwrap_or(Ordering::Equal));
                }
            }
            last_temp.set(Some(t.temperature));
            update_history(&mut temp_history, t.temperature);
        }

        // Light trend and history
        if let Some(l) = &s.light {
            if let Some(prev) = *last_light.peek() {
                if l.light_level != prev {
                    light_trend.set(l.light_level.cmp(&prev));
                }
            }
            last_light.set(Some(l.light_level));
            update_history(&mut light_history, l.light_level as f64);
        }

        // Motion history
        if let Some(m) = &s.motion {
            update_history(&mut motion_history, if m.presence { 1.0 } else { 0.0 });
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

    // Periodic refresh to keep sparklines scrolling even without events
    use_future(move || async move {
        loop {
            #[cfg(feature = "server")]
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            #[cfg(not(feature = "server"))]
            gloo_timers::future::sleep(std::time::Duration::from_secs(30)).await;
            
            // Just touching the histories to trigger re-render of sparklines
            temp_history.read();
            light_history.read();
            motion_history.read();
        }
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

    let transition_class = if is_glowing() {
        "transition-all duration-100 ease-out" // Fast "pop" in
    } else {
        "transition-all duration-700 ease-in-out" // Smooth fade out
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
            class: "p-4 rounded-lg {border_class} {bg_class} {glow_class} {transition_class}",
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
                                    class: "flex items-center justify-between",
                                    div {
                                        span { class: "{motion_class}", "{status}" }
                                        span { class: "text-xs text-gray-400 ml-2", "@{time}" }
                                    }
                                    Sparkline { history: motion_history, is_discrete: true, color: "#f87171" }
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
                                    class: "flex items-center justify-between",
                                    div {
                                        span { 
                                            class: "font-semibold", 
                                            "{t.temperature:.1}°C"
                                            match temp_trend() {
                                                Ordering::Greater => rsx! { span { class: "text-red-500 ml-1 text-sm animate-pulse", "↑" } },
                                                Ordering::Less => rsx! { span { class: "text-blue-500 ml-1 text-sm animate-pulse", "↓" } },
                                                Ordering::Equal => rsx! { "" }
                                            }
                                        }
                                        span { class: "text-xs text-gray-400 ml-2", "@{time}" }
                                    }
                                    Sparkline { history: temp_history, color: "#60a5fa" }
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
                                    class: "flex items-center justify-between",
                                    div {
                                        span { 
                                            class: "font-semibold", 
                                            "{l.light_level} lx"
                                            match light_trend() {
                                                Ordering::Greater => rsx! { span { class: "text-yellow-500 ml-1 text-sm animate-pulse", "↑" } },
                                                Ordering::Less => rsx! { span { class: "text-gray-400 ml-1 text-sm animate-pulse", "↓" } },
                                                Ordering::Equal => rsx! { "" }
                                            }
                                        }
                                        span { class: "text-xs text-gray-400 ml-2", "@{time}" }
                                    }
                                    Sparkline { history: light_history, color: "#fbbf24" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
