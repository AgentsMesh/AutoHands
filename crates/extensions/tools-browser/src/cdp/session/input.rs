//! Input (mouse and keyboard) operations for CDP page session.

use serde_json::json;
use tracing::debug;

use crate::cdp::error::CdpError;
use crate::cdp::protocol::{KeyEventType, MouseButton, MouseEventType};

use super::core::PageSession;

impl PageSession {
    /// Click at coordinates.
    pub async fn click(&self, x: f64, y: f64) -> Result<(), CdpError> {
        self.call(
            "Input.dispatchMouseEvent",
            Some(json!({
                "type": MouseEventType::MousePressed,
                "x": x,
                "y": y,
                "button": MouseButton::Left,
                "clickCount": 1,
            })),
        )
        .await?;

        self.call(
            "Input.dispatchMouseEvent",
            Some(json!({
                "type": MouseEventType::MouseReleased,
                "x": x,
                "y": y,
                "button": MouseButton::Left,
                "clickCount": 1,
            })),
        )
        .await?;

        debug!("Clicked at ({}, {})", x, y);
        Ok(())
    }

    /// Double click at coordinates.
    pub async fn double_click(&self, x: f64, y: f64) -> Result<(), CdpError> {
        for click_count in [1, 2] {
            self.call(
                "Input.dispatchMouseEvent",
                Some(json!({
                    "type": MouseEventType::MousePressed,
                    "x": x,
                    "y": y,
                    "button": MouseButton::Left,
                    "clickCount": click_count,
                })),
            )
            .await?;

            self.call(
                "Input.dispatchMouseEvent",
                Some(json!({
                    "type": MouseEventType::MouseReleased,
                    "x": x,
                    "y": y,
                    "button": MouseButton::Left,
                    "clickCount": click_count,
                })),
            )
            .await?;
        }
        Ok(())
    }

    /// Move mouse to coordinates.
    pub async fn mouse_move(&self, x: f64, y: f64) -> Result<(), CdpError> {
        self.call(
            "Input.dispatchMouseEvent",
            Some(json!({
                "type": MouseEventType::MouseMoved,
                "x": x,
                "y": y,
            })),
        )
        .await?;
        Ok(())
    }

    /// Scroll by delta.
    pub async fn scroll(&self, x: f64, y: f64, delta_x: f64, delta_y: f64) -> Result<(), CdpError> {
        self.call(
            "Input.dispatchMouseEvent",
            Some(json!({
                "type": MouseEventType::MouseWheel,
                "x": x,
                "y": y,
                "deltaX": delta_x,
                "deltaY": delta_y,
            })),
        )
        .await?;
        Ok(())
    }

    /// Type text.
    pub async fn type_text(&self, text: &str) -> Result<(), CdpError> {
        self.call("Input.insertText", Some(json!({"text": text})))
            .await?;
        debug!("Typed {} characters", text.len());
        Ok(())
    }

    /// Press a key.
    pub async fn press_key(&self, key: &str) -> Result<(), CdpError> {
        self.call(
            "Input.dispatchKeyEvent",
            Some(json!({
                "type": KeyEventType::KeyDown,
                "key": key,
            })),
        )
        .await?;

        self.call(
            "Input.dispatchKeyEvent",
            Some(json!({
                "type": KeyEventType::KeyUp,
                "key": key,
            })),
        )
        .await?;

        Ok(())
    }

    /// Press key combination (e.g., "Control+a").
    pub async fn press_key_combo(&self, combo: &str) -> Result<(), CdpError> {
        let parts: Vec<&str> = combo.split('+').collect();
        let modifiers = Self::get_modifiers(&parts[..parts.len() - 1]);
        let key = parts.last().unwrap_or(&"");

        self.call(
            "Input.dispatchKeyEvent",
            Some(json!({
                "type": KeyEventType::KeyDown,
                "key": key,
                "modifiers": modifiers,
            })),
        )
        .await?;

        self.call(
            "Input.dispatchKeyEvent",
            Some(json!({
                "type": KeyEventType::KeyUp,
                "key": key,
                "modifiers": modifiers,
            })),
        )
        .await?;

        Ok(())
    }

    /// Get modifier flags from modifier names.
    pub(super) fn get_modifiers(modifiers: &[&str]) -> i32 {
        let mut flags = 0;
        for m in modifiers {
            match m.to_lowercase().as_str() {
                "alt" => flags |= 1,
                "control" | "ctrl" => flags |= 2,
                "meta" | "command" | "cmd" => flags |= 4,
                "shift" => flags |= 8,
                _ => {}
            }
        }
        flags
    }
}
