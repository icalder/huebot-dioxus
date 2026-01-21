#[allow(dead_code)]
fn get_client() -> &'static super::client::Client {
    super::get_hue_client()
}

#[tokio::test]
async fn test_generated_client() {
    let hue_client = get_client();

    let rooms = hue_client.get_rooms().await.unwrap();
    let room_names: Vec<_> = rooms
        .data
        .iter()
        .filter_map(|r| r.metadata.as_ref().and_then(|m| m.name.as_ref()))
        .collect();
    println!("Rooms: {:?}", room_names);

    let lights = hue_client.get_lights().await.unwrap();
    let light_ids: Vec<_> = lights
        .data
        .iter()
        .filter_map(|l| l.id.as_ref().map(|id| id.to_string()))
        .collect();
    println!("Lights: {:?}", light_ids);
}

#[derive(Debug)]
#[allow(dead_code)]
struct HueLight {
    id: String,
    name: String,
}

#[tokio::test]
async fn test_typescript_get_lights() {
    let hue_client = get_client();
    let lights_response = hue_client.get_lights().await.unwrap();

    let lights: Vec<HueLight> = lights_response
        .data
        .iter()
        .filter_map(|l| {
            let id = l.id.as_ref()?.to_string();
            let name = l.metadata.as_ref()?.name.as_ref()?.to_string();
            Some(HueLight { id, name })
        })
        .collect();

    println!("TypeScript-style Lights: {:#?}", lights);
}

#[derive(Debug)]
#[allow(dead_code)]
enum HueSensor {
    Motion {
        id: String,
        enabled: bool,
        motion: bool,
    },
    Temperature {
        id: String,
        temperature: f64,
    },
    LightLevel {
        id: String,
        light_level: i64,
    },
}

#[tokio::test]
async fn test_typescript_get_sensors() {
    let hue_client = get_client();

    // Motion Sensors
    let motion_response = hue_client.get_motion_sensors().await.unwrap();
    let mut sensors: Vec<HueSensor> = motion_response
        .data
        .iter()
        .filter_map(|m| {
            Some(HueSensor::Motion {
                id: m.id.as_ref()?.to_string(),
                enabled: m.enabled?,
                motion: m.motion.as_ref()?.motion.unwrap_or(false),
            })
        })
        .collect();

    // Temperature Sensors
    let temp_response = hue_client.get_temperatures().await.unwrap();
    sensors.extend(temp_response.data.iter().filter_map(|t| {
        Some(HueSensor::Temperature {
            id: t.id.as_ref()?.to_string(),
            temperature: t.temperature.as_ref()?.temperature.unwrap_or(0.0),
        })
    }));

    // Light Level Sensors
    let light_response = hue_client.get_light_levels().await.unwrap();
    sensors.extend(light_response.data.iter().filter_map(|l| {
        Some(HueSensor::LightLevel {
            id: l.id.as_ref()?.to_string(),
            light_level: l.light.as_ref()?.light_level.unwrap_or(0),
        })
    }));

    println!("TypeScript-style Sensors: {:#?}", sensors);
}

#[tokio::test]
async fn test_typescript_get_sensor() {
    let hue_client = get_client();
    // Fetch all motion sensors to get a valid ID
    let motion_response = hue_client.get_motion_sensors().await.unwrap();

    if let Some(first) = motion_response.data.first() {
        let id = first.id.as_ref().unwrap().to_string();
        println!("Fetching specific sensor ID: {}", id);

        let sensor = hue_client.get_motion_sensor(&id).await.unwrap();
        println!("Fetched Sensor: {:?}", sensor);
    } else {
        println!("No motion sensors found to test get_sensor");
    }
}

#[tokio::test]
async fn test_typescript_configure_sensor() {
    let hue_client = get_client();
    // Fetch all motion sensors to get a valid ID
    let motion_response = hue_client.get_motion_sensors().await.unwrap();

    if let Some(first) = motion_response.data.first() {
        let id = first.id.as_ref().unwrap().to_string();
        let current_state = first.enabled.unwrap_or(true);
        let new_state = !current_state;

        println!(
            "Toggling sensor {} from {} to {}",
            id, current_state, new_state
        );

        let update = super::client::types::MotionPut {
            enabled: Some(new_state),
            sensitivity: None,
            type_: None,
        };

        let _ = hue_client.update_motion_sensor(&id, &update).await.unwrap();

        // Verify
        let verify = hue_client.get_motion_sensor(&id).await.unwrap();
        // Accessing the first element of the list returned by get_motion_sensor
        let verified_state = verify.data[0].enabled.unwrap();

        println!("Sensor state after update: {}", verified_state);
        assert_eq!(verified_state, new_state);

        // Revert
        let revert = super::client::types::MotionPut {
            enabled: Some(current_state),
            sensitivity: None,
            type_: None,
        };
        let _ = hue_client.update_motion_sensor(&id, &revert).await.unwrap();
        println!("Reverted sensor to original state");
    } else {
        println!("No motion sensors found to test configure_sensor");
    }
}
