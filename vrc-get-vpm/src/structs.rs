pub mod setting {
    use crate::environment::RepoSource;
    use crate::utils::json::{JsonError, JsonObject, JsonValue};
    use indexmap::IndexMap;
    use std::path::{Path, PathBuf};
    use url::Url;

    #[derive(Debug, Clone)]
    pub struct UserRepoSetting {
        local_path: Box<Path>,
        name: Option<Box<str>>,
        // must be non-relative url.
        url: Option<Url>,
        pub(crate) id: Option<Box<str>>,
        pub(crate) headers: IndexMap<Box<str>, Box<str>>,
    }

    impl UserRepoSetting {
        pub(crate) fn from_json_value(value: JsonValue) -> Result<Self, JsonError> {
            let mut object = value.into_object()?;
            Ok(Self {
                local_path: PathBuf::from(object.get_req("localPath")?.into_string()?)
                    .into_boxed_path(),
                name: (object.get_opt("name"))
                    .try_map(JsonValue::into_string)?
                    .map(Into::into),
                url: (object.get_opt("url"))
                    .parse_opt(|x| Url::parse(&x).map(Some))?
                    .flatten(),
                id: (object.get_opt("name"))
                    .try_map(JsonValue::into_string)?
                    .map(Into::into),
                headers: (object.get_opt("headers"))
                    .try_map(|value| {
                        (value.into_object())?
                            .into_iter()
                            .map(|(key, value)| {
                                Ok((key.into_boxed_str(), value.into_string()?.into()))
                            })
                            .collect::<Result<IndexMap<_, _>, JsonError>>()
                    })?
                    .unwrap_or(IndexMap::new()),
            })
        }

        pub(crate) fn to_json_value(&self) -> JsonValue {
            let mut object = JsonObject::new();
            object.insert("localPath", &*self.local_path.display().to_string());
            if let Some(name) = &self.name {
                object.insert("name", name);
            }
            if let Some(url) = &self.url {
                object.insert("url", url.as_str());
            }
            if let Some(id) = &self.id {
                object.insert("id", id);
            }
            if !self.headers.is_empty() {
                object.insert(
                    "headers",
                    self.headers
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect::<JsonObject>(),
                );
            }
            object.into()
        }

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
