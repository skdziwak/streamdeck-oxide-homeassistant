use std::{any::{Any, TypeId}, collections::BTreeMap, env, sync::Arc};

use streamdeck_homeassistant::{
    config::{self, HomeAssistantConfig},
    plugins::{self, HomeAssistantPlugin},
    PersistentHassConnection
};
use streamdeck_oxide::{elgato_streamdeck, generic_array::typenum::{U3, U5}, plugins::{Plugin, PluginContext, PluginNavigation}, run_with_external_triggers, ExternalTrigger, RenderConfig, Theme};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the configuration
    let config: HomeAssistantConfig = config::load_config("config.yaml")?;
    let config = Arc::new(config);

    let hid = elgato_streamdeck::new_hidapi()?;
    let devices = elgato_streamdeck::list_devices(&hid);
    let (kind, serial) = devices
        .into_iter()
        .find(|(kind, _)| *kind == elgato_streamdeck::info::Kind::Mk2)
        .ok_or("No Stream Deck found")?;

    println!("Found Stream Deck: {:?} ({})", kind, serial);

    let deck = Arc::new(elgato_streamdeck::AsyncStreamDeck::connect(
        &hid, kind, &serial,
    )?);
    println!("Connected to Stream Deck successfully!");

    let hass = PersistentHassConnection::new(
        config.url.clone(),
        env::var("HASS_API_TOKEN").map_err(|err| {
            format!("Failed to get HASS_API_TOKEN from environment: {}", err)
        })?,
        std::time::Duration::from_secs(5),
    ).await?;

    let context = PluginContext::new(
        BTreeMap::from([
            (TypeId::of::<HomeAssistantConfig>(), Box::new(config.clone()) as Box<dyn Any + Send + Sync>),
            (TypeId::of::<PersistentHassConnection>(), Box::new(hass) as Box<dyn Any + Send + Sync>),
        ]),
    );

    let (sender, receiver) = tokio::sync::mpsc::channel::<ExternalTrigger<PluginNavigation<U5, U3>, U5, U3, PluginContext>>(1);

    let default_menu = config.menu.clone();
    sender.send(ExternalTrigger::new(
        PluginNavigation::<U5, U3>::new(plugins::HomeAssistantPlugin {
            menu: default_menu,
            back_navigation: None,
        }),
        true
    )).await?;

    run_with_external_triggers(Theme::light(), RenderConfig::default(), deck, context, receiver).await?;

    Ok(())
}
