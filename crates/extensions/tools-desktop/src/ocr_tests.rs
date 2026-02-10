use super::*;

#[test]
fn test_ocr_error_display() {
    let err = OcrError::RecognitionFailed("test error".to_string());
    assert!(err.to_string().contains("test error"));
}

#[test]
fn test_ocr_result_serialize() {
    let result = OcrResult {
        text: "Hello World".to_string(),
        confidence: 0.95,
        blocks: vec![TextBlock {
            text: "Hello".to_string(),
            x: 10,
            y: 20,
            width: 50,
            height: 20,
            confidence: 0.95,
        }],
    };

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("Hello World"));
    assert!(json.contains("0.95"));
}

#[test]
fn test_text_block_serialize() {
    let block = TextBlock {
        text: "Test".to_string(),
        x: 100,
        y: 200,
        width: 50,
        height: 20,
        confidence: 0.9,
    };

    let json = serde_json::to_string(&block).unwrap();
    assert!(json.contains("Test"));
    assert!(json.contains("100"));
}

#[test]
fn test_ocr_controller_new() {
    let controller = OcrController::new();
    assert!(controller.is_ok());
}

#[test]
fn test_ocr_controller_default() {
    let _controller = OcrController::default();
}
