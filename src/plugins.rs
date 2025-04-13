//! StreamDeck plugins for HomeAssistant integration.
//!
//! This module contains the main plugin implementation and specialized
//! plugins for different types of HomeAssistant entities.

pub mod rgb;
use std::sync::Arc;

use resvg::tiny_skia::Color;
use streamdeck_oxide::{
    generic_array::{
        typenum::{U3, U5},
        ArrayLength,
    },
    md_icons,
    plugins::{Plugin, PluginContext, PluginNavigation},
    view::customizable::{ClickButton, CustomizableView, ToggleButton},
    Theme, View,
};

use crate::{
    config::{HomeAssistantButton, HomeAssistantConfig, HomeAssistantMenu},
    hass::PersistentHassConnection,
};

/// Main plugin for HomeAssistant integration.
///
/// This plugin renders a menu of HomeAssistant entities on the Stream Deck
/// and handles navigation between menus.
#[derive(Clone)]
pub struct HomeAssistantPlugin<W: ArrayLength, H: ArrayLength> {
    /// The menu configuration to display
    pub menu: HomeAssistantMenu,
    /// Optional navigation to return to when "Back" is pressed
    pub back_navigation: Option<PluginNavigation<W, H>>,
}

/// Adds a button to the view based on the HomeAssistant button configuration.
///
/// # Arguments
///
/// * `view` - The view to add the button to
/// * `x` - The x coordinate on the Stream Deck
/// * `y` - The y coordinate on the Stream Deck
/// * `item` - The button configuration
/// * `back_navigation` - Optional navigation for nested menus
fn add_button<W, H>(
    view: &mut CustomizableView<W, H, PluginContext, PluginNavigation<W, H>>,
    x: usize,
    y: usize,
    item: &HomeAssistantButton,
    back_navigation: &Option<PluginNavigation<W, H>>,
) -> Result<(), Box<dyn std::error::Error>>
where
    W: ArrayLength,
    H: ArrayLength,
{
    match item {
        HomeAssistantButton::Switch { entity_id, name } => {
            let entity_id = entity_id.clone();
            let entity_id_2 = entity_id.clone();
            view.set_button(
                x,
                y,
                ToggleButton::new(
                    name,
                    Some(md_icons::filled::ICON_TOGGLE_OFF),
                    move |ctx: PluginContext| {
                        let entity_id = entity_id.clone();
                        async move {
                            let hass = ctx
                                .get_context::<PersistentHassConnection>()
                                .await
                                .ok_or("Failed to get PersistentHassConnection")?;
                            let state = hass
                                .get_state(&entity_id)
                                .await
                                .ok_or("Failed to get state")?;

                            Ok(state.state == "on")
                        }
                    },
                    move |ctx, value| {
                        let entity_id = entity_id_2.clone();
                        async move {
                            let hass = ctx
                                .get_context::<PersistentHassConnection>()
                                .await
                                .ok_or("Failed to get PersistentHassConnection")?;
                            hass.call_service(
                                "switch",
                                if value { "turn_on" } else { "turn_off" },
                                Some(serde_json::json!({ "entity_id": entity_id })),
                            )
                            .await
                            .map_err(|e| e.to_string())?;
                            Ok(())
                        }
                    },
                )
                .when_active(name, Some(md_icons::filled::ICON_TOGGLE_ON)),
            )
        }
        HomeAssistantButton::RgbLight { entity_id, name } => view.set_navigation(
            x,
            y,
            PluginNavigation::new(rgb::RgbControllerPlugin {
                entity_id: entity_id.clone(),
                back_navigation: back_navigation.clone(),
            }),
            name,
            Some(md_icons::filled::ICON_LIGHTBULB),
        ),
        HomeAssistantButton::Menu(home_assistant_menu) => view.set_navigation(
            x,
            y,
            PluginNavigation::new(HomeAssistantPlugin {
                menu: home_assistant_menu.clone(),
                back_navigation: back_navigation.clone(),
            }),
            home_assistant_menu.name.clone(),
            Some(md_icons::filled::ICON_MENU),
        ),
    }
}

/// Generates a menu view from a HomeAssistantPlugin configuration.
///
/// # Arguments
///
/// * `plugin` - The plugin configuration
///
/// # Returns
///
/// A customizable view with buttons configured according to the plugin
fn generate_menu<W, H>(
    plugin: &HomeAssistantPlugin<W, H>,
) -> Result<CustomizableView<W, H, PluginContext, PluginNavigation<W, H>>, Box<dyn std::error::Error>>
where
    W: ArrayLength,
    H: ArrayLength,
{
    let mut view = CustomizableView::new();
    let back_navigation = Some(PluginNavigation::<W, H>::new(HomeAssistantPlugin::<W, H> {
        menu: plugin.menu.clone(),
        back_navigation: plugin.back_navigation.clone(),
    }));
    for (index, item) in plugin.menu.buttons.iter().enumerate() {
        if index > W::to_usize() * H::to_usize() {
            break;
        }
        let x = index % W::to_usize();
        let y = index / W::to_usize();
        add_button(&mut view, x, y, item, &back_navigation)?;
    }

    if let Some(back_navigation) = &plugin.back_navigation {
        view.set_navigation(
            W::to_usize() - 1,
            H::to_usize() - 1,
            back_navigation.clone(),
            "Back",
            Some(md_icons::filled::ICON_MENU),
        )?;
    }

    Ok(view)
}

/// Implementation of the StreamDeck Plugin trait for HomeAssistantPlugin.
#[async_trait::async_trait]
impl<W, H> Plugin<W, H> for HomeAssistantPlugin<W, H>
where
    W: ArrayLength,
    H: ArrayLength,
{
    fn name(&self) -> &'static str {
        "HomeAssistantPlugin"
    }

    async fn get_view(
        &self,
        context: PluginContext,
    ) -> Result<
        Box<dyn View<W, H, PluginContext, PluginNavigation<W, H>>>,
        Box<dyn std::error::Error>,
    > {
        let _config = context
            .get_context::<HomeAssistantConfig>()
            .await
            .ok_or("Failed to get HomeAssistantConfig")?;
        Ok(Box::new(generate_menu(self)?))
    }
}
