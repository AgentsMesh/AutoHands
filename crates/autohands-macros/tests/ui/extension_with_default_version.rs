use autohands_macros::extension;

// Test extension without explicit version (should use default 0.1.0)
#[extension(
    id = "default-version-ext",
    name = "Default Version Extension"
)]
struct DefaultVersionExtension;

fn main() {
    let ext = DefaultVersionExtension;
    let manifest = ext.manifest();
    assert_eq!(manifest.id, "default-version-ext");
    assert_eq!(manifest.name, "Default Version Extension");
    // Description should be empty by default
    assert!(manifest.description.is_empty());
}
