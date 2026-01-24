use dioxus::prelude::*; 
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use crate::components::{SensorDataGraph, HistoryPoint};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphPoint<T> {
    pub timestamp: DateTime<Utc>,
    pub value: T,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensorGraphData {
    pub name: String,
    pub motions: Vec<GraphPoint<bool>>,
    pub temperatures: Vec<GraphPoint<f32>>,
    pub light_levels: Vec<GraphPoint<i32>>,
}

#[server]
pub async fn get_graph_data(sensor_id: String) -> Result<SensorGraphData, ServerFnError> {
    let client = crate::hue::get_hue_client();
    let pool = crate::hue::get_db_pool().await?;

    let sensors = client.get_sensors().await.map_err(|e| ServerFnError::new(e.to_string()))?;
    let sensor = sensors.iter().find(|s| s.device_id == sensor_id)
        .ok_or_else(|| ServerFnError::new("Sensor not found"))?;

    let end = Utc::now();
    let start = end - Duration::hours(24);

    let mut motions = Vec::new();
    let mut temperatures = Vec::new();
    let mut light_levels = Vec::new();

    let extract_id = |id_v1: &Option<String>| {
        id_v1.as_ref()
            .and_then(|s| s.split('/').last())
            .and_then(|s| s.parse::<i32>().ok())
    };

    if let Some(m) = &sensor.motion {
        if let Some(v1_id) = extract_id(&m.id_v1) {
            let rows = sqlx::query!(
                "select creationtime as \"creationtime!\", motion from sensor_motion($1, $2, $3)",
                v1_id, start, end
            ).fetch_all(&pool).await.map_err(|e| ServerFnError::new(e.to_string()))?;

            for row in rows {
                if let Some(value) = row.motion {
                    motions.push(GraphPoint {
                        timestamp: DateTime::from_naive_utc_and_offset(row.creationtime, Utc),
                        value,
                    });
                }
            }
        }
    }

    if let Some(t) = &sensor.temperature {
        if let Some(v1_id) = extract_id(&t.id_v1) {
            let rows = sqlx::query!(
                "select creationtime as \"creationtime!\", temperature from sensor_temperature($1, $2, $3)",
                v1_id, start, end
            ).fetch_all(&pool).await.map_err(|e| ServerFnError::new(e.to_string()))?;

            for row in rows {
                if let Some(value) = row.temperature {
                    temperatures.push(GraphPoint {
                        timestamp: DateTime::from_naive_utc_and_offset(row.creationtime, Utc),
                        value,
                    });
                }
            }
        }
    }

    if let Some(l) = &sensor.light {
        if let Some(v1_id) = extract_id(&l.id_v1) {
            let rows = sqlx::query!(
                "select creationtime as \"creationtime!\", light_level from sensor_light_level($1, $2, $3)",
                v1_id, start, end
            ).fetch_all(&pool).await.map_err(|e| ServerFnError::new(e.to_string()))?;

            for row in rows {
                if let Some(value) = row.light_level {
                    light_levels.push(GraphPoint {
                        timestamp: DateTime::from_naive_utc_and_offset(row.creationtime, Utc),
                        value,
                    });
                }
            }
        }
    }

    Ok(SensorGraphData {
        name: sensor.name.clone(),
        motions,
        temperatures,
        light_levels,
    })
}

#[component]
pub fn Graphs(sensor_id: String) -> Element {
    let data = use_loader(move || get_graph_data(sensor_id.clone()))?;
    let data = data.read();

    let motion_history = data.motions.iter().map(|p| HistoryPoint {
        value: if p.value { 1.0 } else { 0.0 },
        time: p.timestamp,
    }).collect::<Vec<_>>();

    let temp_history = data.temperatures.iter().map(|p| HistoryPoint {
        value: p.value as f64,
        time: p.timestamp,
    }).collect::<Vec<_>>();

    let light_history = data.light_levels.iter().map(|p| HistoryPoint {
        value: p.value as f64,
        time: p.timestamp,
    }).collect::<Vec<_>>();

    rsx! {
        div {
            class: "w-full p-4 pb-20 max-w-[100vw] overflow-x-hidden",
            div { class: "max-w-7xl mx-auto",
                div { class: "flex items-center gap-4 mb-6",
                    Link {
                        to: crate::Route::Sensors {},
                        class: "p-2 rounded-full hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors",
                        "←"
                    }
                    h1 { class: "text-2xl font-bold", "Sensor Graphs for {data.name}" }
                }
                div {
                    class: "grid grid-cols-1 gap-8",
                    div {
                        class: "p-4 bg-white dark:bg-gray-800 rounded-lg shadow w-full",
                        h2 { class: "text-lg font-semibold mb-2", "Motion" }
                        div { class: "h-64 w-full", 
                            SensorDataGraph { 
                                history: motion_history, 
                                is_discrete: true, 
                                color: "#f87171" // red-400
                            }
                        }
                    }
                                    div {
                                        class: "p-4 bg-white dark:bg-gray-800 rounded-lg shadow w-full",
                                        h2 { class: "text-lg font-semibold mb-2", "Temperature" }
                                        div { class: "h-64 w-full", 
                                            SensorDataGraph { 
                                                history: temp_history, 
                                                unit: "°C".to_string(),
                                                color: "#60a5fa" // blue-400
                                            }
                                        }
                                    }
                                    div {
                                        class: "p-4 bg-white dark:bg-gray-800 rounded-lg shadow w-full",
                                        h2 { class: "text-lg font-semibold mb-2", "Light Level" }
                                        div { class: "h-64 w-full", 
                                            SensorDataGraph { 
                                                history: light_history, 
                                                unit: "lx".to_string(),
                                                color: "#fbbf24" // amber-400
                                            }
                                        }
                                    }                }
            }
        }
    }
}