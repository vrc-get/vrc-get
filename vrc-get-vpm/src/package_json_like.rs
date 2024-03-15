use crate::package_manifest::PartialUnityVersion;
use crate::version::{Version, VersionRange};
use indexmap::IndexMap;
use url::Url;

pub trait PackageJsonLike {
    fn name(&self) -> &str;
    fn version(&self) -> &Version;
    fn vpm_dependencies(&self) -> &IndexMap<Box<str>, VersionRange>;
    fn legacy_packages(&self) -> &[Box<str>];
    fn display_name(&self) -> Option<&str>;
    fn description(&self) -> Option<&str>;
    fn changelog_url(&self) -> Option<&Url>;
    fn unity(&self) -> Option<&PartialUnityVersion>;
    fn is_yanked(&self) -> bool;
    fn aliases(&self) -> &[Box<str>];
}

macro_rules! impl_package_json_like {
    ($t: ty) => {
        impl $crate::package_json_like::PackageJsonLike for $t {
            fn name(&self) -> &str {
                self.name()
            }

            fn version(&self) -> &Version {
                self.version()
            }

            fn vpm_dependencies(&self) -> &IndexMap<Box<str>, VersionRange> {
                self.vpm_dependencies()
            }

            fn legacy_packages(&self) -> &[Box<str>] {
                self.legacy_packages()
            }

            fn display_name(&self) -> Option<&str> {
                self.display_name()
            }

            fn description(&self) -> Option<&str> {
                self.description()
            }

            fn changelog_url(&self) -> Option<&Url> {
                self.changelog_url()
            }

            fn unity(&self) -> Option<&PartialUnityVersion> {
                self.unity()
            }

            fn is_yanked(&self) -> bool {
                self.is_yanked()
            }

            fn aliases(&self) -> &[Box<str>] {
                self.aliases()
            }
        }
    };
}
