use super::*;

#[test]
fn test_parse_coordinates_json() {
    let response = r#"{"x": 100, "y": 200, "confidence": 0.95}"#;
    let coords = parse_coordinates(response).unwrap();
    assert_eq!(coords.x, 100);
    assert_eq!(coords.y, 200);
    assert!((coords.confidence - 0.95).abs() < 0.01);
}

#[test]
fn test_parse_coordinates_text_pattern() {
    let response = "The button is located at x: 150, y: 300";
    let coords = parse_coordinates(response).unwrap();
    assert_eq!(coords.x, 150);
    assert_eq!(coords.y, 300);
}

#[test]
fn test_parse_coordinates_tuple_pattern() {
    let response = "Found element at (250, 400)";
    let coords = parse_coordinates(response).unwrap();
    assert_eq!(coords.x, 250);
    assert_eq!(coords.y, 400);
}

#[test]
fn test_parse_coordinates_simple_numbers() {
    let response = "Click at 300, 500";
    let coords = parse_coordinates(response).unwrap();
    assert_eq!(coords.x, 300);
    assert_eq!(coords.y, 500);
}

#[test]
fn test_parse_coordinates_invalid() {
    let response = "I cannot find the element";
    let result = parse_coordinates(response);
    assert!(result.is_err());
}

#[test]
fn test_element_coordinates_serialize() {
    let coords = ElementCoordinates {
        x: 100,
        y: 200,
        width: Some(50),
        height: Some(30),
        confidence: 0.9,
    };
    let json = serde_json::to_string(&coords).unwrap();
    assert!(json.contains("100"));
    assert!(json.contains("200"));
}

#[test]
fn test_ai_click_params_deserialize() {
    let json = r#"{"page_id": "page_1", "target": "login button"}"#;
    let params: AiClickParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.page_id, "page_1");
    assert_eq!(params.target, "login button");
}

#[test]
fn test_ai_fill_params_deserialize() {
    let json = r#"{"page_id": "page_1", "field": "email input", "value": "test@example.com"}"#;
    let params: AiFillParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.page_id, "page_1");
    assert_eq!(params.field, "email input");
    assert_eq!(params.value, "test@example.com");
    assert!(params._clear_first); // default
}

#[test]
fn test_ai_fill_params_with_clear() {
    let json = r#"{"page_id": "page_1", "field": "name", "value": "John", "clear_first": false}"#;
    let params: AiFillParams = serde_json::from_str(json).unwrap();
    assert!(!params._clear_first);
}

#[test]
fn test_ai_extract_params_deserialize() {
    let json = r#"{"page_id": "page_1", "query": "product prices"}"#;
    let params: AiExtractParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.page_id, "page_1");
    assert_eq!(params.query, "product prices");
    assert_eq!(params.format, "json"); // default
}

#[test]
fn test_ai_extract_params_with_format() {
    let json = r#"{"page_id": "page_1", "query": "headlines", "format": "list"}"#;
    let params: AiExtractParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.format, "list");
}
