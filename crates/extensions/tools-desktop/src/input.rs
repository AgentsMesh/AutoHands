//! Mouse and keyboard input control.

use std::thread;
use std::time::Duration;

use enigo::{
    Axis, Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings,
};
use thiserror::Error;

/// Input control errors.
#[derive(Debug, Error)]
pub enum InputError {
    #[error("Input failed: {0}")]
    Failed(String),

    #[error("Invalid key: {0}")]
    InvalidKey(String),
}

/// Mouse button types.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl From<MouseButton> for Button {
    fn from(btn: MouseButton) -> Self {
        match btn {
            MouseButton::Left => Button::Left,
            MouseButton::Right => Button::Right,
            MouseButton::Middle => Button::Middle,
        }
    }
}

/// Input controller for mouse and keyboard.
pub struct InputController {
    enigo: Enigo,
}

impl InputController {
    /// Create a new input controller.
    pub fn new() -> Result<Self, InputError> {
        let enigo = Enigo::new(&Settings::default())
            .map_err(|e| InputError::Failed(e.to_string()))?;
        Ok(Self { enigo })
    }

    // ========================================================================
    // Mouse operations
    // ========================================================================

    /// Move mouse to absolute position.
    pub fn mouse_move(&mut self, x: i32, y: i32) -> Result<(), InputError> {
        self.enigo
            .move_mouse(x, y, Coordinate::Abs)
            .map_err(|e| InputError::Failed(e.to_string()))
    }

    /// Move mouse relative to current position.
    pub fn mouse_move_relative(&mut self, dx: i32, dy: i32) -> Result<(), InputError> {
        self.enigo
            .move_mouse(dx, dy, Coordinate::Rel)
            .map_err(|e| InputError::Failed(e.to_string()))
    }

    /// Click mouse button.
    pub fn mouse_click(&mut self, button: MouseButton) -> Result<(), InputError> {
        self.enigo
            .button(button.into(), Direction::Click)
            .map_err(|e| InputError::Failed(e.to_string()))
    }

    /// Double click mouse button.
    pub fn mouse_double_click(&mut self, button: MouseButton) -> Result<(), InputError> {
        self.mouse_click(button)?;
        thread::sleep(Duration::from_millis(50));
        self.mouse_click(button)
    }

    /// Press and hold mouse button.
    pub fn mouse_down(&mut self, button: MouseButton) -> Result<(), InputError> {
        self.enigo
            .button(button.into(), Direction::Press)
            .map_err(|e| InputError::Failed(e.to_string()))
    }

    /// Release mouse button.
    pub fn mouse_up(&mut self, button: MouseButton) -> Result<(), InputError> {
        self.enigo
            .button(button.into(), Direction::Release)
            .map_err(|e| InputError::Failed(e.to_string()))
    }

    /// Scroll mouse wheel.
    pub fn mouse_scroll(&mut self, delta: i32, horizontal: bool) -> Result<(), InputError> {
        let axis = if horizontal { Axis::Horizontal } else { Axis::Vertical };
        self.enigo
            .scroll(delta, axis)
            .map_err(|e| InputError::Failed(e.to_string()))
    }

    /// Drag from current position to target.
    pub fn mouse_drag(&mut self, x: i32, y: i32, button: MouseButton) -> Result<(), InputError> {
        self.mouse_down(button)?;
        thread::sleep(Duration::from_millis(50));
        self.mouse_move(x, y)?;
        thread::sleep(Duration::from_millis(50));
        self.mouse_up(button)
    }

    // ========================================================================
    // Keyboard operations
    // ========================================================================

    /// Type a string of text.
    pub fn type_text(&mut self, text: &str) -> Result<(), InputError> {
        self.enigo
            .text(text)
            .map_err(|e| InputError::Failed(e.to_string()))
    }

    /// Press a single key.
    pub fn key_press(&mut self, key: &str) -> Result<(), InputError> {
        let k = parse_key(key)?;
        self.enigo
            .key(k, Direction::Click)
            .map_err(|e| InputError::Failed(e.to_string()))
    }

    /// Hold a key down.
    pub fn key_down(&mut self, key: &str) -> Result<(), InputError> {
        let k = parse_key(key)?;
        self.enigo
            .key(k, Direction::Press)
            .map_err(|e| InputError::Failed(e.to_string()))
    }

    /// Release a key.
    pub fn key_up(&mut self, key: &str) -> Result<(), InputError> {
        let k = parse_key(key)?;
        self.enigo
            .key(k, Direction::Release)
            .map_err(|e| InputError::Failed(e.to_string()))
    }

    /// Press a key combination (e.g., Ctrl+C).
    pub fn hotkey(&mut self, keys: &[&str]) -> Result<(), InputError> {
        // Press all modifier keys
        for key in keys.iter().take(keys.len().saturating_sub(1)) {
            self.key_down(key)?;
        }

        // Press and release the final key
        if let Some(last) = keys.last() {
            self.key_press(last)?;
        }

        // Release modifier keys in reverse order
        for key in keys.iter().rev().skip(1) {
            self.key_up(key)?;
        }

        Ok(())
    }
}

impl Default for InputController {
    fn default() -> Self {
        Self::new().expect("Failed to create input controller")
    }
}

/// Parse a key string to enigo Key.
fn parse_key(key: &str) -> Result<Key, InputError> {
    let k = match key.to_lowercase().as_str() {
        // Letters
        "a" => Key::Unicode('a'),
        "b" => Key::Unicode('b'),
        "c" => Key::Unicode('c'),
        "d" => Key::Unicode('d'),
        "e" => Key::Unicode('e'),
        "f" => Key::Unicode('f'),
        "g" => Key::Unicode('g'),
        "h" => Key::Unicode('h'),
        "i" => Key::Unicode('i'),
        "j" => Key::Unicode('j'),
        "k" => Key::Unicode('k'),
        "l" => Key::Unicode('l'),
        "m" => Key::Unicode('m'),
        "n" => Key::Unicode('n'),
        "o" => Key::Unicode('o'),
        "p" => Key::Unicode('p'),
        "q" => Key::Unicode('q'),
        "r" => Key::Unicode('r'),
        "s" => Key::Unicode('s'),
        "t" => Key::Unicode('t'),
        "u" => Key::Unicode('u'),
        "v" => Key::Unicode('v'),
        "w" => Key::Unicode('w'),
        "x" => Key::Unicode('x'),
        "y" => Key::Unicode('y'),
        "z" => Key::Unicode('z'),

        // Numbers
        "0" => Key::Unicode('0'),
        "1" => Key::Unicode('1'),
        "2" => Key::Unicode('2'),
        "3" => Key::Unicode('3'),
        "4" => Key::Unicode('4'),
        "5" => Key::Unicode('5'),
        "6" => Key::Unicode('6'),
        "7" => Key::Unicode('7'),
        "8" => Key::Unicode('8'),
        "9" => Key::Unicode('9'),

        // Special keys
        "enter" | "return" => Key::Return,
        "tab" => Key::Tab,
        "space" => Key::Space,
        "backspace" => Key::Backspace,
        "delete" | "del" => Key::Delete,
        "escape" | "esc" => Key::Escape,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        "up" => Key::UpArrow,
        "down" => Key::DownArrow,
        "left" => Key::LeftArrow,
        "right" => Key::RightArrow,

        // Modifiers
        "ctrl" | "control" => Key::Control,
        "alt" => Key::Alt,
        "shift" => Key::Shift,
        "meta" | "cmd" | "command" | "win" | "super" => Key::Meta,

        // Function keys
        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,

        // Single character
        s if s.len() == 1 => Key::Unicode(s.chars().next().unwrap()),

        _ => return Err(InputError::InvalidKey(key.to_string())),
    };

    Ok(k)
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;
