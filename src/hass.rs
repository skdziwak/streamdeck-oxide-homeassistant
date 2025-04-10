//! HomeAssistant connection management module.
//!
//! This module provides a persistent connection to a HomeAssistant instance
//! with automatic reconnection and state caching.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use hass_rs::{HassClient, HassEntity};
use tokio::sync::RwLock;

/// A persistent connection to a HomeAssistant instance.
///
/// This struct maintains a connection to HomeAssistant, automatically
/// reconnects if the connection is lost, and caches entity states.
pub struct PersistentHassConnection {
    hass: Arc<RwLock<HassClient>>,
    url: String,
    token: String,
    close: tokio::sync::mpsc::Sender<()>,
    states: RwLock<BTreeMap<String, HassEntity>>,
    update_interval: Duration,
}

impl PersistentHassConnection {
    /// Creates a new persistent connection to HomeAssistant.
    ///
    /// # Arguments
    ///
    /// * `url` - The WebSocket URL of the HomeAssistant instance
    /// * `token` - The long-lived access token for authentication
    /// * `update_interval` - How often to refresh entity states
    ///
    /// # Returns
    ///
    /// An Arc-wrapped connection or an error
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::time::Duration;
    /// use streamdeck_homeassistant::hass::PersistentHassConnection;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let connection = PersistentHassConnection::new(
    ///     "ws://homeassistant.local:8123/api/websocket".to_string(),
    ///     "your_access_token".to_string(),
    ///     Duration::from_secs(5),
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(
        url: String,
        token: String,
        update_interval: Duration,
    ) -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        let (tx, rx) = tokio::sync::mpsc::channel::<()>(1);
        let mut hass = HassClient::new(&url).await?;
        hass.auth_with_longlivedtoken(&token).await?;
        let connection = Self {
            hass: Arc::new(RwLock::new(hass)),
            url,
            token,
            close: tx,
            states: RwLock::new(BTreeMap::new()),
            update_interval,
        };
        let connection = Arc::new(connection);
        let connection_clone = connection.clone();

        tokio::spawn(async move {
            connection_clone.keep_alive(rx).await;
        });

        Ok(connection)
    }

    async fn create_client(&self) -> Result<HassClient, Box<dyn std::error::Error>> {
        let mut client = HassClient::new(&self.url).await?;
        client.auth_with_longlivedtoken(&self.token).await?;
        Ok(client)
    }

    async fn replace_client(&self) -> Result<(), Box<dyn std::error::Error>> {
        let client = self.create_client().await?;
        let mut hass = self.hass.write().await;
        *hass = client;
        Ok(())
    }

    /// Calls a service in HomeAssistant.
    ///
    /// # Arguments
    ///
    /// * `domain` - The domain of the service (e.g., "light", "switch")
    /// * `service` - The service to call (e.g., "turn_on", "turn_off")
    /// * `data` - Optional data to pass to the service
    ///
    /// # Returns
    ///
    /// Ok(()) if successful, or an error
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::sync::Arc;
    /// # use streamdeck_homeassistant::hass::PersistentHassConnection;
    /// # async fn example(hass: Arc<PersistentHassConnection>) -> Result<(), Box<dyn std::error::Error>> {
    /// // Turn on a light
    /// hass.call_service(
    ///     "light",
    ///     "turn_on",
    ///     Some(serde_json::json!({
    ///         "entity_id": "light.living_room",
    ///         "brightness_pct": 75
    ///     })),
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn call_service(
        &self,
        domain: &str,
        service: &str,
        data: Option<serde_json::Value>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut client = self.hass.write().await;
        client
            .call_service(domain.to_string(), service.to_string(), data)
            .await?;
        Ok(())
    }

    /// Fetches all entity states from HomeAssistant and updates the cache.
    ///
    /// # Returns
    ///
    /// Ok(()) if successful, or an error message
    pub async fn fetch_states(&self) -> Result<(), String> {
        let mut client = self.hass.write().await;
        let states = client.get_states().await.map_err(|e| e.to_string())?;
        let mut state_map = self.states.write().await;
        for state in states {
            state_map.insert(state.entity_id.clone(), state);
        }
        Ok(())
    }

    /// Gets the state of an entity from the cache.
    ///
    /// # Arguments
    ///
    /// * `entity_id` - The ID of the entity (e.g., "light.living_room")
    ///
    /// # Returns
    ///
    /// The entity state if found, or None
    pub async fn get_state(&self, entity_id: &str) -> Option<HassEntity> {
        let state_map = self.states.read().await;
        state_map.get(entity_id).cloned()
    }

    async fn keep_alive(self: Arc<Self>, mut end: tokio::sync::mpsc::Receiver<()>) {
        loop {
            let close_future = end.recv();
            let fetch_future = self.fetch_states();
            tokio::select! {
                _ = close_future => {
                    println!("Closing connection");
                    break;
                }
                result = fetch_future => {
                    if let Err(e) = result {
                        eprintln!("Error fetching states: {}", e);
                        match self.replace_client().await {
                            Ok(_) => {
                                println!("Replaced client");
                            }
                            Err(e) => {
                                eprintln!("Error replacing client: {}", e);
                            }
                        }
                    }
                }
            }
            tokio::time::sleep(self.update_interval).await;
        }
    }
}

impl Drop for PersistentHassConnection {
    fn drop(&mut self) {
        let _ = self.close.try_send(());
    }
}
