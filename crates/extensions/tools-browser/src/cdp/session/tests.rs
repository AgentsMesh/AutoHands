use super::core::PageSession;

#[test]
fn test_quad_center() {
    let quad = vec![0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0];
    let (x, y) = PageSession::quad_center(&quad);
    assert_eq!(x, 50.0);
    assert_eq!(y, 50.0);
}

#[test]
fn test_get_modifiers() {
    let modifiers = ["Control", "Shift"];
    let flags = PageSession::get_modifiers(&modifiers);
    assert_eq!(flags, 10); // 2 + 8
}

#[test]
fn test_get_modifiers_mac() {
    let modifiers = ["Meta", "a"];
    // Only Meta should be counted, 'a' is not a modifier
    let flags = PageSession::get_modifiers(&modifiers[..1]);
    assert_eq!(flags, 4);
}
