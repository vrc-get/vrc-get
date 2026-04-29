use crate::version::{PrereleaseAcceptance, UnityVersion, Version, VersionRange};
use crate::{PackageManifest, unity_compatible};

#[derive(Clone, Copy)]
pub struct VersionSelector<'a> {
    inner: SelectorInner<'a>,
}

#[derive(Clone, Copy)]
enum SelectorInner<'a> {
    Specific(&'a Version),
    Latest {
        project_unity: Option<UnityVersion>,
        include_prerelease: bool,
    },
    Range {
        project_unity: Option<UnityVersion>,
        range: &'a VersionRange,
        allow_prerelease: PrereleaseAcceptance,
    },
    Ranges {
        project_unity: Option<UnityVersion>,
        ranges: &'a [&'a VersionRange],
        allow_prerelease: PrereleaseAcceptance,
    },
}

impl<'a> VersionSelector<'a> {
    pub fn specific_version(version: &'a Version) -> Self {
        Self {
            inner: SelectorInner::Specific(version),
        }
    }

    pub fn latest_for(unity_version: Option<UnityVersion>, include_prerelease: bool) -> Self {
        Self {
            inner: SelectorInner::Latest {
                project_unity: unity_version,
                include_prerelease,
            },
        }
    }

    pub fn range_for(
        unity_version: Option<UnityVersion>,
        range: &'a VersionRange,
        allow_prerelease: PrereleaseAcceptance,
    ) -> Self {
        Self {
            inner: SelectorInner::Range {
                project_unity: unity_version,
                range,
                allow_prerelease,
            },
        }
    }

    pub fn ranges_for(
        unity_version: Option<UnityVersion>,
        ranges: &'a [&'a VersionRange],
        allow_prerelease: PrereleaseAcceptance,
    ) -> Self {
        Self {
            inner: SelectorInner::Ranges {
                project_unity: unity_version,
                ranges,
                allow_prerelease,
            },
        }
    }
}

impl VersionSelector<'_> {
    pub(crate) fn as_specific(&self) -> Option<&Version> {
        match self.inner {
            SelectorInner::Specific(version) => Some(version),
            _ => None,
        }
    }
}

impl VersionSelector<'_> {
    pub fn satisfies(&self, package: &PackageManifest) -> bool {
        fn unity_and_yank(package: &PackageManifest, project_unity: Option<UnityVersion>) -> bool {
            if package.is_yanked() {
                return false;
            }

            if let Some(unity) = project_unity
                && !unity_compatible(package, unity)
            {
                return false;
            }

            true
        }

        match self.inner {
            SelectorInner::Specific(finding) => finding == package.version(),
            SelectorInner::Latest {
                include_prerelease: true,
                project_unity,
            } => unity_and_yank(package, project_unity),
            SelectorInner::Latest {
                include_prerelease: false,
                project_unity,
            } => package.version().is_stable() && unity_and_yank(package, project_unity),
            SelectorInner::Range {
                range,
                project_unity,
                allow_prerelease,
            } => {
                range.match_pre(package.version(), allow_prerelease)
                    && unity_and_yank(package, project_unity)
            }
            SelectorInner::Ranges {
                ranges,
                project_unity,
                allow_prerelease,
            } => {
                ranges
                    .iter()
                    .all(|x| x.match_pre(package.version(), allow_prerelease))
                    && unity_and_yank(package, project_unity)
            }
        }
    }
}
