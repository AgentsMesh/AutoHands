use autohands_macros::extension;

// Test extension with struct fields
#[extension(
    id = "fields-ext",
    name = "Fields Extension",
    version = "2.0.0",
    description = "Extension with fields"
)]
struct FieldsExtension {
    counter: u32,
    name: String,
}

impl FieldsExtension {
    pub fn new(counter: u32, name: String) -> Self {
        Self { counter, name }
    }
}

fn main() {
    let ext = FieldsExtension::new(42, "test".to_string());
    let manifest = ext.manifest();
    assert_eq!(manifest.id, "fields-ext");
    assert_eq!(manifest.name, "Fields Extension");
    assert_eq!(manifest.description, "Extension with fields");

    // Verify struct fields still work
    assert_eq!(ext.counter, 42);
    assert_eq!(ext.name, "test");
}
