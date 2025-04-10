# StreamDeck HomeAssistant

A Rust library for integrating Elgato Stream Deck devices with Home Assistant,
allowing you to control your smart home devices directly from your Stream Deck.

## Features

- Connect to Home Assistant via WebSocket API
- Control switches and lights
- Support for RGB lights with color selection
- Nested menu navigation
- Persistent connection with automatic reconnection

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamdeck-homeassistant = "0.1.0"
```

## Usage

### Basic Example

```rust
use std::{any::{Any, TypeId}, collections::BTreeMap, env, sync::Arc};

use streamdeck_homeassistant::{
    config::{self, HomeAssistantConfig},
    plugins::{self, HomeAssistantPlugin},
    PersistentHassConnection
};
use streamdeck_oxide::{
    elgato_streamdeck, 
    generic_array::typenum::{U3, U5}, 
    plugins::{Plugin, PluginContext, PluginNavigation}, 
    run_with_external_triggers, 
    ExternalTrigger, 
    RenderConfig, 
    Theme
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the configuration
    let config: HomeAssistantConfig = config::load_config("config.yaml")?;
    let config = Arc::new(config);

    // Connect to the Stream Deck
    let hid = elgato_streamdeck::new_hidapi()?;
    let devices = elgato_streamdeck::list_devices(&hid);
    let (kind, serial) = devices
        .into_iter()
        .find(|(kind, _)| *kind == elgato_streamdeck::info::Kind::Mk2)
        .ok_or("No Stream Deck found")?;

    let deck = Arc::new(elgato_streamdeck::AsyncStreamDeck::connect(
        &hid, kind, &serial,
    )?);

    // Connect to Home Assistant
    let hass = PersistentHassConnection::new(
        config.url.clone(),
        env::var("HASS_API_TOKEN")?,
        std::time::Duration::from_secs(5),
    ).await?;

    // Set up the plugin context
    let context = PluginContext::new(
        BTreeMap::from([
            (TypeId::of::<HomeAssistantConfig>(), Box::new(config.clone()) as Box<dyn Any + Send + Sync>),
            (TypeId::of::<PersistentHassConnection>(), Box::new(hass) as Box<dyn Any + Send + Sync>),
        ]),
    );

    // Set up the navigation
    let (sender, receiver) = tokio::sync::mpsc::channel::<ExternalTrigger<PluginNavigation<U5, U3>, U5, U3, PluginContext>>(1);
    let default_menu = config.menu.clone();
    sender.send(ExternalTrigger::new(
        PluginNavigation::<U5, U3>::new(plugins::HomeAssistantPlugin {
            menu: default_menu,
            back_navigation: None,
        }),
        true
    )).await?;

    // Run the Stream Deck
    run_with_external_triggers(Theme::light(), RenderConfig::default(), deck, context, receiver).await?;

    Ok(())
}
```

### Configuration

Create a `config.yaml` file with your Home Assistant configuration:

```yaml
url: "ws://homeassistant.local:8123/api/websocket"
menu:
  name: "Home"
  buttons:
    - type: "switch"
      entity_id: "switch.living_room_light"
      name: "Living Room Light"

    - type: "rgb_light"
      entity_id: "light.bedroom_rgb"
      name: "Bedroom RGB"

    - type: "menu"
      name: "Kitchen"
      buttons:
        - type: "switch"
          entity_id: "switch.kitchen_light"
          name: "Kitchen Light"
```

Set your Home Assistant API token as an environment variable:

```bash
export HASS_API_TOKEN="your_long_lived_access_token"
```

## Beta Version

This library is currently in beta. While it is functional, there may be bugs or
incomplete features. Please report any issues you encounter. Colorful buttons
for RGB lights are not yet implemented, due to base library limitations, that
will be fixed in the future.

## License

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
