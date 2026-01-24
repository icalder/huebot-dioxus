use dioxus::prelude::*;

#[component]
pub fn Clock() -> Element {
    let mut now = use_signal(|| chrono::Local::now().format("%H:%M:%S").to_string());

    use_future(move || async move {
        loop {
            #[cfg(feature = "server")]
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            #[cfg(not(feature = "server"))]
            gloo_timers::future::sleep(std::time::Duration::from_secs(1)).await;

            now.set(chrono::Local::now().format("%H:%M:%S").to_string());
        }
    });

    rsx! {
        span {
            class: "text-lg text-gray-500 font-mono",
            "Local Time: {now}"
        }
    }
}
