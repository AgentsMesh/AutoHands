use super::*;
use autohands_protocols::Tool;
use image::ImageFormat;

#[test]
fn test_parse_format() {
    assert!(matches!(parse_format("png").unwrap(), ImageFormat::Png));
    assert!(matches!(parse_format("jpg").unwrap(), ImageFormat::Jpeg));
    assert!(matches!(parse_format("jpeg").unwrap(), ImageFormat::Jpeg));
    assert!(matches!(parse_format("gif").unwrap(), ImageFormat::Gif));
    assert!(matches!(parse_format("webp").unwrap(), ImageFormat::WebP));
    assert!(parse_format("invalid").is_err());
}

#[test]
fn test_resize_params_deserialize() {
    let json = r#"{"input": "test.png", "width": 100}"#;
    let params: ImageResizeParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.input, "test.png");
    assert_eq!(params.width, Some(100));
    assert!(params.preserve_aspect); // default
}

#[test]
fn test_crop_params_deserialize() {
    let json = r#"{"input": "test.png", "x": 10, "y": 20, "width": 100, "height": 50}"#;
    let params: ImageCropParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.x, 10);
    assert_eq!(params.y, 20);
    assert_eq!(params.width, 100);
    assert_eq!(params.height, 50);
}

#[test]
fn test_convert_params_deserialize() {
    let json = r#"{"input": "test.png", "output": "test.jpg", "format": "jpeg"}"#;
    let params: ImageConvertParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.format, Some("jpeg".to_string()));
}

#[test]
fn test_info_params_deserialize() {
    let json = r#"{"path": "test.png"}"#;
    let params: ImageInfoParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.path, "test.png");
}

#[test]
fn test_rotate_params_deserialize() {
    let json = r#"{"input": "test.png", "degrees": 90}"#;
    let params: ImageRotateParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.degrees, 90);
}

#[test]
fn test_flip_params_deserialize() {
    let json = r#"{"input": "test.png", "direction": "horizontal"}"#;
    let params: ImageFlipParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.direction, "horizontal");
}

#[test]
fn test_tool_definitions() {
    let resize = ImageResizeTool::new();
    assert_eq!(resize.definition().id, "image_resize");

    let crop = ImageCropTool::new();
    assert_eq!(crop.definition().id, "image_crop");

    let convert = ImageConvertTool::new();
    assert_eq!(convert.definition().id, "image_convert");

    let info = ImageInfoTool::new();
    assert_eq!(info.definition().id, "image_info");

    let rotate = ImageRotateTool::new();
    assert_eq!(rotate.definition().id, "image_rotate");

    let flip = ImageFlipTool::new();
    assert_eq!(flip.definition().id, "image_flip");
}
