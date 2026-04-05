use cargo_metadata::Metadata;
use std::sync::OnceLock;

#[allow(dead_code)]
pub fn cargo_metadata() -> &'static Metadata {
    static CACHE: OnceLock<Metadata> = OnceLock::new();
    CACHE.get_or_init(|| {
        ::cargo_metadata::MetadataCommand::new()
            .exec()
            .expect("cargo metadata failed")
    })
}
