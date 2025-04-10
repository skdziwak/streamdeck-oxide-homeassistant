//! # StreamDeck HomeAssistant Integration
//! 
//! This library provides integration between Elgato Stream Deck devices and Home Assistant,
//! allowing you to control your smart home devices directly from your Stream Deck.
//! 
//! ## Features
//! 
//! - Connect to Home Assistant via WebSocket API
//! - Control switches and lights
//! - Support for RGB lights with color selection
//! - Nested menu navigation
//! - Persistent connection with automatic reconnection

pub mod config;
pub mod plugins;
pub mod hass;

// Re-export main types for convenience
pub use config::{HomeAssistantConfig, HomeAssistantMenu, HomeAssistantButton};
pub use hass::PersistentHassConnection;
pub use plugins::HomeAssistantPlugin;