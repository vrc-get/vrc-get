pub mod setting {
    use crate::environment::RepoSource;
    use indexmap::IndexMap;
    use serde::{Deserialize, Serialize};
    use std::path::Path;
    use url::Url;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(rename_all = "camelCase")]
    pub struct UserRepoSetting {
        local_path: Box<Path>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<Box<str>>,
        // must be non-relative url.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        url: Option<Url>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub(crate) id: Option<Box<str>>,
        #[serde(default)]
        pub(crate) headers: IndexMap<Box<str>, Box<str>>,
    }

    impl UserRepoSetting {
        pub fn new(
            local_path: Box<Path>,
            name: Option<Box<str>>,
            url: Option<Url>,
            id: Option<Box<str>>,
        ) -> Self {
            Self {
                local_path,
                name,
                id: id.or(url.as_ref().map(Url::to_string).map(Into::into)),
                url,
                headers: IndexMap::new(),
            }
        }

        pub fn local_path(&self) -> &Path {
            &self.local_path
        }

        pub fn name(&self) -> Option<&str> {
            self.name.as_deref()
        }

        pub fn url(&self) -> Option<&Url> {
            self.url.as_ref()
        }

        pub fn id(&self) -> Option<&str> {
            self.id.as_deref()
        }

        pub fn headers(&self) -> &IndexMap<Box<str>, Box<str>> {
            &self.headers
        }

        pub(crate) fn to_source(&self) -> RepoSource<'_> {
            RepoSource::new(&self.local_path, &self.headers, self.url.as_ref())
        }
    }
}
