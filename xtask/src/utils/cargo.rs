use cargo_metadata::Metadata;
use cargo_metadata::semver::Version;
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

pub fn gui_version() -> &'static Version {
    cargo_metadata()
        .packages
        .iter()
        .find(|p| p.name == "vrc-get-gui")
        .map(|p| &p.version)
        .expect("vrc-get-gui metadata not found")
}
