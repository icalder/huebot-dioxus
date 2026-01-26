use crate::components::HistoryPoint;
use chrono::{DateTime, Duration, Utc};
use dioxus::prelude::*;

#[component]
pub fn SensorDataGraph(
    history: Vec<HistoryPoint>,
    #[props(default = 1000)] width: u32,
    #[props(default = 300)] height: u32,
    #[props(default = false)] is_discrete: bool,
    #[props(default = String::new())] unit: String,
    color: String,
) -> Element {
    let mut hovered_point = use_signal(|| None::<HistoryPoint>);

    if history.is_empty() {
        return rsx! {
            div { class: "h-full w-full flex items-center justify-center text-gray-400 italic", "No data available" }
        };
    }

    let end_time = Utc::now();
    let start_time = end_time - Duration::hours(24);

    // Normalize X (time) over 24 hours
    let x_scale = move |t: DateTime<Utc>| {
        let elapsed = (t - start_time).num_seconds() as f64;
        let total = 24.0 * 3600.0;
        (elapsed / total * width as f64).clamp(0.0, width as f64)
    };

    // Normalize Y (value)
    let (min_v, max_v) = if is_discrete {
        (-0.1, 1.1)
    } else {
        let mut min = history[0].value;
        let mut max = history[0].value;
        for p in history.iter() {
            if p.value < min {
                min = p.value;
            }
            if p.value > max {
                max = p.value;
            }
        }
        if (max - min).abs() < 0.1 {
            (min - 1.0, max + 1.0)
        } else {
            let padding = (max - min) * 0.1;
            (min - padding, max + padding)
        }
    };

    let y_scale = move |v: f64| {
        let range = max_v - min_v;
        height as f64 - ((v - min_v) / range * height as f64).clamp(0.0, height as f64)
    };

    let mut sorted_history = history.clone();
    sorted_history.sort_by_key(|p| p.time);

    // Apply smoothing for non-discrete data
    let display_history = if !is_discrete && sorted_history.len() > 3 {
        let mut smoothed = Vec::new();
        let window_size = 5;
        for i in 0..sorted_history.len() {
            let start = i.saturating_sub(window_size / 2);
            let end = (i + window_size / 2).min(sorted_history.len() - 1);
            let count = (end - start + 1) as f64;
            let sum: f64 = sorted_history[start..=end].iter().map(|p| p.value).sum();
            smoothed.push(HistoryPoint {
                value: sum / count,
                time: sorted_history[i].time,
            });
        }
        smoothed
    } else {
        sorted_history
    };

    let mut path_data = String::new();
    let mut graph_points = Vec::new();
    for (i, p) in display_history.iter().enumerate() {
        let x = x_scale(p.time);
        let y = y_scale(p.value);
        graph_points.push((x, y, p.value));

        if i == 0 {
            path_data.push_str(&format!("M {} {}", x, y));
        } else if is_discrete {
            let prev_y = y_scale(display_history[i - 1].value);
            path_data.push_str(&format!(" L {} {}", x, prev_y));
            path_data.push_str(&format!(" L {} {}", x, y));
        } else {
            path_data.push_str(&format!(" L {} {}", x, y));
        }
    }

    // Pre-calculate hit regions for hover effect
    let hit_regions = if !display_history.is_empty() {
        let mut regions = Vec::with_capacity(display_history.len());
        let last_idx = display_history.len() - 1;
        for (i, p) in display_history.iter().enumerate() {
            let current_x = x_scale(p.time);

            let start_x = if i == 0 {
                0.0
            } else {
                (x_scale(display_history[i - 1].time) + current_x) / 2.0
            };

            let end_x = if i == last_idx {
                width as f64
            } else {
                (current_x + x_scale(display_history[i + 1].time)) / 2.0
            };

            regions.push((start_x, end_x, p.clone()));
        }
        regions
    } else {
        Vec::new()
    };

    // Generate hour labels with percentages for HTML positioning
    let label_items = (0..=24)
        .filter(|h| h % 3 == 0)
        .map(|h| {
            let time = start_time + Duration::hours(h as i64);
            let pct = (h as f64 / 24.0) * 100.0;
            let label = time
                .with_timezone(&chrono::Local)
                .format("%H:%M")
                .to_string();
            (pct, label)
        })
        .collect::<Vec<_>>();

    // Generate Y labels for non-discrete data
    let y_labels = if !is_discrete {
        (0..=4)
            .map(|i| {
                let pct = (i as f64 / 4.0) * 100.0;
                let val = max_v - (i as f64 / 4.0) * (max_v - min_v);
                (pct, val)
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    rsx! {
        div { class: "relative w-full h-full pt-2 pb-8 pl-2 pr-12",
            svg {
                width: "100%",
                height: "100%",
                view_box: "0 0 {width} {height}",
                preserve_aspect_ratio: "none",
                class: "overflow-visible",
                onmouseleave: move |_| hovered_point.set(None),

                // Grid lines (horizontal)
                if !is_discrete {
                    for i in 0..=4 {
                        {
                            let y = (height as f64 / 4.0) * i as f64;
                            rsx! {
                                line {
                                    x1: "0", y1: "{y}", x2: "{width}", y2: "{y}",
                                    stroke: "currentColor", stroke_width: "0.5", class: "text-gray-200 dark:text-gray-700",
                                    vector_effect: "non-scaling-stroke"
                                }
                            }
                        }
                    }
                } else {
                    // Two lines for discrete: Off and On
                    for i in [0.0, 1.0] {
                        {
                            let y = y_scale(i);
                            rsx! {
                                line {
                                    x1: "0", y1: "{y}", x2: "{width}", y2: "{y}",
                                    stroke: "currentColor", stroke_width: "0.5", class: "text-gray-200 dark:text-gray-700",
                                    vector_effect: "non-scaling-stroke"
                                }
                            }
                        }
                    }
                }

                // Path or Points
                if !is_discrete {
                    path {
                        d: "{path_data}",
                        fill: "none",
                        stroke: "{color}",
                        stroke_width: "2",
                        stroke_linejoin: "round",
                        stroke_linecap: "round",
                        vector_effect: "non-scaling-stroke"
                    }
                } else {
                    g {
                        for (x, y, val) in graph_points {
                            {
                                let radius = if val > 0.5 { 5 } else { 3 };
                                let opacity = if val > 0.5 { 1.0 } else { 0.3 };
                                rsx! {
                                    circle {
                                        cx: "{x}",
                                        cy: "{y}",
                                        r: "{radius}",
                                        fill: "{color}",
                                        fill_opacity: "{opacity}"
                                    }
                                }
                            }
                        }
                    }
                }

                // X-Axis ticks (keep in SVG for alignment)
                for (pct, _label) in &label_items {
                    {
                        let x = (*pct / 100.0) * width as f64;
                        rsx! {
                            line {
                                x1: "{x}", y1: "{height}", x2: "{x}", y2: "{height + 5}",
                                stroke: "currentColor", stroke_width: "1.5", class: "text-gray-300 dark:text-gray-200",
                                vector_effect: "non-scaling-stroke"
                            }
                        }
                    }
                }

                // Hit Targets (Transparent)
                g {
                    for (start_x, end_x, p) in hit_regions {
                        rect {
                            x: "{start_x}",
                            y: "0",
                            width: "{end_x - start_x}",
                            height: "{height}",
                            fill: "transparent",
                            style: "pointer-events: all",
                            onmouseenter: move |_| hovered_point.set(Some(p.clone())),
                        }
                    }
                }

                // Hover Indicator
                if let Some(p) = hovered_point() {
                    {
                        let x = x_scale(p.time);
                        let y = y_scale(p.value);
                        rsx! {
                            // Vertical Line
                            line {
                                x1: "{x}", y1: "0", x2: "{x}", y2: "{height}",
                                stroke: "{color}",
                                stroke_width: "1",
                                stroke_dasharray: "4 2",
                                opacity: "0.5",
                                vector_effect: "non-scaling-stroke"
                            }
                            // Dot
                            circle {
                                cx: "{x}", cy: "{y}", r: "4",
                                fill: "{color}",
                                stroke: "white",
                                stroke_width: "2"
                            }
                        }
                    }
                }
            }

            // HTML Labels (prevents font stretching)
            div { class: "absolute bottom-0 left-2 right-12 h-6 pointer-events-none",
                for (pct, label) in label_items {
                    span {
                        class: "absolute text-xs font-bold text-gray-400 dark:text-white whitespace-nowrap",
                        style: "left: {pct}%; transform: translateX(-50%);",
                        "{label}"
                    }
                }
            }

            // Y-Axis Labels
            if !is_discrete {
                div { class: "absolute top-2 bottom-8 right-0 w-12 pointer-events-none",
                    for (pct, val) in y_labels {
                        span {
                            class: "absolute right-2 text-[10px] font-bold text-gray-400 dark:text-white whitespace-nowrap",
                            style: "top: {pct}%; transform: translateY(-50%);",
                            if val >= 1000.0 {
                                "{val / 1000.0:.1}k{unit}"
                            } else if val >= 10.0 {
                                "{val:.0}{unit}"
                            } else {
                                "{val:.1}{unit}"
                            }
                        }
                    }
                }
            }

            // Tooltip Popup
            if let Some(p) = hovered_point() {
                {
                    let x_pct = (x_scale(p.time) / width as f64) * 100.0;
                    let y_pct = (y_scale(p.value) / height as f64) * 100.0;
                    let time_str = p.time.with_timezone(&chrono::Local).format("%H:%M").to_string();
                    let val_str = if is_discrete {
                         if p.value > 0.5 { "Active".to_string() } else { "Inactive".to_string() }
                    } else {
                         format!("{:.1}{}", p.value, unit)
                    };

                    let is_top = y_pct < 20.0;
                    let is_right = x_pct > 80.0;
                    let transform_style = format!(
                        "translate({}, {})",
                        if is_right { "-100%" } else { "-50%" },
                        if is_top { "10px" } else { "-120%" }
                    );

                    rsx! {
                        div {
                            class: "absolute z-10 pointer-events-none bg-white dark:bg-gray-800 rounded shadow-lg p-2 border border-gray-200 dark:border-gray-700 text-xs",
                            style: "left: {x_pct}%; top: {y_pct}%; transform: {transform_style};",

                            div { class: "font-bold text-gray-700 dark:text-gray-200 whitespace-nowrap", "{time_str}" }
                            div { class: "text-gray-600 dark:text-gray-400 whitespace-nowrap", "{val_str}" }
                        }
                    }
                }
            }
        }
    }
}
