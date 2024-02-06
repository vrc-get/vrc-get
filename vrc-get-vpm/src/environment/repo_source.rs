use indexmap::IndexMap;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use url::Url;

pub(crate) struct RepoSource<'a> {
    cache_path: Cow<'a, Path>,
    headers: &'a IndexMap<Box<str>, Box<str>>,
    url: Option<Cow<'a, Url>>,
}

impl<'a> RepoSource<'a> {
    pub fn new(
        cache_path: &'a Path,
        headers: &'a IndexMap<Box<str>, Box<str>>,
        url: Option<&'a Url>,
    ) -> Self {
        Self {
            cache_path: cache_path.into(),
            headers,
            url: url.map(Cow::Borrowed),
        }
    }

    pub fn new_owned(
        cache_path: PathBuf,
        headers: &'a IndexMap<Box<str>, Box<str>>,
        url: Option<Url>,
    ) -> Self {
        Self {
            cache_path: cache_path.into(),
            headers,
            url: url.map(Cow::Owned),
        }
    }

    pub fn cache_path(&self) -> &Path {
        &self.cache_path
    }

    pub fn headers(&self) -> &IndexMap<Box<str>, Box<str>> {
        self.headers
    }

    pub fn url(&self) -> Option<&Url> {
        self.url.as_deref()
    }
}
