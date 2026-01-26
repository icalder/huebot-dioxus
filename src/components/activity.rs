use chrono::{DateTime, Utc};
use dioxus::prelude::*;

#[component]
pub fn ActivityIndicator(last_update: ReadSignal<DateTime<Utc>>) -> Element {
    let mut now = use_signal(Utc::now);

    // Update 'now' every second to keep the indicator moving
    use_future(move || async move {
        loop {
            #[cfg(feature = "server")]
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            #[cfg(not(feature = "server"))]
            gloo_timers::future::sleep(std::time::Duration::from_secs(1)).await;
            now.set(Utc::now());
        }
    });

    let elapsed = (now() - last_update()).num_seconds();
    let max_freshness = 300; // 5 minutes
    let freshness = (1.0 - (elapsed as f64 / max_freshness as f64)).clamp(0.0, 1.0);

    // Calculate color based on freshness
    let color = if freshness > 0.5 {
        "text-green-500"
    } else if freshness > 0.2 {
        "text-yellow-500"
    } else if freshness > 0.0 {
        "text-orange-500"
    } else {
        "text-red-500 animate-pulse"
    };

    // SVG Circular progress math
    let radius = 10.0;
    let circumference = 2.0 * std::f64::consts::PI * radius;
    let offset = circumference * (1.0 - freshness);

    rsx! {
        div {
            class: "flex items-center gap-2",
            svg {
                width: "24",
                height: "24",
                view_box: "0 0 32 32",
                class: "transform -rotate-90",
                // Background track
                circle {
                    cx: "16",
                    cy: "16",
                    r: "{radius}",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "3",
                    class: "text-gray-200 dark:text-gray-800"
                }
                // Freshness ring
                circle {
                    cx: "16",
                    cy: "16",
                    r: "{radius}",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "3",
                    stroke_dasharray: "{circumference}",
                    stroke_dashoffset: "{offset}",
                    stroke_linecap: "round",
                    class: "{color} transition-all duration-1000"
                }
            }
        }
    }
}
