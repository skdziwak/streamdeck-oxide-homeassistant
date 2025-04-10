//! RGB light controller plugin for HomeAssistant.
//!
//! This plugin provides a color picker interface for RGB lights in HomeAssistant.

use streamdeck_oxide::{
    generic_array::ArrayLength,
    md_icons,
    plugins::{Plugin, PluginContext, PluginNavigation},
    view::customizable::{ClickButton, CustomizableView, ToggleButton},
    View,
};

use crate::hass::PersistentHassConnection;

/// Plugin for controlling RGB lights in HomeAssistant.
///
/// This plugin displays a grid of color buttons and an on/off toggle
/// for controlling RGB lights.
#[derive(Clone)]
pub struct RgbControllerPlugin<W: ArrayLength, H: ArrayLength> {
    /// Optional navigation to return to when "Back" is pressed
    pub(crate) back_navigation: Option<PluginNavigation<W, H>>,
    /// The entity ID of the RGB light to control
    pub(crate) entity_id: String,
}

/// Predefined colors for the RGB controller
const COLORS: &[(&str, (u8, u8, u8))] = &[
    ("Red", (255, 0, 0)),
    ("Green", (0, 255, 0)),
    ("Blue", (0, 0, 255)),
    ("White", (255, 255, 255)),
    ("Warm White", (255, 200, 100)),
    ("Cool White", (200, 220, 255)),
    ("Purple", (128, 0, 128)),
    ("Yellow", (255, 255, 0)),
    ("Cyan", (0, 255, 255)),
    ("Orange", (255, 165, 0)),
    ("Pink", (255, 192, 203)),
    ("Teal", (0, 128, 128)),
];

/// Converts RGB color values to HSV (Hue, Saturation, Value) format.
///
/// # Arguments
///
/// * `r` - Red component (0-255)
/// * `g` - Green component (0-255)
/// * `b` - Blue component (0-255)
///
/// # Returns
///
/// A tuple of (hue, saturation, value) where:
/// * hue is in degrees (0-360)
/// * saturation is in percent (0-100)
/// * value is in percent (0-100)
fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r_f = r as f32 / 255.0;
    let g_f = g as f32 / 255.0;
    let b_f = b as f32 / 255.0;
    
    let max = r_f.max(g_f).max(b_f);
    let min = r_f.min(g_f).min(b_f);
    let delta = max - min;
    
    // Hue calculation
    let h = if delta == 0.0 {
        0.0
    } else if max == r_f {
        60.0 * (((g_f - b_f) / delta) % 6.0)
    } else if max == g_f {
        60.0 * (((b_f - r_f) / delta) + 2.0)
    } else {
        60.0 * (((r_f - g_f) / delta) + 4.0)
    };
    
    // Saturation calculation
    let s = if max == 0.0 { 0.0 } else { delta / max };
    
    // Value calculation
    let v = max;
    
    (h, s * 100.0, v * 100.0)
}

/// Implementation of the StreamDeck Plugin trait for RgbControllerPlugin.
#[async_trait::async_trait]
impl<W, H> Plugin<W, H> for RgbControllerPlugin<W, H>
where
    W: ArrayLength,
    H: ArrayLength,
{
    fn name(&self) -> &'static str {
        "RgbControllerPlugin"
    }

    async fn get_view(
        &self,
        _context: PluginContext,
    ) -> Result<
        Box<dyn View<W, H, PluginContext, PluginNavigation<W, H>>>,
        Box<dyn std::error::Error>,
    > {
        let mut view = CustomizableView::new();
        
        // Add the on/off toggle button at the top left
        let entity_id = self.entity_id.clone();
        let entity_id_2 = entity_id.clone();
        view.set_button(
            0,
            0,
            ToggleButton::new(
                "On/Off",
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
                            "light",
                            if value { "turn_on" } else { "turn_off" },
                            Some(serde_json::json!({ "entity_id": entity_id })),
                        )
                        .await
                        .map_err(|e| e.to_string())?;
                        Ok(())
                    }
                },
            )
            .when_active("On/Off", Some(md_icons::filled::ICON_TOGGLE_ON)),
        )?;
        
        // Add color buttons
        let max_buttons = W::to_usize() * H::to_usize() - 2; // Reserve space for on/off and back buttons
        let colors_to_show = std::cmp::min(COLORS.len(), max_buttons);
        
        for (index, &(color_name, (r, g, b))) in COLORS.iter().take(colors_to_show).enumerate() {
            let button_index = index + 1; // Skip the first button (on/off)
            let x = button_index % W::to_usize();
            let y = button_index / W::to_usize();
            
            let entity_id = self.entity_id.clone();
            let (h, s, v) = rgb_to_hsv(r, g, b);
            
            view.set_button(
                x,
                y,
                ClickButton::new(
                    color_name,
                    None, // No icon, will use color as background
                    move |ctx: PluginContext| {
                        let entity_id = entity_id.clone();
                        let h = h;
                        let s = s;
                        let v = v;
                        async move {
                            let hass = ctx
                                .get_context::<PersistentHassConnection>()
                                .await
                                .ok_or("Failed to get PersistentHassConnection")?;
                            
                            // Turn on the light with the selected color
                            hass.call_service(
                                "light",
                                "turn_on",
                                Some(serde_json::json!({
                                    "entity_id": entity_id,
                                    "hs_color": [h, s],
                                    "brightness_pct": v
                                })),
                            )
                            .await
                            .map_err(|e| e.to_string())?;
                            
                            Ok(())
                        }
                    },
                )
            )?;
        }
        
        // Add back button
        if let Some(back_navigation) = &self.back_navigation {
            view.set_navigation(
                W::to_usize() - 1,
                H::to_usize() - 1,
                back_navigation.clone(),
                "Back",
                Some(md_icons::filled::ICON_ARROW_BACK),
            )?;
        }
        
        Ok(Box::new(view))
    }
}
