use super::*;

#[test]
fn test_request_id_increment() {
    let id = AtomicU64::new(1);
    assert_eq!(id.fetch_add(1, Ordering::SeqCst), 1);
    assert_eq!(id.fetch_add(1, Ordering::SeqCst), 2);
    assert_eq!(id.load(Ordering::SeqCst), 3);
}
