use crate::version::{UnityVersion, Version, VersionRange};
use crate::{unity_compatible, PackageInfo};

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
    },
    Ranges {
        project_unity: Option<UnityVersion>,
        ranges: &'a [&'a VersionRange],
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

    pub fn range_for(unity_version: Option<UnityVersion>, range: &'a VersionRange) -> Self {
        Self {
            inner: SelectorInner::Range {
                project_unity: unity_version,
                range,
            },
        }
    }

    pub fn ranges_for(unity_version: Option<UnityVersion>, ranges: &'a [&'a VersionRange]) -> Self {
        Self {
            inner: SelectorInner::Ranges {
                project_unity: unity_version,
                ranges,
            },
        }
    }
}

impl<'a> VersionSelector<'a> {
    pub(crate) fn as_specific(&self) -> Option<&Version> {
        match self.inner {
            SelectorInner::Specific(version) => Some(version),
            _ => None,
        }
    }
}

impl<'a> VersionSelector<'a> {
    pub fn satisfies(&self, package: &PackageInfo) -> bool {
        fn unity_and_yank(package: &PackageInfo, project_unity: Option<UnityVersion>) -> bool {
            #[cfg(feature = "experimental-yank")]
            if package.is_yanked() {
                return false;
            }

            if let Some(unity) = project_unity {
                if !unity_compatible(package, unity) {
                    return false;
                }
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
            } => range.matches(package.version()) && unity_and_yank(package, project_unity),
            SelectorInner::Ranges {
                ranges,
                project_unity,
            } => {
                ranges.iter().all(|x| x.matches(package.version()))
                    && unity_and_yank(package, project_unity)
            }
        }
    }
}
