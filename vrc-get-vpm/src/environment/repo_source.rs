use crate::UserRepoSetting;
use std::path::PathBuf;
use url::Url;

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub(crate) enum PreDefinedRepoSource {
    Official,
    Curated,
}

impl PreDefinedRepoSource {
    pub fn file_name(self) -> &'static str {
        match self {
            PreDefinedRepoSource::Official => "vrc-official.json",
            PreDefinedRepoSource::Curated => "vrc-curated.json",
        }
    }

    pub fn url(self) -> Url {
        match self {
            PreDefinedRepoSource::Official => {
                Url::parse("https://packages.vrchat.com/official?download").unwrap()
            }
            PreDefinedRepoSource::Curated => {
                Url::parse("https://packages.vrchat.com/curated?download").unwrap()
            }
        }
    }
}

#[derive(Clone)]
#[non_exhaustive]
pub(crate) enum RepoSource {
    PreDefined(PreDefinedRepoSource, Url, PathBuf),
    UserRepo(UserRepoSetting),
}

pub(crate) static DEFINED_REPO_SOURCES: &[PreDefinedRepoSource] = &[
    PreDefinedRepoSource::Official,
    PreDefinedRepoSource::Curated,
];
