# Development

Your new jumpstart project includes basic organization with an organized `assets` folder and a `components` folder.
If you chose to develop with the router feature, you will also have a `views` folder.

```
project/
├─ assets/ # Any assets that are used by the app should be placed here
├─ src/
│  ├─ main.rs # The entrypoint for the app. It also defines the routes for the app.
│  ├─ components/
│  │  ├─ mod.rs # Defines the components module
│  │  ├─ hero.rs # The Hero component for use in the home page
│  │  ├─ echo.rs # The echo component uses server functions to communicate with the server
│  ├─ views/ # The views each route will render in the app.
│  │  ├─ mod.rs # Defines the module for the views route and re-exports the components for each route
│  │  ├─ blog.rs # The component that will render at the /blog/:id route
│  │  ├─ home.rs # The component that will render at the / route
├─ Cargo.toml # The Cargo.toml file defines the dependencies and feature flags for your project
```

### Automatic Tailwind (Dioxus 0.7+)

As of Dioxus 0.7, there no longer is a need to manually install tailwind. Simply `dx serve` and you're good to go!

Automatic tailwind is supported by checking for a file called `tailwind.css` in your app's manifest directory (next to Cargo.toml). To customize the file, use the dioxus.toml:

```toml
[application]
tailwind_input = "my.css"
tailwind_output = "assets/out.css"
```

### Tailwind Manual Install

To use tailwind plugins or manually customize tailwind, you can can install the Tailwind CLI and use it directly.

1. Install npm: https://docs.npmjs.com/downloading-and-installing-node-js-and-npm
2. Install the Tailwind CSS CLI: https://tailwindcss.com/docs/installation/tailwind-cli
3. Run the following command in the root of the project to start the Tailwind CSS compiler:

```bash
npx @tailwindcss/cli -i ./input.css -o ./assets/tailwind.css --watch
```

Note: if you create any tailwind css files in `assets/styling` they *must* be imported in `tailwind.css`.

### Serving Your App

Run the following command in the root of your project to start developing with the default platform:

```bash
dx serve --platform web
# if you want to allow testing from other devices ion your network use:
dx serve --addr 0.0.0.0
```

To run for a different platform, use the `--platform platform` flag. E.g.
```bash
dx serve --platform desktop
```

### Philips Hue OpenAPI Spec

[openhue-api](https://github.com/openhue/openhue-api)

The macro runs at compile time.  To see docs for the generated code, use: `cargo doc --no-deps --open`
NB: `nix-shell -p wslu` to get `wslview`.

`cargo test hue::tests -- --nocapture`

### Container Build & Run

You can build a container image for the application using Nix. This creates a layered image that can be loaded into Podman or Docker.

1.  **Build the image:**
    ```bash
    nix build .#docker
    ```
    This will produce a `result` symlink pointing to the tarball of the image.

2.  **Load the image into Podman:**
    ```bash
    podman load < result
    ```
    The image will be tagged as `huebot:latest`.

3.  **Run the container:**
    ```bash
    podman run --rm -p 8080:8080 huebot:latest
    ```
    The application will be accessible at `http://localhost:8080`.

4.  **Push to Docker Hub:**
    Tag the image with your Docker Hub username and push it:
    ```bash
    podman tag huebot:latest icalder/huebot:latest
    podman push icalder/huebot:latest
    ```

### TODO

  Here is a summary of the changes:
   1. OpenAPI Specification: Added motion_area_candidate and device_software_update to the resource type enums in
      hue-openapi.yaml. This fixed the "Invalid Response Payload" error caused by progenitor failing to deserialize
      unknown variants returned by the Hue Bridge.
