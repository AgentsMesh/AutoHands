use super::*;
use std::collections::HashMap;

#[test]
fn test_bounding_box_contains() {
    let bbox = BoundingBox {
        x: 10.0, y: 20.0, width: 100.0, height: 50.0,
    };
    assert!(bbox.contains(50.0, 40.0));
    assert!(!bbox.contains(0.0, 0.0));
    assert!(!bbox.contains(200.0, 40.0));
}

#[test]
fn test_bounding_box_center() {
    let bbox = BoundingBox {
        x: 0.0, y: 0.0, width: 100.0, height: 100.0,
    };
    assert_eq!(bbox.center(), (50.0, 50.0));
}

#[test]
fn test_bounding_box_intersects() {
    let box1 = BoundingBox { x: 0.0, y: 0.0, width: 100.0, height: 100.0 };
    let box2 = BoundingBox { x: 50.0, y: 50.0, width: 100.0, height: 100.0 };
    let box3 = BoundingBox { x: 200.0, y: 200.0, width: 100.0, height: 100.0 };
    assert!(box1.intersects(&box2));
    assert!(!box1.intersects(&box3));
}

#[test]
fn test_clickability_score_button() {
    let (score, reasons) = DomProcessor::calculate_clickability_score(
        "button",
        &NodeAttributes::default(),
        &HashMap::new(),
        true,
        &HashMap::new(),
    );
    assert!(score > 0.4);
    assert!(reasons.contains(&"native_tag:button".to_string()));
    assert!(reasons.contains(&"has_event_listener".to_string()));
}

#[test]
fn test_clickability_score_link() {
    let mut attrs = NodeAttributes::default();
    attrs.href = Some("https://example.com".to_string());

    let (score, reasons) = DomProcessor::calculate_clickability_score(
        "a",
        &attrs,
        &HashMap::new(),
        false,
        &HashMap::new(),
    );
    assert!(score > 0.4);
    assert!(reasons.contains(&"native_tag:a".to_string()));
    assert!(reasons.contains(&"has_href".to_string()));
}

#[test]
fn test_clickability_score_cursor_pointer() {
    let mut styles = HashMap::new();
    styles.insert("cursor".to_string(), "pointer".to_string());

    let (score, reasons) = DomProcessor::calculate_clickability_score(
        "div",
        &NodeAttributes::default(),
        &styles,
        false,
        &HashMap::new(),
    );
    assert!(score > 0.1);
    assert!(reasons.contains(&"cursor_pointer".to_string()));
}

#[test]
fn test_node_to_llm_string() {
    let mut attrs = NodeAttributes::default();
    attrs.id = Some("login-btn".to_string());
    attrs.aria_label = Some("Login".to_string());

    let node = EnhancedNode {
        id: "node_1".to_string(),
        backend_node_id: 1,
        tag_name: "button".to_string(),
        attributes: attrs,
        text_content: "Sign In".to_string(),
        bounding_box: BoundingBox::default(),
        is_visible: true,
        is_in_viewport: true,
        clickability_score: 0.9,
        clickability_reasons: vec!["native_tag:button".to_string()],
        paint_order: 1,
        is_interactive: true,
        is_focusable: true,
        parent_id: None,
        children: vec![],
        xpath: "/html/body/button".to_string(),
        css_selector: "#login-btn".to_string(),
        computed_styles: HashMap::new(),
    };

    let output = node.to_llm_string(0);
    assert!(output.contains("[0]"));
    assert!(output.contains("<button>"));
    assert!(output.contains("Sign In"));
    assert!(output.contains("id=login-btn"));
}

#[test]
fn test_viewport_default() {
    let viewport = ViewportInfo::default();
    assert_eq!(viewport.width, 1280);
    assert_eq!(viewport.height, 720);
    assert_eq!(viewport.device_pixel_ratio, 1.0);
}
