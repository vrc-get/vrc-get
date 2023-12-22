use indexmap::IndexMap;
use std::path::Path;
use either::{Either, for_both};
use url::Url;

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub(crate) enum PreDefinedRepoSource {
    Official,
    Curated,
}

pub(crate) static DEFINED_REPO_SOURCES: &[PreDefinedRepoSource] = &[
    PreDefinedRepoSource::Official,
    PreDefinedRepoSource::Curated,
];

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

impl <L: RepoSource, R: RepoSource> RepoSource for Either<L, R> {
    fn cache_path(&self) -> &Path {
        for_both!(self, src => src.cache_path())
    }

    fn headers(&self) -> &IndexMap<String, String> {
        for_both!(self, src => src.headers())
    }

    fn url(&self) -> Option<&Url> {
        for_both!(self, src => src.url())
    }
}
