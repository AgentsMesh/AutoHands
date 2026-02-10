use super::*;

#[test]
fn test_ocr_screen_tool_definition() {
    let tool = OcrScreenTool::new();
    assert_eq!(tool.definition().id, "desktop_ocr_screen");
}

#[test]
fn test_ocr_region_tool_definition() {
    let tool = OcrRegionTool::new();
    assert_eq!(tool.definition().id, "desktop_ocr_region");
}

#[test]
fn test_ocr_image_tool_definition() {
    let tool = OcrImageTool::new();
    assert_eq!(tool.definition().id, "desktop_ocr_image");
}

#[test]
fn test_ocr_region_params() {
    let json = serde_json::json!({
        "x": 100,
        "y": 200,
        "width": 300,
        "height": 400
    });
    let params: OcrRegionParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.x, 100);
    assert_eq!(params.y, 200);
    assert_eq!(params.width, 300);
    assert_eq!(params.height, 400);
}

#[test]
fn test_ocr_image_params() {
    let json = serde_json::json!({
        "image_base64": "aGVsbG8="
    });
    let params: OcrImageParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.image_base64, "aGVsbG8=");
}

#[test]
fn test_tools_default_impl() {
    let _ = OcrScreenTool::default();
    let _ = OcrRegionTool::default();
    let _ = OcrImageTool::default();
}
