You are an expert [0.7 Dioxus](https://dioxuslabs.com/learn/0.7) assistant. Dioxus 0.7 changes every api in dioxus. Only use this up to date documentation. `cx`, `Scope`, and `use_state` are gone

Provide concise code examples with detailed descriptions

# Dioxus Dependency

You can add Dioxus to your `Cargo.toml` like this:

```toml
[dependencies]
dioxus = { version = "0.7.1" }

[features]
default = ["web", "webview", "server"]
web = ["dioxus/web"]
webview = ["dioxus/desktop"]
server = ["dioxus/server"]
```

# Launching your application

You need to create a main function that sets up the Dioxus runtime and mounts your root component.

```rust
use dioxus::prelude::*;

fn main() {
	dioxus::launch(App);
}

#[component]
fn App() -> Element {
	rsx! { "Hello, Dioxus!" }
}
```

Then serve with `dx serve`:

```sh
curl -sSL http://dioxus.dev/install.sh | sh
dx serve
```

# UI with RSX

```rust
rsx! {
	div {
		class: "container", // Attribute
		color: "red", // Inline styles
		width: if condition { "100%" }, // Conditional attributes
		"Hello, Dioxus!"
	}
	// Prefer loops over iterators
	for i in 0..5 {
		div { "{i}" } // use elements or components directly in loops
	}
	if condition {
		div { "Condition is true!" } // use elements or components directly in conditionals
	}

	{children} // Expressions are wrapped in brace
	{(0..5).map(|i| rsx! { span { "Item {i}" } })} // Iterators must be wrapped in braces
}
```

# Assets

The asset macro can be used to link to local files to use in your project. All links start with `/` and are relative to the root of your project.

```rust
rsx! {
	img {
		src: asset!("/assets/image.png"),
		alt: "An image",
	}
}
```

## Styles

The `document::Stylesheet` component will inject the stylesheet into the `<head>` of the document

```rust
rsx! {
	document::Stylesheet {
		href: asset!("/assets/styles.css"),
	}
}
```

Important: tailwind classes must be inlined or defined with @apply. Do not use rust string constants, they break hot-reloading.

# Components

Components are the building blocks of apps

* Component are functions annotated with the `#[component]` macro.
* The function name must start with a capital letter or contain an underscore.
* A component re-renders only under two conditions:
	1.  Its props change (as determined by `PartialEq`).
	2.  An internal reactive state it depends on is updated.

```rust
#[component]
fn Input(mut value: Signal<String>) -> Element {
	rsx! {
		input {
            value,
			oninput: move |e| {
				*value.write() = e.value();
			},
			onkeydown: move |e| {
				if e.key() == Key::Enter {
					value.write().clear();
				}
			},
		}
	}
}
```

Each component accepts function arguments (props)

* Props must be owned values, not references. Use `String` and `Vec<T>` instead of `&str` or `&[T]`.
* Props must implement `PartialEq` and `Clone`.
* To make props reactive and copy, you can wrap the type in `ReadOnlySignal`. Any reactive state like memos and resources that read `ReadOnlySignal` props will automatically re-run when the prop changes.

# State

A signal is a wrapper around a value that automatically tracks where it's read and written. Changing a signal's value causes code that relies on the signal to rerun.

## Local State

The `use_signal` hook creates state that is local to a single component. You can call the signal like a function (e.g. `my_signal()`) to clone the value, or use `.read()` to get a reference. `.write()` gets a mutable reference to the value.

Use `use_memo` to create a memoized value that recalculates when its dependencies change. Memos are useful for expensive calculations that you don't want to repeat unnecessarily.

```rust
#[component]
fn Counter() -> Element {
	let mut count = use_signal(|| 0);
	let mut doubled = use_memo(move || count() * 2); // doubled will re-run when count changes because it reads the signal

	rsx! {
		h1 { "Count: {count}" } // Counter will re-render when count changes because it reads the signal
		h2 { "Doubled: {doubled}" }
		button {
			onclick: move |_| *count.write() += 1, // Writing to the signal rerenders Counter
			"Increment"
		}
		button {
			onclick: move |_| count.with_mut(|count| *count += 1), // use with_mut to mutate the signal
			"Increment with with_mut"
		}
	}
}
```

## Context API

The Context API allows you to share state down the component tree. A parent provides the state using `use_context_provider`, and any child can access it with `use_context`

```rust
#[component]
fn App() -> Element {
	let mut theme = use_signal(|| "light".to_string());
	use_context_provider(|| theme); // Provide a type to children
	rsx! { Child {} }
}

#[component]
fn Child() -> Element {
	let theme = use_context::<Signal<String>>(); // Consume the same type
	rsx! {
		div {
			"Current theme: {theme}"
		}
	}
}
```

# Async

For state that depends on an asynchronous operation (like a network request), Dioxus provides a hook called `use_resource`. This hook manages the lifecycle of the async task and provides the result to your component.

* The `use_resource` hook takes an `async` closure. It re-runs this closure whenever any signals it depends on (reads) are updated
* The `Resource` object returned can be in several states when read:
1. `None` if the resource is still loading
2. `Some(value)` if the resource has successfully loaded

```rust
let mut dog = use_resource(move || async move {
	// api request
});

match dog() {
	Some(dog_info) => rsx! { Dog { dog_info } },
	None => rsx! { "Loading..." },
}
```

There is also `use_loader`.  Example:

```rust
#[server]
async fn get_sensors() -> Result<Vec<SensorViewData>, ServerFnError> {
    // ... code to fetch sensors using await

    Ok(sensors)
}

/// The Sensors page component that will be rendered when the current route is `[Route::Sensors]`
#[component]
pub fn Sensors() -> Element {
    let sensors = use_loader(get_sensors)?;

    rsx! {
        div {
            h1 { "Sensors" }
            ul {
                for sensor in sensors.read().iter() {
                    li { key: "{sensor.id}", "{sensor.name}" }
                }
            }
        }
    }
}
```

# Routing

All possible routes are defined in a single Rust `enum` that derives `Routable`. Each variant represents a route and is annotated with `#[route("/path")]`. Dynamic Segments can capture parts of the URL path as parameters by using `:name` in the route string. These become fields in the enum variant.

The `Router<Route> {}` component is the entry point that manages rendering the correct component for the current URL.

You can use the `#[layout(NavBar)]` to create a layout shared between pages and place an `Outlet<Route> {}` inside your layout component. The child routes will be rendered in the outlet.

```rust
#[derive(Routable, Clone, PartialEq)]
enum Route {
	#[layout(NavBar)] // This will use NavBar as the layout for all routes
		#[route("/")]
		Home {},
		#[route("/blog/:id")] // Dynamic segment
		BlogPost { id: i32 },
}

#[component]
fn NavBar() -> Element {
	rsx! {
		a { href: "/", "Home" }
		Outlet<Route> {} // Renders Home or BlogPost
	}
}

#[component]
fn App() -> Element {
	rsx! { Router::<Route> {} }
}
```

```toml
dioxus = { version = "0.7.1", features = ["router"] }
```

# Fullstack

Fullstack enables server rendering and ipc calls. It uses Cargo features (`server` and a client feature like `web`) to split the code into a server and client binaries.

```toml
dioxus = { version = "0.7.1", features = ["fullstack"] }
```

## Server Functions

Use the `#[post]` / `#[get]` macros to define an `async` function that will only run on the server. On the server, this macro generates an API endpoint. On the client, it generates a function that makes an HTTP request to that endpoint.

```rust
#[post("/api/double/:path/&query")]
async fn double_server(number: i32, path: String, query: i32) -> Result<i32, ServerFnError> {
	tokio::time::sleep(std::time::Duration::from_secs(1)).await;
	Ok(number * 2)
}
```

## Hydration

Hydration is the process of making a server-rendered HTML page interactive on the client. The server sends the initial HTML, and then the client-side runs, attaches event listeners, and takes control of future rendering.

### Errors
The initial UI rendered by the component on the client must be identical to the UI rendered on the server.

* Use the `use_server_future` hook instead of `use_resource`. It runs the future on the server, serializes the result, and sends it to the client, ensuring the client has the data immediately for its first render.
* Any code that relies on browser-specific APIs (like accessing `localStorage`) must be run *after* hydration. Place this code inside a `use_effect` hook.

# Progenitor

Progenitor is used to dynamically create a Hue APIU client from the OpenAPI spec. You can create docs including the generated structs by running `cargo doc --no-deps`. The docs are created in `target/doc/huebot`.

# Reliability & Performance

The Hue Bridge is sensitive to concurrent requests and can return 404 HTML errors when overwhelmed.

## Concurrency Limiting (Semaphore)
The `ClientEx` struct in `src/hue/client.rs` uses a `tokio::sync::Semaphore` to limit concurrent bridge requests to **3**.

## Retry Logic
A `retry` helper in `src/hue/client.rs` handles transient bridge errors with:
- **5 attempts** maximum.
- **Exponential backoff**: `attempts * attempts * 100ms` (up to 2.5s delay).
- Detailed logging of transient and persistent errors.

## Metadata Caching

To avoid redundant bridge load, sensor metadata is cached in `src/hue/mod.rs` via `SENSORS_CACHE`:

- **5-minute TTL**: Cache expires after 5 minutes.

- **`get_sensors_cached()`**: Use this helper instead of direct bridge calls when up-to-the-second precision isn't required (e.g., loading graph metadata).



# Animations & VDOM Keys

Dioxus efficiently updates existing DOM elements by default. However, **one-shot CSS animations** only trigger when an element is first created. If Dioxus simply updates an attribute or text node, the browser will not restart the animation.

## Triggering One-Shot Animations

To force an animation to replay when data updates, use the `key` attribute. When the `key` changes, Dioxus destroys the old element and mounts a brand-new one, causing the browser to trigger its CSS animation class from the start.

```rust
#[component]
fn HighlightedValue(value: String, timestamp: String) -> Element {
    rsx! {
        div {
            // Changing the key forces a remount, triggering 'animate-pulse-once'
            key: "{timestamp}",
            class: "animate-pulse-once",
            "{value}"
        }
    }
}
```

### Best Practices:
1.  **Stable Keys:** Ensure the key only changes when you actually want the animation to fire (e.g., use a data fingerprint or a "last updated" timestamp). 
2.  **Key Placement:** In Dioxus 0.7, keys should ideally be placed on the **first node** of a component's output or at the **component call site** to avoid VDOM warnings and ensure reliable remounting.
3.  **Encapsulation:** Wrap the keyed element in a generic component (like `Pulsing {}`) to reuse the animation logic throughout the app.

---

# Event Handling & Parsing Patterns



When consuming external APIs (like Hue's EventStream) where some events are strictly typed and others must be passed through:



## 1. Avoid `#[serde(untagged)]` for Mixed Data

Untagged enums can lead to silent failures or ambiguous matching when payloads share fields. It is brittle when mixing strict types with a catch-all.



## 2. Avoid `#[serde(other)]` for Data Capture

`#[serde(other)]` only works on **unit variants**. If used, the payload of the unknown event is discarded. Do not use it if you need to log or process the unknown data.



## 3. Recommended Pattern: Manual Dispatch + Raw Fallback

For robust handling of partial schemas:

1. Define a `Raw(serde_json::Value)` variant in your enum.

2. Implement a `from_json` method that manually inspects the discriminator field (e.g., `type`).

3. Match known types to specific struct variants.

4. Fallback to `Raw(v)` for anything else.



```rust

impl HueEvent {

    pub fn from_json(v: &serde_json::Value) -> Option<Self> {

        let t = v.get("type").and_then(|t| t.as_str());

        match t {

            Some("motion") => { /* construct Motion variant */ },

            _ => Some(Self::Raw(v.clone())), // Capture everything else

        }

    }

}

```



## 4. Helper Accessors

Ensure helper methods (like `id()` or `owner()`) handle the `Raw` variant by dynamically extracting data from the JSON `Value`. This allows UI components to display metadata even for unmodeled events.



## 5. Transparent Streaming

When acting as a proxy (Server -> Client), prefer streaming the raw JSON `String` rather than re-serializing the Rust Enum. This guarantees data fidelity and prevents serialization artifacts from breaking the client parser.
