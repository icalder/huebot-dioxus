use crate::components::Clock;
use crate::hue::hue_events;
use dioxus::prelude::*;
use std::collections::HashMap;

#[server]
pub async fn get_device_names() -> Result<HashMap<String, String>, ServerFnError> {
    let client = crate::hue::get_hue_client();
    client
        .get_name_map()
        .await
        .map_err(|e| ServerFnError::new(e))
}

#[component]
pub fn EventLog() -> Element {
    let mut events = use_signal(Vec::<(String, String)>::new);
    let names = use_resource(get_device_names);

    use_resource(move || async move {
        match hue_events().await {
            Ok(mut stream) => {
                while let Some(Ok(event)) = stream.next().await {
                    let v: serde_json::Value = serde_json::from_str(&event).unwrap_or(serde_json::Value::Null);
                    
                    // Try to extract the resource ID or the owner ID
                    let id = v.get("id").and_then(|id| id.as_str());
                    let owner_rid = v.get("owner").and_then(|o| o.get("rid")).and_then(|rid| rid.as_str());

                    let device_name = {
                        let names_ready = names.read();
                        let map = names_ready.as_ref().and_then(|res| res.as_ref().ok());
                        
                        if let Some(map) = map {
                            if let Some(id) = id {
                                if let Some(name) = map.get(id) {
                                    Some(name.clone())
                                } else if let Some(owner_rid) = owner_rid {
                                    map.get(owner_rid).cloned()
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    };

                    let display_name = device_name.unwrap_or_else(|| {
                        if let Some(id) = id {
                            format!("Unknown ({})", &id[..8.min(id.len())])
                        } else {
                            "System".to_string()
                        }
                    });

                    events.with_mut(|evs| {
                        evs.push((display_name, event));
                        if evs.len() > 20 {
                            evs.remove(0);
                        }
                    });
                }
            }
            Err(e) => {
                events.with_mut(|evs| {
                    evs.push(("Error".to_string(), format!("Connection error: {}", e)))
                });
            }
        }
    });

    rsx! {
        div {
            class: "container mx-auto p-4",
            div {
                class: "flex justify-between items-baseline mb-4",
                h1 { class: "text-2xl font-bold", "Hue Event Log (Streaming)" }
                Clock {}
            }
            div {
                class: "bg-black text-gray-300 p-4 rounded-lg font-mono text-xs h-[32rem] overflow-y-auto",
                for (name, event) in events.read().iter().rev() {
                    div { 
                        class: "flex gap-4 mb-2 border-b border-gray-800 pb-2",
                        span { 
                            class: "w-32 flex-shrink-0 text-blue-400 font-bold truncate",
                            title: "{name}",
                            "{name}" 
                        }
                        span { 
                            class: "flex-grow break-all text-green-500/80",
                            "{event}" 
                        }
                    }
                }
            }
        }
    }
}