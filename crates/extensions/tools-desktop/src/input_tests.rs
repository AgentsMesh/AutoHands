use super::*;

#[test]
fn test_parse_key_letters() {
    assert!(parse_key("a").is_ok());
    assert!(parse_key("Z").is_ok());
    assert!(parse_key("m").is_ok());
}

#[test]
fn test_parse_key_all_letters() {
    for c in 'a'..='z' {
        assert!(parse_key(&c.to_string()).is_ok(), "Failed for letter: {}", c);
    }
}

#[test]
fn test_parse_key_numbers() {
    for n in 0..=9 {
        assert!(parse_key(&n.to_string()).is_ok(), "Failed for number: {}", n);
    }
}

#[test]
fn test_parse_key_special() {
    assert!(parse_key("enter").is_ok());
    assert!(parse_key("return").is_ok());
    assert!(parse_key("tab").is_ok());
    assert!(parse_key("space").is_ok());
    assert!(parse_key("backspace").is_ok());
    assert!(parse_key("delete").is_ok());
    assert!(parse_key("del").is_ok());
    assert!(parse_key("escape").is_ok());
    assert!(parse_key("esc").is_ok());
    assert!(parse_key("home").is_ok());
    assert!(parse_key("end").is_ok());
    assert!(parse_key("pageup").is_ok());
    assert!(parse_key("pagedown").is_ok());
}

#[test]
fn test_parse_key_arrows() {
    assert!(parse_key("up").is_ok());
    assert!(parse_key("down").is_ok());
    assert!(parse_key("left").is_ok());
    assert!(parse_key("right").is_ok());
}

#[test]
fn test_parse_key_modifiers() {
    assert!(parse_key("ctrl").is_ok());
    assert!(parse_key("control").is_ok());
    assert!(parse_key("alt").is_ok());
    assert!(parse_key("shift").is_ok());
    assert!(parse_key("meta").is_ok());
    assert!(parse_key("cmd").is_ok());
    assert!(parse_key("command").is_ok());
    assert!(parse_key("win").is_ok());
    assert!(parse_key("super").is_ok());
}

#[test]
fn test_parse_key_function_keys() {
    for n in 1..=12 {
        let key = format!("f{}", n);
        assert!(parse_key(&key).is_ok(), "Failed for function key: {}", key);
    }
}

#[test]
fn test_parse_key_invalid() {
    assert!(parse_key("invalid_key_name").is_err());
    assert!(parse_key("nonexistent").is_err());
}

#[test]
fn test_parse_key_case_insensitive() {
    assert!(parse_key("ENTER").is_ok());
    assert!(parse_key("Enter").is_ok());
    assert!(parse_key("CTRL").is_ok());
    assert!(parse_key("Shift").is_ok());
}

#[test]
fn test_parse_key_single_char() {
    assert!(parse_key("+").is_ok());
    assert!(parse_key("-").is_ok());
    assert!(parse_key("=").is_ok());
}

#[test]
fn test_mouse_button_conversion() {
    let _: Button = MouseButton::Left.into();
    let _: Button = MouseButton::Right.into();
    let _: Button = MouseButton::Middle.into();
}

#[test]
fn test_mouse_button_clone() {
    let btn = MouseButton::Left;
    let cloned = btn.clone();
    assert!(matches!(cloned, MouseButton::Left));
}

#[test]
fn test_mouse_button_copy() {
    let btn = MouseButton::Right;
    let copied = btn;
    assert!(matches!(copied, MouseButton::Right));
    assert!(matches!(btn, MouseButton::Right));
}

#[test]
fn test_mouse_button_debug() {
    let btn = MouseButton::Middle;
    let debug_str = format!("{:?}", btn);
    assert!(debug_str.contains("Middle"));
}

#[test]
fn test_mouse_button_deserialize() {
    let left: MouseButton = serde_json::from_str(r#""left""#).unwrap();
    assert!(matches!(left, MouseButton::Left));

    let right: MouseButton = serde_json::from_str(r#""right""#).unwrap();
    assert!(matches!(right, MouseButton::Right));

    let middle: MouseButton = serde_json::from_str(r#""middle""#).unwrap();
    assert!(matches!(middle, MouseButton::Middle));
}

#[test]
fn test_input_error_display() {
    let err = InputError::Failed("operation failed".to_string());
    assert_eq!(err.to_string(), "Input failed: operation failed");

    let err = InputError::InvalidKey("xyz".to_string());
    assert_eq!(err.to_string(), "Invalid key: xyz");
}

#[test]
fn test_input_error_debug() {
    let err = InputError::Failed("test".to_string());
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("Failed"));
}

// Integration tests that require actual input control
#[test]
#[ignore] // Requires actual input control
fn test_input_controller_new() {
    let controller = InputController::new();
    assert!(controller.is_ok());
}

#[test]
#[ignore] // Requires actual input control
fn test_input_controller_default() {
    let _controller = InputController::default();
}
