use chrono::{DateTime, Duration, Utc};
use dioxus::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub struct HistoryPoint {
    pub value: f64,
    pub time: DateTime<Utc>,
}

#[component]
pub fn Sparkline(
    history: Vec<HistoryPoint>,
    #[props(default = 100)] width: u32,
    #[props(default = 24)] height: u32,
    #[props(default = false)] is_discrete: bool,
    color: String,
    reference_time: DateTime<Utc>,
) -> Element {
    let points = history;
    if points.len() < 2 {
        return rsx! {
            svg { width, height, class: "opacity-20",
                line { x1: "0", y1: height / 2, x2: width, y2: height / 2, stroke: "{color}", stroke_width: "1" }
            }
        };
    }

    let now = reference_time;
    let ten_mins_ago = now - Duration::minutes(10);

    // Normalize X (time)
    let x_scale = |t: DateTime<Utc>| {
        let elapsed = (t - ten_mins_ago).num_seconds() as f64;
        let total = 600.0; // 10 minutes
        (elapsed / total * width as f64).clamp(0.0, width as f64)
    };

    // Normalize Y (value)
    let (min_v, max_v) = if is_discrete {
        (-0.1, 1.1)
    } else {
        let mut min = points[0].value;
        let mut max = points[0].value;
        for p in points.iter() {
            if p.value < min {
                min = p.value;
            }
            if p.value > max {
                max = p.value;
            }
        }
        // Add a bit of padding to the range
        if (max - min).abs() < 0.1 {
            (min - 1.0, max + 1.0)
        } else {
            let padding = (max - min) * 0.1;
            (min - padding, max + padding)
        }
    };

    let y_scale = |v: f64| {
        let range = max_v - min_v;
        height as f64 - ((v - min_v) / range * height as f64).clamp(0.0, height as f64)
    };

    let mut path_data = String::new();
    for (i, p) in points.iter().enumerate() {
        let x = x_scale(p.time);
        let y = y_scale(p.value);
        if i == 0 {
            // Start the path at x=0 with the first point's value
            path_data.push_str(&format!("M 0 {} ", y));
            if x > 0.0 {
                path_data.push_str(&format!("L {} {} ", x, y));
            }
        } else if is_discrete {
            // Step function for discrete values (motion)
            let prev_y = y_scale(points[i - 1].value);
            path_data.push_str(&format!(" L {} {}", x, prev_y));
            path_data.push_str(&format!(" L {} {}", x, y));
        } else {
            path_data.push_str(&format!(" L {} {}", x, y));
        }

        // If this is the last point and it's before the end of the window,
        // extend it to the edge.
        if i == points.len() - 1 && x < width as f64 {
            path_data.push_str(&format!(" L {} {}", width, y));
        }
    }

    rsx! {
        svg {
            width,
            height,
            view_box: "0 0 {width} {height}",
            class: "inline-block align-middle ml-2 overflow-visible",
            path {
                d: "{path_data}",
                fill: "none",
                stroke: "{color}",
                stroke_width: "1.5",
                stroke_linejoin: "round",
                stroke_linecap: "round",
            }
        }
    }
}
