use crate::version::{Version, VersionRange};
use crate::PartialUnityVersion;
use indexmap::IndexMap;
use url::Url;

pub trait PackageJsonLike {
    fn name(&self) -> &str;
    fn version(&self) -> &Version;
    fn vpm_dependencies(&self) -> &IndexMap<Box<str>, VersionRange>;
    fn legacy_folders(&self) -> &std::collections::HashMap<Box<str>, Option<Box<str>>>;
    fn legacy_files(&self) -> &std::collections::HashMap<Box<str>, Option<Box<str>>>;
    fn legacy_packages(&self) -> &[Box<str>];
    fn display_name(&self) -> Option<&str>;
    fn description(&self) -> Option<&str>;
    fn url(&self) -> Option<&str>;
    fn zip_sha_256(&self) -> Option<&str>;
    fn changelog_url(&self) -> Option<&Url>;
    fn unity(&self) -> Option<&PartialUnityVersion>;
    fn is_yanked(&self) -> bool;
    fn aliases(&self) -> &[Box<str>];
}

macro_rules! impl_package_json_fn {
    (
        impl $type_name: ident;
        $($vis: vis fn $name: ident(&self) -> $ret: ty = |$v: pat_param| $expr: expr)*
    ) => {
        impl $type_name {
        $(
            #[inline]
            $vis fn $name(&self) -> $ret {
                let $v = self;
                $expr
            }
        )*
        }

        impl $crate::package_json::PackageJsonLike for $type_name {
        $(
            #[inline]
            fn $name(&self) -> $ret {
                self.$name()
            }
        )*
        }
    };
}

macro_rules! impl_package_json {
    (impl $name: ident = |$v: pat_param| $expr: expr) => {
        impl_package_json_fn! {
            impl $name;

            pub fn name(&self) -> &str = |$v| &$expr.name
            pub fn version(&self) -> &Version = |$v| &$expr.version
            pub fn vpm_dependencies(&self) -> &IndexMap<Box<str>, VersionRange> = |$v| &$expr.vpm_dependencies
            pub fn legacy_folders(&self) -> &std::collections::HashMap<Box<str>, Option<Box<str>>> = |$v| &$expr.legacy_folders
            pub fn legacy_files(&self) -> &std::collections::HashMap<Box<str>, Option<Box<str>>> = |$v| &$expr.legacy_files
            pub fn legacy_packages(&self) -> &[Box<str>] = |$v| $expr.legacy_packages.as_slice()
            pub fn display_name(&self) -> Option<&str> = |$v| $expr.display_name.as_deref()
            pub fn description(&self) -> Option<&str> = |$v| $expr.description.as_deref()
            pub fn url(&self) -> Option<&str> = |$v| $expr.url.as_deref()
            pub fn zip_sha_256(&self) -> Option<&str> = |$v| $expr.zip_sha_256.as_deref()
            pub fn changelog_url(&self) -> Option<&Url> = |$v| $expr.changelog_url.as_ref()
            pub fn unity(&self) -> Option<&PartialUnityVersion> = |$v| $expr.unity.as_ref()
            pub fn is_yanked(&self) -> bool = |$v| $expr.vrc_get.yanked.is_yanked()
            pub fn aliases(&self) -> &[Box<str>] = |$v| $expr.vrc_get.aliases.as_slice()
        }
    };
}

macro_rules! package_json_struct {
    {
        $(#[$meta:meta])*
        $vis:vis struct $name: ident {
            $optional_vis:vis optional$(: #[$optional: meta])?;
            $required_vis:vis required$(: #[$required: meta])?;
        }
    } => {
        #[derive(Deserialize, Debug, Clone)]
        #[serde(rename_all = "camelCase")]
        $(#[$meta])*
        $vis struct $name {
            $(#[$required])?
            $required_vis name: Box<str>,
            $(#[$required])?
            $required_vis version: Version,

            $(#[$optional])?
            $optional_vis display_name: Option<Box<str>>,
            $(#[$optional])?
            $optional_vis description: Option<Box<str>>,
            $(#[$optional])?
            $optional_vis unity: Option<crate::PartialUnityVersion>,

            $(#[$optional])?
            $optional_vis url: Option<String>,
            $(#[$optional])?
            #[serde(rename = "zipSHA256")]
            $optional_vis zip_sha_256: Option<Box<str>>,

            $(#[$optional])?
            $optional_vis vpm_dependencies: indexmap::IndexMap<Box<str>, VersionRange>,

            $(#[$optional])?
            $optional_vis legacy_folders: std::collections::HashMap<Box<str>, Option<Box<str>>>,
            $(#[$optional])?
            $optional_vis legacy_files: std::collections::HashMap<Box<str>, Option<Box<str>>>,
            $(#[$optional])?
            $optional_vis legacy_packages: Vec<Box<str>>,

            $(#[$optional])?
            $optional_vis changelog_url: Option<Url>,

            #[serde(rename = "vrc-get")]
            $(#[$optional])?
            $optional_vis vrc_get: VrcGetMeta,
        }

        // Note: please keep in sync with package_manifest
        #[derive(Deserialize, Debug, Clone, Default)]
        #[serde(rename_all = "camelCase")]
        pub(crate) struct VrcGetMeta {
            $(#[$optional])?
            pub(super) yanked: YankState,
            /// aliases for `vrc-get i --name <name> <version>` command.
            $(#[$optional])?
            pub(super) aliases: Vec<Box<str>>,
        }
    };
}
