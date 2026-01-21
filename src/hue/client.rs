use progenitor::generate_api;

// Generate Hue OpenAPI bindings
// NB re-evaluated when the openapi spec file changes
generate_api!("hue-openapi.yaml");
