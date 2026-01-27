use crate::components::Clock;
use dioxus::prelude::*;
use std::collections::HashMap;

#[server]
pub async fn get_device_names() -> Result<HashMap<String, String>, ServerFnError> {
    let client = crate::hue::get_hue_client();
    let names: HashMap<String, String> = client.get_name_map().await.map_err(|e| {
        let msg = e.to_string();
        ServerFnError::new(if msg.len() > 100 { &msg[..100] } else { &msg })
    })?;
    Ok(names)
}

#[component]
pub fn EventLog() -> Element {
    let mut events = use_signal(Vec::<(String, String)>::new);
    let names = use_resource(get_device_names);

    crate::hue::use_hue_event_handler(
        false,
        move |event| {
            let id = event.resource_id();
            let owner_rid = event.owner_rid();

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
                    } else if let Some(owner_rid) = owner_rid {
                        map.get(owner_rid).cloned()
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

            let event_str = match &event {
                crate::hue::events::HueEvent::Raw(v) => serde_json::to_string(v).unwrap_or_default(),
                _ => serde_json::to_string(&event).unwrap_or_default(),
            };

            events.with_mut(|evs: &mut Vec<(String, String)>| {
                evs.push((display_name, event_str));
                if evs.len() > 20 {
                    evs.remove(0);
                }
            });
        },
        move |msg| {
            events.with_mut(|evs| {
                evs.push(("Error".to_string(), format!("Connection error: {}", msg)))
            });
        },
    );

    rsx! {
        div { class: "container mx-auto p-4",
            div { class: "flex justify-between items-baseline mb-4",
                h1 { class: "text-2xl font-bold", "Hue Event Log (Streaming)" }
                Clock {}
            }
            div { class: "bg-black text-gray-300 p-4 rounded-lg font-mono text-xs h-[32rem] overflow-y-auto",
                for (name , event) in events.read().iter().rev() {
                    div { class: "flex gap-4 mb-2 border-b border-gray-800 pb-2",
                        span {
                            class: "w-32 flex-shrink-0 text-blue-400 font-bold truncate",
                            title: "{name}",
                            "{name}"
                        }
                        span { class: "flex-grow break-all text-green-500/80", "{event}" }
                    }
                }
            }
        }
    }
}
