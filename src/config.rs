//! Configuration types and functions for the StreamDeck HomeAssistant integration.

use serde::{Deserialize, Serialize};

/// Main configuration for the HomeAssistant integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct HomeAssistantConfig {
    /// WebSocket URL for the HomeAssistant instance (e.g., "ws://192.168.0.1:8123/api/websocket")
    pub url: String,
    /// Root menu configuration
    pub menu: HomeAssistantMenu,
}

/// Represents a menu in the StreamDeck interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct HomeAssistantMenu {
    /// Display name for the menu
    pub name: String,
    /// List of buttons in this menu
    pub buttons: Vec<HomeAssistantButton>,
}

/// Represents different types of buttons that can be placed on the StreamDeck.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HomeAssistantButton {
    /// A simple on/off switch
    Switch { entity_id: String, name: String },
    /// An RGB light with color control
    RgbLight { entity_id: String, name: String },
    /// A submenu containing more buttons
    Menu(HomeAssistantMenu),
}

/// Loads a configuration from a YAML file.
///
/// # Arguments
///
/// * `arg` - Path to the YAML configuration file
///
/// # Returns
///
/// The parsed configuration or an error
///
/// # Example
///
/// ```no_run
/// use streamdeck_homeassistant::config;
///
/// let config = config::load_config("config.yaml").expect("Failed to load config");
/// println!("Connected to HomeAssistant at: {}", config.url);
/// ```
pub fn load_config<S: Into<String>>(
    arg: S,
) -> Result<HomeAssistantConfig, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(arg.into())?;
    let reader = std::io::BufReader::new(file);
    let config: HomeAssistantConfig = serde_yaml::from_reader(reader)?;
    Ok(config)
}
