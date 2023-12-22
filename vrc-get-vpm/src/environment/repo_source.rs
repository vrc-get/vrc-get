use crate::UserRepoSetting;
use indexmap::IndexMap;
use lazy_static::lazy_static;
use std::path::{Path, PathBuf};
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

pub(crate) trait RepoSource {
    fn cache_path(&self) -> &Path;
    fn headers(&self) -> &IndexMap<String, String>;
    fn url(&self) -> Option<&Url>;
}

impl RepoSource for RepoSourceImpl {
    fn cache_path(&self) -> &Path {
        match self {
            RepoSourceImpl::PreDefined(_, _, path) => path.as_path(),
            RepoSourceImpl::UserRepo(repo) => repo.local_path.as_path(),
        }
    }

    fn headers(&self) -> &IndexMap<String, String> {
        match self {
            RepoSourceImpl::PreDefined(_, _, _) => {
                lazy_static! {
                    static ref HEADERS: IndexMap<String, String> = IndexMap::new();
                }
                &HEADERS
            }
            RepoSourceImpl::UserRepo(repo) => &repo.headers,
        }
    }

    fn url(&self) -> Option<&Url> {
        match self {
            RepoSourceImpl::PreDefined(_, url, _) => Some(url),
            RepoSourceImpl::UserRepo(repo) => repo.url.as_ref(),
        }
    }
}

#[derive(Clone)]
#[non_exhaustive]
pub(crate) enum RepoSourceImpl {
    PreDefined(PreDefinedRepoSource, Url, PathBuf),
    UserRepo(UserRepoSetting),
}

pub(crate) static DEFINED_REPO_SOURCES: &[PreDefinedRepoSource] = &[
    PreDefinedRepoSource::Official,
    PreDefinedRepoSource::Curated,
];
