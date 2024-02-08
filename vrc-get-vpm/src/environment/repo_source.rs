use indexmap::IndexMap;
use std::path::Path;
use url::Url;

pub(crate) struct RepoSource<'a> {
    cache_path: &'a Path,
    headers: &'a IndexMap<Box<str>, Box<str>>,
    url: Option<&'a Url>,
}

impl<'a> RepoSource<'a> {
    pub fn new(
        cache_path: &'a Path,
        headers: &'a IndexMap<Box<str>, Box<str>>,
        url: Option<&'a Url>,
    ) -> Self {
        Self {
            cache_path,
            headers,
            url,
        }
    }

    pub fn cache_path(&self) -> &Path {
        self.cache_path
    }

    pub fn headers(&self) -> &IndexMap<Box<str>, Box<str>> {
        self.headers
    }

    pub fn url(&self) -> Option<&Url> {
        self.url
    }
}
