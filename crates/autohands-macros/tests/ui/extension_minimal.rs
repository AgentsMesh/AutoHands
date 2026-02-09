use autohands_macros::extension;

#[extension(
    id = "minimal",
    name = "Minimal Extension"
)]
struct MinimalExtension;

fn main() {
    let ext = MinimalExtension;
    let manifest = ext.manifest();
    assert_eq!(manifest.id, "minimal");
    assert_eq!(manifest.name, "Minimal Extension");
}
