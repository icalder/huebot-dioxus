use crate::components::Clock;
use dioxus::prelude::*;

#[server(output = StreamingText)]
pub async fn hue_events() -> Result<dioxus::fullstack::TextStream, ServerFnError> {
    let client = crate::hue::get_hue_client();
    
    let stream = client
        .event_stream()
        .await
        .map_err(|e| ServerFnError::new(e))?;
        
    Ok(dioxus::fullstack::TextStream::new(stream))
}

#[component]
pub fn EventLog() -> Element {
    let mut events = use_signal(Vec::<String>::new);

    use_resource(move || async move {
        match hue_events().await {
            Ok(mut stream) => {
                while let Some(Ok(event)) = stream.next().await {
                    events.with_mut(|evs| {
                        evs.push(event);
                        if evs.len() > 20 {
                            evs.remove(0);
                        }
                    });
                }
            }
            Err(e) => {
                events.with_mut(|evs| evs.push(format!("Error connecting to stream: {}", e)));
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
                class: "bg-black text-green-400 p-4 rounded-lg font-mono text-sm h-96 overflow-y-auto",
                for event in events.read().iter().rev() {
                    div { class: "mb-2 border-b border-gray-800 pb-1", "{event}" }
                }
            }
        }
    }
}