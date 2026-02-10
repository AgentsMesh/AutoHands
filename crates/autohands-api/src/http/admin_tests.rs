
    use super::*;

    #[test]
    fn test_extension_info() {
        let info = ExtensionInfo {
            id: "test".to_string(),
            name: "Test Extension".to_string(),
            version: "1.0.0".to_string(),
            description: "A test extension".to_string(),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("test"));
    }

    #[test]
    fn test_error_response() {
        let err = ErrorResponse::new("Not found", "not_found");
        assert_eq!(err.error, "Not found");
        assert_eq!(err.code, "not_found");
    }
