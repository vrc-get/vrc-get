use either::{for_both, Either};
use enum_map::Enum;
use indexmap::IndexMap;
use lazy_static::lazy_static;
use std::path::{Path, PathBuf};
use url::Url;
use crate::utils::PathBufExt;

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug, Enum)]
pub(crate) enum PreDefinedRepoType {
    Official,
    Curated,
}

impl PreDefinedRepoType {
    pub fn file_name(self) -> &'static str {
        match self {
            PreDefinedRepoType::Official => "vrc-official.json",
            PreDefinedRepoType::Curated => "vrc-curated.json",
        }
    }

    pub fn url(self) -> Url {
        match self {
            PreDefinedRepoType::Official => {
                Url::parse("https://packages.vrchat.com/official?download").unwrap()
            }
            PreDefinedRepoType::Curated => {
                Url::parse("https://packages.vrchat.com/curated?download").unwrap()
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct PredefinedSource {
    pub(crate) url: Url,
    path: PathBuf,
}

impl PredefinedSource {
    pub(crate) fn new(folder: &Path, source: PreDefinedRepoType) -> Self {
        Self {
            url: source.url(),
            path: folder.join("Repos").joined(source.file_name()),
        }
    }
}

impl RepoSource for PredefinedSource {
    fn cache_path(&self) -> &Path {
        self.path.as_path()
    }

    fn headers(&self) -> &IndexMap<String, String> {
        lazy_static! {
            static ref EMPTY_HEADERS: IndexMap<String, String> = IndexMap::new();
        }
        &EMPTY_HEADERS
    }

    fn url(&self) -> Option<&Url> {
        Some(&self.url)
    }
}

pub(crate) trait RepoSource {
    fn cache_path(&self) -> &Path;
    fn headers(&self) -> &IndexMap<String, String>;
    fn url(&self) -> Option<&Url>;
}

impl<T: RepoSource + ?Sized> RepoSource for &T {
    fn cache_path(&self) -> &Path {
        T::cache_path(self)
    }

    fn headers(&self) -> &IndexMap<String, String> {
        T::headers(self)
    }

    fn url(&self) -> Option<&Url> {
        T::url(self)
    }
}

impl<L: RepoSource, R: RepoSource> RepoSource for Either<L, R> {
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
