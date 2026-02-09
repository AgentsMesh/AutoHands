use autohands_macros::extension;

#[extension(
    id = "test-extension",
    name = "Test Extension",
    version = "1.2.3",
    description = "A test extension"
)]
struct TestExtension {
    value: i32,
}

impl TestExtension {
    pub fn new(value: i32) -> Self {
        Self { value }
    }
}

fn main() {
    let ext = TestExtension::new(42);
    let manifest = ext.manifest();
    assert_eq!(manifest.id, "test-extension");
    assert_eq!(manifest.name, "Test Extension");
    assert_eq!(manifest.description, "A test extension");
}
